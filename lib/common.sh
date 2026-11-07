#!/usr/bin/env bash
# common.sh: functions tools.sh, ci.sh and migrate.sh share. Sourced, never run; defines only.
#
#   scion_dirs, scions_in   which directories are graft's, and which scions each holds
#   locate, flake_update    the repo's top and where its graft flake sits (mkFlake's flakeDir)
#   lock_node, is_semver    a flake.lock's root inputs, read as data
#   full_check              the repo's whole `nix flake check`, each check by name
#   upstream_input          what changed upstream for one moved input between two locks
#   upstream_range          the same between two commits or tags (CHANGELOG sections or commits),
#                           fetched with plain git and returned as JSON
#   changelog_between, cap  the text helpers under them

MARKERS=("<!-- managed by graft: https://github.com/danielnaab/graft -->" "Managed by [graft]")
nix_() { nix --extra-experimental-features 'nix-command flakes' "$@"; }
# git whose output parses the same whatever the user's config: no colour, no grep column
git() { command git -c color.ui=never -c grep.column=false "$@"; }
# graft's lib/, for jq's -L: upstream.jq's helpers (clean: upstream text as plain lines)
GRAFT_LIB=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)

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

# The flake directory the scions READMEs record (`<!-- graft flake: DIR -->`, assemble.sh's second
# line), or nothing when none does (the flake is at the top). Read from the repo, so checked: a
# plain relative path, the same in every README.
recorded_flake_dir() {
  local sd d fds
  sd=$(scion_dirs) || exit 1
  fds=$(while IFS= read -r d; do
          [ -z "$d" ] || sed -n 's/^<!-- graft flake: \(.*\) -->$/\1/p' "$d/README.md"
        done <<<"$sd" | sort -u)
  [ -n "$fds" ] || return 0
  [ "$(wc -l <<<"$fds")" -eq 1 ] || die "the scions READMEs name different flake directories: $(paste -sd ' ' <<<"$fds")"
  [[ $fds =~ ^[A-Za-z0-9._-]+(/[A-Za-z0-9._-]+)*$ && ! $fds =~ (^|/)\.\.?(/|$) ]] \
    || die "the scions README names a flake directory graft won't use: $fds"
  echo "$fds"
}

# locate [REPO]: cd to the repo's top and set where its graft flake is. With REPO (-C) that's the
# top, and the flake is where the scions README records it, else at the top; run from graft's own
# flake without -C, the same for the current directory. Run from the repo's own flake, the flake
# says (GRAFT_FLAKE_DIR): at the top, the current directory is the top as it always was; in a
# subdirectory, the top is the git checkout's. Sets:
#   FLAKE       the flake's directory under the top: `.`, or `nix/workspace` say
#   FLAKE_REF   the reference that evaluates it: `.`, or `path:/abs/top/nix/workspace`, which copies
#               only that directory to the store (a git+file flake at the top would copy the repo).
#               Absolute, since Nix won't write the lock of a relative `path:` flake, and
#               percent-encoded, so a space in the path parses.
#   FLAKE_NIX FLAKE_LOCK   its flake.nix and flake.lock, relative to the top
#   GRAFT_RUN   how a person runs its app from the top: `nix run .#graft`, `nix run path:DIR#graft`
# shellcheck disable=SC2034 # set for the scripts that source this
locate() {
  local fd="" top
  if [ -n "${1:-}" ]; then
    cd "$1" || die "can't cd to $1"
  elif [ -n "${GRAFT_FLAKE_DIR:-}" ] && [ "$GRAFT_FLAKE_DIR" != . ]; then
    fd=$GRAFT_FLAKE_DIR
    top=$(git rev-parse --show-toplevel 2>/dev/null) || die "run inside the repo's git checkout (graft's flake is its $fd/)"
    cd "$top" || exit 1
  elif [ "${GRAFT_FLAKE_DIR:-}" = . ]; then
    fd=.
  fi
  [ -n "$fd" ] || fd=$(recorded_flake_dir) || exit 1
  FLAKE=${fd:-.}
  if [ "$FLAKE" = . ]; then
    FLAKE_REF=. FLAKE_NIX=flake.nix FLAKE_LOCK=flake.lock GRAFT_RUN="nix run .#graft"
  else
    FLAKE_REF="path:$(jq -rn --arg p "$(pwd -P)/$FLAKE" '$p | split("/") | map(@uri) | join("/")')" FLAKE_NIX=$FLAKE/flake.nix FLAKE_LOCK=$FLAKE/flake.lock
    GRAFT_RUN="nix run path:$FLAKE#graft"
    [ -f "$FLAKE_NIX" ] || die "no $FLAKE_NIX in $(pwd)"
  fi
}

