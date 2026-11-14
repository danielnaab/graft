#!/usr/bin/env bash
# migrate.sh [-C REPO] [--base REF] [--no-check] [--json]: a migration brief for a branch where an
# upgrade is already applied (the bot's graft/upgrade, say): what moved, what upstream says about
# it, what in this repo refers to what moved, and what `nix flake check` says broke. Printed for an
# agent or a person to act on; graft never runs an agent itself.
#   nix run .#graft migrate | claude -p        (or: workmux add x -p "$(nix run .#graft migrate)")
#
# The base is the merge-base of HEAD with REF (default: origin/HEAD, else origin/main,
# origin/master, main, master). Moved inputs come from flake.lock at the base against the working
# tree's; the scions diff is the base against the working tree, uncommitted changes and new files
# included (staged in a copy of the index, so the repo's own index is never touched).
#
# Sections, each also a field of --json: the header (repo, branch, what moved, the rules);
# `upstream` (each moved provider's CHANGELOG.md sections between the two tags, Upgrading
# subsections first, or its commits: fetched with plain git as graft ci pr does, quoted as the
# provider's text); `changes` (the scions diff, renames and deletions first); `references` (lines
# of the repo's own files, outside graft's directories and workflows, that name a renamed or
# deleted scion path); `checks` (each of checks.<system> by name, then `nix flake check`, unless
# --no-check). Run as `nix run .#graft migrate` this is the branch's own graft, from its evaluated
# flake; run from graft's flake (`nix run github:danielnaab/graft/vX.Y.Z -- migrate -C .`) with
# --no-check, nothing of the branch's is evaluated or run: its files are only read.
#
# Exit 0 whenever a brief is printed, failing checks or not: it's a report. Nonzero only when it
# can't make one (no flake.lock, no git, no base).
set -euo pipefail

here=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
# shellcheck source-path=SCRIPTDIR source=common.sh
. "$here/common.sh"

repo="" base_ref="" check=1 json=0
while [ $# -gt 0 ]; do
  case $1 in
    -C) [ $# -ge 2 ] || die "-C needs a directory"; repo=$2; shift 2 ;;
    --base) [ $# -ge 2 ] || die "--base needs a ref"; base_ref=$2; shift 2 ;;
    --base=*) base_ref=${1#--base=}; shift ;;
    --no-check) check=0; shift ;;
    --json) json=1; shift ;;
    -h|--help) sed -n '2,6p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'; exit 0 ;;
    *) die "usage: graft migrate [-C REPO] [--base REF] [--no-check] [--json]" ;;
  esac
done
locate "$repo"
top=$TOP
git rev-parse -q --verify 'HEAD^{commit}' >/dev/null || die "no commits here yet"

# the base: the merge-base with the default branch, or with --base
if [ -z "$base_ref" ]; then
  base_ref=$(git symbolic-ref -q --short refs/remotes/origin/HEAD 2>/dev/null) || base_ref=""
  if [ -z "$base_ref" ]; then
    for r in origin/main origin/master main master; do
      if git rev-parse -q --verify "$r^{commit}" >/dev/null; then base_ref=$r; break; fi
    done
  fi
  [ -n "$base_ref" ] || die "can't tell the default branch (no origin/HEAD, main or master); pass --base REF"
  git rev-parse -q --verify "$base_ref^{commit}" >/dev/null 2>&1 || die "the default branch $base_ref isn't a commit here (fetch it, or pass --base REF)"
fi
git rev-parse -q --verify --end-of-options "$base_ref^{commit}" >/dev/null 2>&1 || die "--base $base_ref: not a commit"
base=$(git merge-base --end-of-options "$base_ref" HEAD) || die "$base_ref and HEAD have no common ancestor"

