#!/usr/bin/env bash
# tools.sh outdated|upgrade [-C REPO] [--json] [NAME[@TAG]...]
#
#   outdated  for each scion, its pinned tag and the newest tag upstream (git ls-remote).
#   upgrade   move each named scion (default: every outdated one) to TAG (default: the newest),
#             re-lock it, and rebuild its scions directory. Prints nothing to do when nothing is.
#
# Both read the repo as data: graft's README table in each scions directory says which flake
# inputs are scions, and flake.lock says what each is pinned to. Neither evaluates the repo's
# flake outputs, so they can run from graft's own flake against any checkout (a central bot) as
# well as from the repo's own (`nix run .#graft-outdated`, CI). One exception, in upgrade:
# `nix flake update <name>` re-locks the input, which reads the flake's `inputs` but never calls
# `outputs`. The rebuilt directory comes from lib/assemble.sh, the same script mkGraft's check
# builds from, so `nix flake check` passes after an upgrade exactly when the copy is right.
#
# Only tags shaped like semver (v1.2.3 or 1.2.3) count as versions. A scion pinned to a branch or a
# bare commit is reported as `untagged` and never upgraded.
set -euo pipefail

MARKERS=("<!-- managed by graft: https://github.com/danielnaab/graft -->" "Managed by [graft]")
here=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
nix_() { nix --extra-experimental-features 'nix-command flakes' "$@"; }
die() { echo "graft: $*" >&2; exit 1; }

# The scions directories in the repo: those whose README.md carries graft's marker. git grep exits
# 1 for no match; anything else (not a repo, a repo git won't trust) is an error, not "no scions".
scion_dirs() {
  local args=() m out rc=0
  for m in "${MARKERS[@]}"; do args+=(-e "$m"); done
  out=$(git grep -l -F "${args[@]}" -- README.md '*/README.md') || rc=$?
  [ "$rc" -le 1 ] || die "can't search $(pwd) with git (exit $rc; see above)"
  [ -n "$out" ] || return 0
  sed -E 's#/?README\.md$##' <<<"$out" | grep -v '^$' || true
}

# The scions a directory's README table names, one per line.
scions_in() { sed -nE 's/^\| \[([A-Za-z0-9][A-Za-z0-9._-]*)\]\(\1\/\) \|.*/\1/p' "$1/README.md"; }

# flake.lock's node for a root input, as JSON, or nothing.
node() { jq -c --arg n "$1" '.nodes.root.inputs[$n] as $k | if ($k|type)=="string" then .nodes[$k] else empty end' flake.lock; }

remote() {  # the git URL of a locked input's upstream
  jq -r 'if .original.type=="github" then "https://github.com/\(.original.owner)/\(.original.repo).git"
         elif .original.type=="git" then .original.url
         else empty end' <<<"$1"
}

semver_key() { sed -E 's/^v//' <<<"$1"; }
is_semver() { [[ $1 =~ ^v?[0-9]+\.[0-9]+\.[0-9]+$ ]]; }

# The newest semver tag at URL, spelled like REF (with or without the v). Nothing, with git's
# error on stderr, if the remote can't be listed. http.extraHeader is cleared: a header the repo
# configured for its own server (a CI checkout's token) must never be sent to a provider's.
latest_tag() {
  local url=$1 ref=$2 prefix="" tags
  [[ $ref == v* ]] && prefix=v
  tags=$(git -c http.extraHeader= ls-remote --tags --refs "$url" 2>&1) \
    || { echo "graft: can't list tags at $url: $(tail -1 <<<"$tags")" >&2; return 0; }
  sed -E 's#.*refs/tags/##' <<<"$tags" | grep -E "^${prefix}[0-9]+\.[0-9]+\.[0-9]+$" | sort -V | tail -1 || true
}