# flake_update NAME...: re-lock inputs of the flake locate found, as a path: flake when it's in a
# subdirectory (needs Nix 2.19 or later for --flake), so the repo's top is never copied.
flake_update() {
  if [ "$FLAKE" = . ]; then nix_ flake update "$@"; else nix_ flake update --flake "$FLAKE_REF" "$@"; fi
}

# One root input's node in the flake.lock at $1, as JSON, or nothing.
lock_node() { jq -c --arg n "$2" '.nodes.root.inputs[$n] as $k | if ($k|type)=="string" then .nodes[$k] else empty end' "$1"; }

is_semver() { [[ $1 =~ ^v?[0-9]+\.[0-9]+\.[0-9]+$ ]]; }

# full_check REPORT: the repo's whole check on the working tree (of the flake locate found): each of
# checks.<system> built on its own (so a failure has a name), then `nix flake check` for the rest
# (evaluation, other systems' outputs). A flake in a subdirectory has no checks.<system>.graft, so
# `graft check` runs first, as `graft`. Logs go to stdout as they run. Fails if anything did, after
# writing to REPORT one line "nix flake check failed: NAME..." and the last lines of each failing log.
full_check() {
  local report=$1 system names n log failed=() tails=""
  system=$(nix_ eval --impure --raw --expr builtins.currentSystem)
  log=$(mktemp)
  if [ "$FLAKE" != . ]; then
    echo "graft: graft check"
    if ! nix_ run "$FLAKE_REF#graft" check 2>&1 | tee "$log"; then
      failed+=("graft"); tails+="graft:"$'\n'"$(tail -n 30 "$log")"$'\n\n'
      echo "::error::check graft failed on the upgraded tree"
    fi
  fi
  if names=$(nix_ eval --json "$FLAKE_REF#checks.$system" --apply builtins.attrNames 2>"$log" | jq -r '.[]'); then
    while IFS= read -r n; do
      [ -n "$n" ] || continue
      echo "graft: checks.$system.$n"
      [[ $n =~ ^[A-Za-z0-9_][A-Za-z0-9_\'-]*$ ]] || { echo "graft: (an odd name: left to nix flake check)"; continue; }
      if ! nix_ build --no-link -L "$FLAKE_REF#checks.$system.$n" 2>&1 | tee "$log"; then
        failed+=("$n"); tails+="$n:"$'\n'"$(tail -n 30 "$log")"$'\n\n'
        echo "::error::check $n failed on the upgraded tree"
      fi
    done <<<"$names"
  else
    cat "$log" >&2
    failed+=("checks"); tails+="checks (evaluating checks.$system):"$'\n'"$(tail -n 30 "$log")"$'\n\n'
  fi
  if ! nix_ flake check -L "$FLAKE_REF" 2>&1 | tee "$log"; then
    if [ ${#failed[@]} -eq 0 ]; then
      failed+=("flake-check"); tails+="nix flake check:"$'\n'"$(tail -n 30 "$log")"$'\n\n'
    fi
    echo "::error::nix flake check failed on the upgraded tree"
  fi
  rm -f "$log"
  [ ${#failed[@]} -gt 0 ] || return 0
  { echo "nix flake check failed: ${failed[*]}"; echo; printf '%s' "${tails:0:12000}"; } > "$report"
  return 1
}

# upstream_input BASE_LOCK HEAD_LOCK NAME: what changed upstream for root input NAME between the two
# flake.locks, as upstream_range's JSON, or nothing when its locked commit is the same in both (or
# it's missing from either). The upstream is the one BASE_LOCK names (the default branch's:
# reviewed); an input HEAD_LOCK points somewhere else is reported (status elsewhere), not fetched.
# HEAD_LOCK's revs and refs are untrusted: upstream_range checks them before git sees them.
upstream_input() {
  local base=$1 head=$2 name=$3 o nw ov nv status=""
  o=$(lock_node "$base" "$name"); nw=$(lock_node "$head" "$name")
  if [ -z "$o" ] || [ -z "$nw" ]; then return 0; fi
  ov=$(jq -r '.locked.rev // empty' <<<"$o"); nv=$(jq -r '.locked.rev // empty' <<<"$nw")
  [ "$ov" != "$nv" ] || return 0
  if [ "$(jq -n --argjson a "$o" --argjson b "$nw" '($a.original|del(.ref,.rev)) == ($b.original|del(.ref,.rev))')" != true ]; then
    status=elsewhere
  fi
  upstream_range "$name" "$o" "$ov" "$nv" "$(jq -r '.original.ref // empty' <<<"$o")" \
    "$(jq -r '.original.ref // empty' <<<"$nw")" "$status"
}

# upstream_range NAME NODE FROM_REV TO_REV FROM_REF TO_REF [STATUS]: what changed upstream for an
# input (NODE, its flake.lock node, says where it comes from) between two commits, as one JSON
# object. TO_REV may be empty when TO_REF is a version tag: the tag is fetched and its commit used
# (graft changelog, which has no lock for the target). A rev must be a full commit hash and a ref a
# version tag before git sees it; a ref that isn't one is dropped. STATUS, if given, is reported
# with nothing fetched. The provider is fetched with `git fetch` into a bare repo (no checkout: no
# hooks, no filters run), without http.extraHeader. Fetched text is data: it reaches the JSON
# through files and jq, never a command line or eval. Fields:
#   name from to from_rev to_rev  from/to: the tags, or the first 12 of the commits
#   status    fetched | elsewhere (the caller's) | not-commit | not-git | unfetchable
#   url compare                   where it was fetched from; a compare link (GitHub only), or null
#   changelog upgrading rest      CHANGELOG.md's sections for from < version <= to (headings two
#                                 levels down), all of them, only their Upgrading subsections, and
#                                 all but those; "" when there are none
#   commits   up to 51 lines of `git log --oneline` for the range, when there's no changelog text
#   in_history                    whether from_rev was found upstream
upstream_range() {
  local name=$1 o=$2 ov=$3 nv=$4 ot=$5 nt=$6 status=${7:-} url cmp tmp hist=false
  is_semver "$ot" || ot=""; is_semver "$nt" || nt=""
  url=$(jq -r 'if .original.type=="github" then "https://github.com/\(.original.owner)/\(.original.repo).git"
               elif .original.type=="git" then .original.url else empty end' <<<"$o")
  tmp=$(mktemp -d) || return 1
  : > "$tmp/changelog"; : > "$tmp/upgrading"; : > "$tmp/rest"; : > "$tmp/commits"
  if [ -n "$status" ]; then
    :
  elif ! [[ $ov =~ ^[0-9a-f]{40}$ ]] || ! [[ $nv =~ ^[0-9a-f]{40}$ || ( -z $nv && -n $nt ) ]]; then
    status=not-commit
  elif [ -z "$url" ]; then
    status=not-git
  else
    status=fetched
    git init -q --bare "$tmp/git" || { rm -rf "$tmp"; return 1; }
    # by commit (GitHub and Forgejo serve any reachable one), else by tag; either way the commit
    # checked against the lock. With no commit, the tag's.
    if [ -n "$nv" ]; then
      { git -C "$tmp/git" -c http.extraHeader= fetch -q --no-tags -- "$url" "$nv" \
          || { [ -n "$nt" ] && git -C "$tmp/git" -c http.extraHeader= fetch -q --no-tags -- "$url" "refs/tags/$nt"; }; } 2>/dev/null || true
    elif git -C "$tmp/git" -c http.extraHeader= fetch -q --no-tags -- "$url" "refs/tags/$nt" 2>/dev/null; then
      nv=$(git -C "$tmp/git" rev-parse -q --verify 'FETCH_HEAD^{commit}' 2>/dev/null) || nv=""
    fi
    if [ -z "$nv" ] || ! git -C "$tmp/git" cat-file -e "$nv^{commit}" 2>/dev/null; then
      status=unfetchable
    else
      git -C "$tmp/git" cat-file -e "$ov^{commit}" 2>/dev/null \
        || git -C "$tmp/git" -c http.extraHeader= fetch -q --no-tags -- "$url" "$ov" 2>/dev/null || true
      if git -C "$tmp/git" cat-file -e "$ov^{commit}" 2>/dev/null; then hist=true; fi
      # cut, not dropped, past 1 MB (head closing the pipe kills git show: not a failure)
      { git -C "$tmp/git" show "$nv:CHANGELOG.md" 2>/dev/null || true; } | head -c 1000000 > "$tmp/cl.md"
      if [ -n "$ot" ] && [ -n "$nt" ] && [ -s "$tmp/cl.md" ]; then
        changelog_between "$ot" "$nt" all < "$tmp/cl.md" > "$tmp/changelog" || : > "$tmp/changelog"
        changelog_between "$ot" "$nt" upgrading < "$tmp/cl.md" > "$tmp/upgrading" || : > "$tmp/upgrading"
        changelog_between "$ot" "$nt" rest < "$tmp/cl.md" > "$tmp/rest" || : > "$tmp/rest"
      fi
      if ! grep -q '[^[:space:]]' "$tmp/changelog" && [ "$hist" = true ]; then
        git -C "$tmp/git" log --oneline --no-decorate -n 51 "$ov..$nv" > "$tmp/commits" 2>/dev/null || : > "$tmp/commits"
      fi
    fi
  fi
  # a rev the caller gives is shown only if it is one
  [[ $ov =~ ^[0-9a-f]{40}$ ]] || ov="?"
  [[ $nv =~ ^[0-9a-f]{40}$ ]] || nv="?"
  cmp=$(jq -r --arg a "${ot:-$ov}" --arg b "${nt:-$nv}" \
    'if .original.type=="github" then "https://github.com/\(.original.owner)/\(.original.repo)/compare/\($a)...\($b)" else empty end' <<<"$o")
  jq -n -L "$GRAFT_LIB" --arg name "$name" --arg from "${ot:-${ov:0:12}}" --arg to "${nt:-${nv:0:12}}" \
    --arg from_rev "$ov" --arg to_rev "$nv" --arg status "$status" --arg url "$url" --arg compare "$cmp" \
    --rawfile changelog "$tmp/changelog" --rawfile upgrading "$tmp/upgrading" --rawfile rest "$tmp/rest" \
    --rawfile commits "$tmp/commits" --argjson hist "$hist" \
    'include "upstream"; def trim: sub("^\\s+"; "") | sub("\\s+$"; "");
     {name: $name, from: $from, to: $to, from_rev: $from_rev, to_rev: $to_rev, status: $status,
      url: (if $url == "" then null else $url end), compare: (if $compare == "" then null else $compare end),
      changelog: ($changelog | clean | trim), upgrading: ($upgrading | clean | trim), rest: ($rest | clean | trim),
      commits: ($commits | clean | split("\n") | map(select(length > 0))), in_history: $hist}' \
    || { rm -rf "$tmp"; return 1; }
  rm -rf "$tmp"
}

# changelog_between OLD NEW [all|upgrading|rest]: from a CHANGELOG.md on stdin, the sections headed
# `## vX.Y.Z` or `## X.Y.Z` (or `## [X.Y.Z]`, anything after) for OLD < version <= NEW, headings
# moved two levels down to sit under the caller's own. `upgrading` keeps only each section's
# Upgrading subsections (a `### Upgrading…` heading, to the next `###` or `##`), under their
# version's heading; `rest` everything else.
changelog_between() {
  awk -v old="${1#v}" -v new="${2#v}" -v part="${3:-all}" '
    function cmp(a, b,   x, y, i) {
      split(a, x, "."); split(b, y, ".")
      for (i = 1; i <= 3; i++) { if (x[i] + 0 < y[i] + 0) return -1; if (x[i] + 0 > y[i] + 0) return 1 }
      return 0
    }
    function shift(l,   h, n) {
      if (l !~ /^#+ /) return l
      h = l; sub(/^#+/, "", h); n = length(l) - length(h) + 2; if (n > 6) n = 6
      return substr("######", 1, n) h
    }
    # print a kept line (shifted, unless it is RAW: inside a fence) if it belongs to the part asked
    # for; in `upgrading`, its version heading goes first, once
    function emit(l, raw) {
      if (part == "all" || (part == "upgrading") == upg) {
        if (part == "upgrading" && pending != "") { if (printed) print ""; print pending; print ""; pending = "" }
        print (raw ? l : shift(l)); printed = 1
      }
    }
    { sub(/\r$/, ""); line[NR] = $0 }
    END {
      # Code fences first, over the whole file: up to 3 spaces and 3 or more ` or ~, closed by a
      # bare run of the same character at least as long. An opener never closed is plain text, so
      # one stray fence cannot hide the headings after it.
      open = 0
      for (i = 1; i <= NR; i++) {
        if (!match(line[i], /^ {0,3}(```+|~~~+)/)) continue
        run = substr(line[i], RSTART, RLENGTH); sub(/^ +/, "", run)
        if (!open) { open = i; fence = run }
        else if (substr(run, 1, 1) == substr(fence, 1, 1) && length(run) >= length(fence) \
                 && line[i] ~ /^ {0,3}(`+|~+)[ \t]*$/) { inside[open] = i; open = 0 }
      }
      # then the sections, skipping headings inside a fence
      keep = 0; skip_to = 0; upg = 0; pending = ""; printed = 0
      for (i = 1; i <= NR; i++) {
        l = line[i]
        if (i <= skip_to) { if (keep) emit(l, 1); continue }
        if (i in inside) { skip_to = inside[i]; if (keep) emit(l, 1); continue }
        if (l ~ /^## /) {
          v = ""
          if (match(l, /^## \[?v?[0-9]+\.[0-9]+\.[0-9]+([] \t]|$)/)) {
            v = substr(l, RSTART, RLENGTH); sub(/^## \[?v?/, "", v); sub(/[] \t]$/, "", v)
          }
          keep = (v != "" && cmp(v, old) > 0 && cmp(v, new) <= 0)
          upg = 0
          if (keep && part == "upgrading") { pending = shift(l); continue }
        }
        if (l ~ /^# /) keep = 0
        if (!keep) continue
        if (l ~ /^### /) upg = (tolower(l) ~ /^### +upgrading/)
        emit(l)
      }
    }
  '
}

# At most N characters of stdin, cut at a line, with a note when cut.
cap() {
  awk -v max="$1" '{ if (total + length($0) + 1 > max) { cut = 1; exit } total += length($0) + 1; print }
                   END { if (cut) print "\n(cut at " max " characters; the full text is upstream)" }'
}