tmp=$GRAFT_TMP/migrate
mkdir -p "$tmp"
empty_lock='{"nodes":{"root":{"inputs":{}}}}'
git show "$base:$FLAKE_LOCK" > "$tmp/base.lock" 2>/dev/null || echo "$empty_lock" > "$tmp/base.lock"
jq -e . "$tmp/base.lock" >/dev/null 2>&1 || echo "$empty_lock" > "$tmp/base.lock"
if [ -f "$FLAKE_LOCK" ]; then
  jq -e . "$FLAKE_LOCK" >/dev/null 2>&1 || die "$FLAKE_LOCK isn't JSON"
  cp "$FLAKE_LOCK" "$tmp/head.lock"
else
  echo "$empty_lock" > "$tmp/head.lock"
fi

# graft's directory and workflows here
dirs=() pathspec=()
if [ -n "$SCIONS" ]; then dirs=("$SCIONS"); pathspec=(":(literal)$SCIONS"); fi
workflows=()
for f in .forgejo/workflows/*; do
  if [ -f "$f" ] && [ ! -L "$f" ] && grep -q "^$WORKFLOW_MARKER" "$f"; then workflows+=("$f"); fi
done

# a pin as text: its version tag; another ref (a branch, under --override-input say) with its
# commit; else its commit
label() {
  local ref rev
  ref=$(lock_ref "$1"); rev=$(jq -r '(.locked.rev // "-")[0:12]' <<<"$1")
  if is_semver "$ref"; then echo "$ref"; elif [ -n "$ref" ]; then echo "$ref at $rev"; else echo "$rev"; fi
}

# moved lock inputs: graft's with what upstream says, any other just named
: > "$tmp/moved"; : > "$tmp/other"
while IFS= read -r n; do
  o=$(lock_node "$tmp/base.lock" "$n"); nw=$(lock_node "$tmp/head.lock" "$n")
  ov=$(jq -r '.locked.rev // .locked.narHash // empty' <<<"${o:-null}")
  nv=$(jq -r '.locked.rev // .locked.narHash // empty' <<<"${nw:-null}")
  [ "$ov" != "$nv" ] || continue
  if [ "$n" != graft ]; then
    jq -n --arg name "$n" --arg from "$([ -n "$o" ] && label "$o")" --arg to "$([ -n "$nw" ] && label "$nw")" \
      '{name: $name, kind: "other", status: "moved", from: $from, to: $to}' >> "$tmp/other"
  elif [ -z "$o" ] || [ -z "$nw" ]; then
    jq -n --arg s "$([ -z "$o" ] && echo added || echo removed)" --arg from "$([ -n "$o" ] && label "$o")" \
      --arg to "$([ -n "$nw" ] && label "$nw")" '{name: "graft", kind: "graft", status: $s, from: $from, to: $to}' >> "$tmp/moved"
  else
    echo "graft: reading what changed upstream in graft" >&2
    j=$(upstream_input "$tmp/base.lock" "$tmp/head.lock" graft)
    [ -n "$j" ] || j=$(jq -n --arg from "$(label "$o")" --arg to "$(label "$nw")" \
      '{name: "graft", from: $from, to: $to, status: "not-commit"}')
    jq -c '. + {kind: "graft"}' <<<"$j" >> "$tmp/moved"
  fi
done < <(jq -r -s '[.[].nodes.root.inputs | to_entries[] | select(.value|type=="string") | .key] | unique[]' "$tmp/base.lock" "$tmp/head.lock")

# moved scions: each gitlink at the base against the scion now (its checkout, else the index), with
# what upstream says and what changed in its tree
: > "$tmp/changes"; : > "$tmp/gone"
if [ -n "$SCIONS" ]; then
  git show "$base:.gitmodules" > "$tmp/base.gitmodules" 2>/dev/null || : > "$tmp/base.gitmodules"
  declare -A surl=()
  while IFS=$'\t' read -r name path url; do [ -z "$name" ] || surl[$name]=$url; done < <(scions_registered "$tmp/base.gitmodules")
  while IFS=$'\t' read -r name path url; do [ -z "$name" ] || surl[$name]=$url; done < <(scions_registered .gitmodules)
  for name in $(printf '%s\n' "${!surl[@]}" | LC_ALL=C sort); do
    path=$SCIONS/$name url=${surl[$name]}
    ov=$(gitlink "$path" "$base")
    nv=$(gitlink "$path")
    if [ -n "$nv" ] && [ -e "$path/.git" ]; then nv=$(git -C "$path" rev-parse -q --verify HEAD 2>/dev/null) || nv=$(gitlink "$path"); fi
    [ "$ov" != "$nv" ] || continue
    ot=""; nt=""
    [ -z "$ov" ] || ot=$(version_at "$url" "$ov" 2>/dev/null) || ot=""
    [ -z "$nv" ] || nt=$(version_at "$url" "$nv" 2>/dev/null) || nt=""
    if [ -z "$ov" ] || [ -z "$nv" ]; then
      jq -n -c --arg name "$name" --arg s "$([ -z "$ov" ] && echo added || echo removed)" \
        --arg from "${ot:-${ov:0:12}}" --arg to "${nt:-${nv:0:12}}" \
        '{name: $name, kind: "scion", status: $s, from: $from, to: $to}' >> "$tmp/moved"
      continue
    fi
    echo "graft: reading what changed upstream in $name" >&2
    j=$(upstream_range "$name" "$(scion_node "$url" "$ot")" "$ov" "$nv" "$ot" "$nt")
    jq -c '. + {kind: "scion"}' <<<"$j" >> "$tmp/moved"
    # the tree's changes: in the scion's own clone when it has both commits, else a bare fetch
    r=$path
    if ! { [ -e "$path/.git" ] && git -C "$path" cat-file -e "$ov^{commit}" 2>/dev/null && git -C "$path" cat-file -e "$nv^{commit}" 2>/dev/null; }; then
      r=$tmp/repo-$name
      git init -q --bare "$r"
      git -C "$r" -c http.extraHeader= fetch -q --no-tags -- "$url" "$ov" "$nv" 2>/dev/null \
        || git -C "$r" -c http.extraHeader= fetch -q -- "$url" 'refs/tags/*:refs/tags/*' 2>/dev/null || true
      if ! git -C "$r" cat-file -e "$ov^{commit}" 2>/dev/null || ! git -C "$r" cat-file -e "$nv^{commit}" 2>/dev/null; then
        echo "graft: can't fetch $name's commits from $url: its changes are left out" >&2
        continue
      fi
    fi
    declare -A plus=() minus=()
    while IFS= read -r -d '' rec; do
      IFS=$'\t' read -r a m q <<<"$rec"
      if [ -z "$q" ]; then IFS= read -r -d '' _ && IFS= read -r -d '' q; fi
      plus[$q]=$a minus[$q]=$m
    done < <(git -C "$r" diff -z --numstat -M "$ov" "$nv")
    while IFS= read -r -d '' st; do
      from=""
      IFS= read -r -d '' q
      if [[ $st == [RC]* ]]; then from=$q; IFS= read -r -d '' q; fi
      jq -n -c --arg s "${st:0:1}" --arg p "$path/$q" --arg f "${from:+$path/$from}" --arg a "${plus[$q]:--}" --arg m "${minus[$q]:--}" \
        '{status: $s, path: $p, from: (if $f == "" then null else $f end),
          added: ($a | tonumber? // null), deleted: ($m | tonumber? // null)}' >> "$tmp/changes"
      case ${st:0:1} in
        R) printf '%s\t%s\t%s\t%s\t%s\0' "$path/$from" renamed "$path/$q" "$r" "$nv" >> "$tmp/gone" ;;
        D) printf '%s\t%s\t\t%s\t%s\0' "$path/$q" deleted "$r" "$nv" >> "$tmp/gone" ;;
      esac
    done < <(git -C "$r" diff -z --name-status -M "$ov" "$nv")
    unset plus minus
  done
fi

# references: lines of the repo's own files naming a renamed or deleted scion path (or a directory
# of one that's gone), the way a file outside the scions directory has to: through its last segment
# ("scions/name/a.md", "../scions/name/a.md", "/scions/name/a.md"), spaces raw or as %20
: > "$tmp/refs"
refs_error=""
declare -A target=()
if [ -s "$tmp/gone" ]; then
  declare -A seen=()
  add_key() {  # add_key KEY JSON
    target[$1]=$2
    if [[ $1 == *" "* ]]; then target[${1// /%20}]=$2; fi
  }
  d=$SCIONS
  while IFS=$'\t' read -r -d '' p how to r nv; do
    [[ $p == "$d"/* ]] || continue
    add_key "${d##*/}/${p#"$d"/}" "$(jq -n -c --arg t "$p" --arg h "$how" --arg to "$to" \
      '{target: $t, status: $h, to: (if $to == "" then null else $to end)}')"
    sp=${p#"$d"/}; sp=${sp#*/}   # the path inside the scion
    a=${sp%/*}
    while [ "$a" != "$sp" ] && [ -n "$a" ] && [ -z "${seen[$p:$a]:-}" ]; do
      seen[$p:$a]=1
      if ! git -C "$r" cat-file -e "$nv:$a" 2>/dev/null; then
        full=${p%"$sp"}$a
        add_key "${d##*/}/${full#"$d"/}" "$(jq -n -c --arg t "$full/" '{target: $t, status: "gone", to: null}')"
      fi
      sp=$a; a=${a%/*}
    done
  done < "$tmp/gone"
  if [ ${#target[@]} -gt 0 ]; then
    # PCREs of up to 200 keys each (one of thousands is too big to compile): the keys longest first
    # (alternation takes the first that matches), each ASCII punctuation character escaped, bounded
    # so `a.md` doesn't match `a.mdx` or `a.md.bak`
    printf '%s\n' "${!target[@]}" | awk '{ print length($0) "\t" $0 }' | sort -rn | cut -f2- \
      | LC_ALL=C sed 's/[!-/:-@[-`{-~]/\\&/g' > "$tmp/keys"
    split -l 200 "$tmp/keys" "$tmp/keys."
    excl=(":(exclude,literal)$FLAKE_LOCK")
    for d in "${dirs[@]}"; do excl+=(":(exclude,literal)$d"); done
    for f in "${workflows[@]}"; do excl+=(":(exclude,literal)$f"); done
    : > "$tmp/hits"
    for k in "$tmp"/keys.*; do
      printf '(?<![A-Za-z0-9._-])(?:%s)(?![A-Za-z0-9_-])(?!\\.[A-Za-z0-9_-])\n' "$(paste -sd '|' "$k")" > "$tmp/pattern"
      rc=0
      git grep -z -n -o -I -P --untracked -f "$tmp/pattern" -- . "${excl[@]}" >> "$tmp/hits" 2>"$tmp/grep.err" || rc=$?
      if [ "$rc" -gt 1 ]; then
        refs_error=$(head -c 200 "$tmp/grep.err" | head -1)
        echo "graft: couldn't search the repo for references: $refs_error" >&2
        break
      fi
    done
    if [ -z "$refs_error" ]; then
      while IFS= read -r -d '' f && IFS= read -r -d '' ln && IFS= read -r m; do
        [ -n "${target[$m]:-}" ] || continue
        jq -c --arg f "$f" --arg l "$ln" '{file: $f, line: ($l | tonumber)} + .' <<<"${target[$m]}"
      done < "$tmp/hits" | jq -s -c 'unique_by([.file, .line, .target]) | sort_by(.file, .line) | .[]' > "$tmp/refs"
    fi
  fi
fi

nothing=0
if [ ! -s "$tmp/moved" ] && [ ! -s "$tmp/changes" ]; then nothing=1; fi

# checks: unless --no-check (and there's something to check)
echo null > "$tmp/checks"
if [ "$check" = 1 ] && [ "$nothing" = 0 ]; then
  echo "graft: running the repo's checks (their logs follow; the brief goes to stdout)" >&2
  if full_check "$tmp/report" >&2; then
    echo '{"ran": true, "passed": true, "failed": [], "log": ""}' > "$tmp/checks"
  else
    head -1 "$tmp/report" | sed 's/^nix flake check failed: //' > "$tmp/failed"
    tail -n +3 "$tmp/report" > "$tmp/log"
    jq -n --rawfile f "$tmp/failed" --rawfile log "$tmp/log" \
      '{ran: true, passed: false, failed: ($f | split(" ") | map(select(length > 0) | rtrimstr("\n"))), log: $log}' > "$tmp/checks"
  fi
elif [ "$check" = 0 ]; then
  echo '{"ran": false, "passed": null, "failed": [], "log": ""}' > "$tmp/checks"
fi

# the repo's name: its origin's, else its main checkout's folder (not a worktree's)
name=$(git remote get-url origin 2>/dev/null || true)
name=${name%/}; name=${name%.git}; name=${name##*/}; name=${name##*:}
if [ -z "$name" ]; then
  common=$(git rev-parse --path-format=absolute --git-common-dir 2>/dev/null) || common=""
  # REPO/.git, or a bare repo's REPO.git (a worktree of a bare clone)
  if [ "${common##*/}" = .git ]; then name=$(basename "$(dirname "$common")")
  elif [ -n "$common" ]; then name=${common##*/}; name=${name%.git}
  else name=$(basename "$top"); fi
fi
branch=$(git branch --show-current)
applied=committed
if [ -n "$(git --no-optional-locks status --porcelain --untracked-files=no -- "$FLAKE_NIX" "$FLAKE_LOCK" "${pathspec[@]}")" ]; then applied=uncommitted; fi
# whether flake.nix moved too (a ref graft upgrade changed): the brief names it with the lock. Only
# a changed line with an input's url counts (graft upgrade writes the tag there), so an edit of the
# repo's own outputs doesn't; one to a url line by hand does, and is a pin too.
nix_moved=false
if grep -qE '^[-+][^-+].*url' <<<"$(git diff "$base" -- "$FLAKE_NIX" 2>/dev/null)"; then nix_moved=true; fi

jq -n --arg repo "$name" --arg branch "$branch" --arg head "$(git rev-parse --short HEAD)" \
  --arg base_ref "$base_ref" --arg base "$base" --arg applied "$applied" --arg refs_error "$refs_error" \
  --arg flake "$FLAKE" --argjson nix_moved "$nix_moved" \
  --argjson nothing "$([ "$nothing" = 1 ] && echo true || echo false)" \
  --args '{repo: $repo, branch: (if $branch == "" then null else $branch end), head: $head,
     flake: (if $flake == "." then null else $flake end), flake_nix_moved: $nix_moved,
     base: {ref: $base_ref, commit: $base}, applied: $applied, nothing_moved: $nothing,
     references_error: (if $refs_error == "" then null else $refs_error end),
     scion_dirs: $ARGS.positional}' "${dirs[@]}" > "$tmp/top"
jq -n --args '$ARGS.positional' "${workflows[@]}" > "$tmp/workflows"
jq -s --slurpfile top "$tmp/top" --slurpfile wf "$tmp/workflows" --slurpfile checks "$tmp/checks" \
  --slurpfile other "$tmp/other" --slurpfile changes "$tmp/changes" --slurpfile refs "$tmp/refs" \
  '$top[0] + {graft_workflows: $wf[0], moved: ., also_moved: $other, changes: $changes,
             references: $refs, checks: $checks[0]}' "$tmp/moved" > "$tmp/brief.json"

if [ "$json" = 1 ]; then jq . "$tmp/brief.json"; else jq -r -L "$here" -f "$here/migrate.jq" "$tmp/brief.json"; fi