# Rows: name<TAB>dir<TAB>pinned<TAB>latest<TAB>status (current|outdated|untagged|unknown)
report() {
  local dir name n ref url latest status
  while IFS= read -r dir; do
    while IFS= read -r name; do
      n=$(node "$name")
      [ -n "$n" ] || { printf '%s\t%s\t-\t-\tunknown\n' "$name" "$dir"; continue; }
      ref=$(jq -r '.original.ref // empty' <<<"$n")
      url=$(remote "$n")
      if [ -z "$url" ] || ! is_semver "$ref"; then
        printf '%s\t%s\t%s\t-\tuntagged\n' "$name" "$dir" "${ref:-$(jq -r '.locked.rev[0:12] // "-"' <<<"$n")}"
        continue
      fi
      latest=$(latest_tag "$url" "$ref")
      status=current
      if [ -z "$latest" ]; then status=unknown; latest=-
      elif [ "$(printf '%s\n%s\n' "$(semver_key "$ref")" "$(semver_key "$latest")" | sort -V | tail -1)" != "$(semver_key "$ref")" ]; then
        status=outdated
      fi
      printf '%s\t%s\t%s\t%s\t%s\n' "$name" "$dir" "$ref" "$latest" "$status"
    done < <(scions_in "$dir")
  done < <(scion_dirs)
}

cmd_outdated() {
  local rows
  rows=$(report)
  if [ "$json" = 1 ]; then
    jq -R -s -c 'split("\n") | map(select(length>0) | split("\t") | {name:.[0], dir:.[1], pinned:.[2], latest:.[3], status:.[4]})' <<<"$rows"
  elif [ -z "$rows" ]; then
    echo "graft: no scions here (no README.md carrying graft's marker)"
  else
    { echo -e "SCION\tDIR\tPINNED\tLATEST\tSTATUS"; echo "$rows"; } | column -t -s $'\t'
  fi
}

# Swap NAME's ref in flake.nix from OLD to NEW. The URL's spelling must appear exactly once.
retarget() {
  local name=$1 old=$2 new=$3 n from to text count
  n=$(node "$name")
  case $(jq -r .original.type <<<"$n") in
    github) from="$(jq -r '"github:\(.original.owner)/\(.original.repo)/"' <<<"$n")$old"
            to="$(jq -r '"github:\(.original.owner)/\(.original.repo)/"' <<<"$n")$new" ;;
    git)    from="ref=$old"; to="ref=$new" ;;
    *)      die "$name: can't retarget a $(jq -r .original.type <<<"$n") input" ;;
  esac
  count=$(grep -c -F -- "$from" flake.nix || true)
  [ "$count" = 1 ] || die "$name: expected \"$from\" once in flake.nix, found it $count times; change the ref by hand"
  text=$(<flake.nix)
  printf '%s\n' "${text/"$from"/"$to"}" > flake.nix
}

# The flake reference that fetches NAME's locked export (Nix applies export-ignore to these).
locked_ref() {
  jq -r 'if .locked.type=="github" then "github:\(.locked.owner)/\(.locked.repo)/\(.locked.rev)"
         elif .locked.type=="git" then "git+\(.locked.url)?rev=\(.locked.rev)&exportIgnore=1"
         else empty end' <<<"$(node "$1")"
}

# The assemble.sh of the graft version the repo pins (its `graft` input in flake.lock), so a run
# from outside writes exactly what the repo's own `nix flake check` expects. A repo whose flake
# has no `graft` input (graft's own tests) gets this copy's.
assembler() {
  local n ref path
  n=$(node graft)
  [ -n "$n" ] || { echo "$here/assemble.sh"; return; }
  ref=$(jq -r 'if .locked.type=="github" then "github:\(.locked.owner)/\(.locked.repo)/\(.locked.rev)"
               elif .locked.type=="git" then "git+\(.locked.url)?rev=\(.locked.rev)"
               elif .locked.type=="path" then "path:\(.locked.path)?narHash=\(.locked.narHash)"
               else empty end' <<<"$n")
  [ -n "$ref" ] || die "can't fetch this repo's graft input ($(jq -r .locked.type <<<"$n"))"
  path=$(nix_ flake prefetch --json "$ref" | jq -r .storePath)
  [ -f "$path/lib/assemble.sh" ] || die "this repo pins a graft older than 2.2 (no lib/assemble.sh); upgrade it first: nix flake update graft, after moving its ref to v2.2.0 or later"
  echo "$path/lib/assemble.sh"
}

# Rebuild DIR from flake.lock with the repo's assemble.sh, then swap it in (only over graft's own dir).
regraft() {
  local dir=$1 asm=$2 name ref path specs=() tmp
  while IFS= read -r name; do
    ref=$(locked_ref "$name")
    [ -n "$ref" ] || die "$name: no fetchable lock entry"
    path=$(nix_ flake prefetch --json "$ref" | jq -r .storePath)
    specs+=("$name=$path=$(jq -r .locked.rev <<<"$(node "$name")")")
  done < <(scions_in "$dir")
  tmp=$(mktemp -d)
  bash "$asm" "$tmp/out" "$dir" "${specs[@]}"
  rm -rf "$dir"
  mv "$tmp/out" "$dir"
  rm -rf "$tmp"
}

cmd_upgrade() {
  local rows want=() name dir pinned latest status target dirs=() spec plan=() asm
  rows=$(report)
  for spec in "$@"; do want+=("$spec"); done
  if [ ${#want[@]} -gt 0 ]; then
    for spec in "${want[@]}"; do
      grep -q "^${spec%%@*}"$'\t' <<<"$rows" || die "no scion named ${spec%%@*}"
    done
  fi
  # Decide everything first, and check the repo's graft can rebuild, before changing any file.
  while IFS=$'\t' read -r name dir pinned latest status; do
    [ -n "$name" ] || continue
    target=""
    if [ ${#want[@]} -gt 0 ]; then
      for spec in "${want[@]}"; do
        [ "${spec%%@*}" = "$name" ] || continue
        if [[ $spec == *@* ]]; then target=${spec#*@}; else target=$latest; fi
      done
      [ -n "$target" ] || continue
      [ "$status" != untagged ] || die "$name is pinned to $pinned, not a tag; change it by hand"
    else
      [ "$status" = outdated ] || continue
      target=$latest
    fi
    if [ "$target" = "$pinned" ] || [ "$target" = - ]; then continue; fi
    is_semver "$target" || die "$name: $target is not a version tag"
    plan+=("$name"$'\t'"$dir"$'\t'"$pinned"$'\t'"$target")
  done <<<"$rows"
  # A scion whose upstream couldn't be read might be behind: say so and fail, not "nothing to do".
  while IFS=$'\t' read -r name dir pinned latest status; do
    [ "$status" = unknown ] || continue
    if [ ${#want[@]} -eq 0 ] || printf '%s\n' "${want[@]%%@*}" | grep -qx "$name"; then
      die "$name: can't tell whether $pinned is current (see above); nothing changed"
    fi
  done <<<"$rows"
  [ ${#plan[@]} -gt 0 ] || { echo "graft: nothing to upgrade"; return 0; }
  asm=$(assembler) || exit 1

  for spec in "${plan[@]}"; do
    IFS=$'\t' read -r name dir pinned target <<<"$spec"
    echo "graft: $name $pinned -> $target"
    retarget "$name" "$pinned" "$target"
    nix_ flake update "$name" >/dev/null 2>&1 || nix_ flake update "$name"
    [[ " ${dirs[*]} " == *" $dir "* ]] || dirs+=("$dir")
  done
  for dir in "${dirs[@]}"; do regraft "$dir" "$asm"; echo "graft: rebuilt $dir/"; done
}

sub=${1:-}; shift || true
repo=. json=0 args=()
while [ $# -gt 0 ]; do
  case $1 in
    -C) repo=$2; shift 2 ;;
    --json) json=1; shift ;;
    -h|--help) sed -n '2,8p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'; exit 0 ;;
    *) args+=("$1"); shift ;;
  esac
done
cd "$repo"
[ -f flake.lock ] || die "no flake.lock in $(pwd)"
case $sub in
  outdated) cmd_outdated ;;
  upgrade) cmd_upgrade "${args[@]}" ;;
  *) die "usage: graft-outdated|graft-upgrade [-C REPO] [--json] [NAME[@TAG]...]" ;;
esac
