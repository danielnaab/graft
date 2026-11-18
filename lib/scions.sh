#!/usr/bin/env bash
# scions.sh add|sync|check [ARGS...]: graft's commands on the scions themselves.
#
#   add [-C REPO] [--scions PATH] URL[@TAG] [NAME]
#             graft URL at TAG (default: its newest release) as the submodule SCIONS/NAME (NAME
#             default: URL's last segment, without .git), checked out at the tag's commit; writes
#             the scions README when missing; stages .gitmodules, the gitlink and the README.
#   sync      (a repo's own app only) initialise every scion not yet initialised (never one level
#             further down, never moving one already there), then write graft's README and
#             workflows for this version (refresh.sh). Never moves a gitlink.
#   check [-C REPO] [--committed]
#             every scion registered, initialised, clean, checked out at its gitlink, and the gitlink
#             a release tag upstream; graft's workflows this version's. --committed (graft ci check)
#             checks only what a commit holds: no initialised scion needed.
#   try [-C REPO] [NAME...] | try [-C REPO] --stop
#             make a git flake's builds read the working tree, every scion's included, by marking
#             .graft-try at the top with intent to add (never a scion's index); report what Nix builds
#             of each scion, and for the named scions (default: all) the `git -C … add` that makes
#             their new files visible. --stop removes .graft-try. A path: flake reads its working tree
#             always: try only reports.
set -euo pipefail

here=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
# shellcheck source-path=SCRIPTDIR source=common.sh
. "$here/common.sh"
template=${GRAFT_TEMPLATE:-$here/../template}

short() { echo "${1:0:7}"; }

# the scions README for this flake
write_readme() {
  local want
  want=$(sed "s|@RUN@|$GRAFT_RUN|g" "$here/scions-readme.md")
  if [ ! -f "$SCIONS/README.md" ] || [ "$(cat "$SCIONS/README.md")" != "$want" ]; then
    printf '%s\n' "$want" > "$SCIONS/README.md"
  fi
}

# fill_nested PATH: initialise the scion's own submodules that aren't, recursively, at the commits
# it records. Nix reads them from the working tree whenever a tracked file of the repo is modified,
# and fails on an empty one; git leaves them empty unless asked.
fill_nested() {
  local sub
  [ -f "$1/.gitmodules" ] || return 0
  git config -f "$1/.gitmodules" --get-regexp '^submodule\..*\.path$' 2>/dev/null | while read -r _ sub; do
    [ -e "$1/$sub/.git" ] || git -C "$1" submodule update -q --init --recursive -- "$sub" \
      || echo "graft: can't fill $1/$sub (see above): Nix fails on it while the repo's tree has a modified file" >&2
  done
}

# empty_nested PATH: the scion's own submodules that are empty, space-separated
empty_nested() {
  local sub out=""
  [ -f "$1/.gitmodules" ] || return 0
  while read -r _ sub; do
    [ -e "$1/$sub/.git" ] || out+="$sub "
  done < <(git config -f "$1/.gitmodules" --get-regexp '^submodule\..*\.path$' 2>/dev/null)
  echo "${out% }"
}

# nested NAME PATH: a note when the scion has submodules of its own
nested() {
  local subs
  [ -f "$2/.gitmodules" ] || return 0
  subs=$(git config -f "$2/.gitmodules" --get-regexp '^submodule\..*\.path$' 2>/dev/null | awk '{ print $2 }' | paste -sd ' ' -) || true
  [ -z "$subs" ] || echo "graft: note: $2 has its own submodules ($subs); graft fills them, and Nix fetches them too, so each must be reachable"
}

# A flake at the repo's top must declare inputs.self.submodules, or a git flake sees no scions. A
# flake that can't (graft's own, used as a path: input) says so with `# graft: no self.submodules`.
declares_submodules() {
  grep -qE '(self\.submodules|submodules)[[:space:]]*=[[:space:]]*true' "$FLAKE_NIX" \
    || grep -qF '# graft: no self.submodules' "$FLAKE_NIX"
}

cmd_add() {
  local repo="" scions="" spec="" name="" url tag="" commit list path p
  while [ $# -gt 0 ]; do
    case $1 in
      -C) if [ $# -lt 2 ] || [ -z "$2" ]; then die "-C needs a repository"; fi; repo=$2; shift 2 ;;
      --scions) if [ $# -lt 2 ] || [ -z "$2" ]; then die "--scions needs a path from the repo's top, like nix/agent-tools/.scions"; fi; scions=$2; shift 2 ;;
      --scions=*) scions=${1#--scions=}; shift ;;
      -h|--help) sed -n '4,7p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'; exit 0 ;;
      -*) die "unknown option $1 (usage: graft add [-C REPO] [--scions PATH] URL[@TAG] [NAME])" ;;
      *) if [ -z "$spec" ]; then spec=$1; elif [ -z "$name" ]; then name=$1; else die "usage: graft add [-C REPO] [--scions PATH] URL[@TAG] [NAME]"; fi; shift ;;
    esac
  done
  [ -n "$spec" ] || die "usage: graft add [-C REPO] [--scions PATH] URL[@TAG] [NAME]"
  locate "$repo"
  # URL@TAG only when what follows the last @ is a version (git@host:x.git keeps its @)
  url=$spec
  if [[ $spec == *@* ]] && is_semver "${spec##*@}"; then url=${spec%@*}; tag=${spec##*@}; fi
  ! relative_url "$url" || die "a relative URL ($url) won't resolve for Nix; give an absolute URL"
  if [ -z "$name" ]; then name=${url%/}; name=${name##*/}; name=${name##*:}; name=${name%.git}; fi
  [[ $name =~ ^[A-Za-z0-9][A-Za-z0-9._-]*$ && $name != self ]] \
    || die "$name: a scion's name must be letters, digits, . _ and - (starting with a letter or digit), and not self: give one (graft add URL NAME)"
  if [ -n "$scions" ]; then
    scions=${scions%/}
    [[ $scions =~ ^[A-Za-z0-9._-]+(/[A-Za-z0-9._-]+)*$ && ! $scions =~ (^|/)\.\.?(/|$) && ! $scions =~ ^\.git(/|$) ]] \
      || die "--scions $scions: give a plain path from the repo's top, like nix/agent-tools/.scions"
    [ -z "$SCIONS" ] || [ "$SCIONS" = "$scions" ] || die "this repository's scions are in $SCIONS; graft supports one scions directory per repository"
    p=.; [[ $scions != */* ]] || p=${scions%/*}
    [ "$p" = . ] || [ -f "$p/flake.nix" ] || die "--scions $scions: no flake.nix in $p; the scions directory goes beside the flake that grafts them"
  else
    scions=$SCIONS_NEW
  fi
  SCIONS=$scions
  if [[ $SCIONS == */* ]]; then FLAKE=${SCIONS%/*}; else FLAKE=.; fi
  flake_vars
  path=$SCIONS/$name
  # a copied scion (graft v2.12) still in the index: `git add` would record the clone as files
  [ -z "$(git ls-files -s -- "$path" | awk '$1 != "160000"')" ] || die "$path is still tracked as files (a copied scion?): git rm -r -q --cached $path, delete the folder, then graft add again"
  [ ! -e "$path" ] || die "$path exists"
  list=$(tags_of "$url") || die "can't list $url's tags (see above)"
  [ -n "$list" ] || die "$url has no semver tags"
  if [ -n "$tag" ]; then
    commit=$(awk -v t="$tag" '$2 == t { print $1 }' <<<"$list")
    [ -n "$commit" ] || die "$name: no tag $tag upstream"
  else
    tag=$(awk '{ print $2 }' <<<"$list" | sort -V | tail -1)
    commit=$(awk -v t="$tag" '$2 == t { print $1 }' <<<"$list")
  fi
  mkdir -p "$SCIONS"
  # a scion removed and grafted again: git keeps its clone, and reuses it with --force
  if [ -d "$(git rev-parse --git-path "modules/$path")" ]; then
    git submodule add -q --force -- "$url" "$path" >/dev/null 2>&1 || die "git submodule add $url $path failed"
    git -C "$path" remote set-url origin "$url"
  else
    git submodule add -q -- "$url" "$path" || die "git submodule add $url $path failed (see above)"
  fi
  git -C "$path" cat-file -e "$commit^{commit}" 2>/dev/null \
    || git -C "$path" fetch -q origin "refs/tags/$tag:refs/tags/$tag" 2>/dev/null \
    || git -C "$path" fetch -q origin "$commit" || die "can't fetch $tag ($commit) from $url"
  git -C "$path" -c advice.detachedHead=false checkout -q --detach "$commit"
  fill_nested "$path"
  write_readme
  git add -- .gitmodules "$path" "$SCIONS/README.md"
  [ "$(gitlink "$path")" = "$commit" ] || die "$path was not recorded as a submodule (git ls-files -s $path); remove it (git rm -r -q --cached $path) and try again"
  echo "graft: $path <- $tag ($(short "$commit"))"
  nested "$name" "$path"
  requires_note "$name" "$path"
  if [ "$FLAKE" = . ] && [ -f "$FLAKE_NIX" ] && ! declares_submodules; then
    echo "graft: next: commit; a git flake also needs inputs.self.submodules = true;"
  else
    echo "graft: next: commit"
  fi
}

# requires_note NAME PATH: a note for each sibling the scion's graft.nix requires that this repo
# doesn't graft (read from the text: `requires.X = "URL";` or `requires = { X = "URL"; };`)
requires_note() {
  local f=$2/graft.nix text pairs sib u
  [ -f "$f" ] || return 0
  text=$(tr '\n' ' ' < "$f")
  pairs=$( { grep -oE 'requires\.[A-Za-z0-9._-]+[[:space:]]*=[[:space:]]*"[^"]*"' <<<"$text" | sed -E 's/^requires\.//'
             grep -oE 'requires[[:space:]]*=[[:space:]]*\{[^}]*\}' <<<"$text" | grep -oE '[A-Za-z0-9._-]+[[:space:]]*=[[:space:]]*"[^"]*"'; } || true)
  while IFS= read -r sib; do
    [ -n "$sib" ] || continue
    u=${sib#*\"}; u=${u%\"}; sib=${sib%%[[:space:]=]*}
    if ! scions_registered .gitmodules | cut -f1 | grep -qxF -- "$sib"; then
      echo "graft: note: $1 requires $sib, which this repository does not graft: graft add $u"
    fi
  done <<<"$pairs"
}

# files_msg PATH: a submodule path the index holds as files
files_msg() { echo "$1 is tracked as files, not as a submodule: git rm -r -q --cached $1, then git add $1 if $1/.git exists; otherwise delete the folder and graft add its URL@TAG again"; }

# status_line NAME PATH: where an initialised scion is
status_line() {
  local head link v
  head=$(git -C "$2" rev-parse -q --verify HEAD 2>/dev/null) || head=""
  link=$(gitlink "$2")
  if [ -z "$head" ]; then echo "graft: $2 is empty"; return; fi
  v=$(local_version_at "$2" "$head")
  if [ -n "$v" ]; then v="$v ($(short "$head"))"; else v="$(short "$head") (untagged)"; fi
  if [ -n "$link" ] && [ "$head" != "$link" ]; then
    echo "graft: $2 at $v, not the recorded $(short "$link"): left as it is (git add $2 records it; git submodule update --recursive -- $2 goes back)"
  else
    echo "graft: $2 at $v"
  fi
}

cmd_sync() {
  [ -n "${GRAFT_IN_REPO:-}" ] || die "sync runs from the repo's own flake (it writes that graft version's files): nix run .#graft sync, or nix run 'path:DIR#graft' sync"
  [ $# -eq 0 ] || die "sync takes no arguments"
  locate ""
  local name path url
  if [ -z "$SCIONS" ]; then
    echo "graft: no scions here yet: graft add URL"
  else
    while IFS=$'\t' read -r name path url; do
      [ -n "$name" ] || continue
      [ -n "$(gitlink "$path")" ] || [ -z "$(git ls-files -- "$path")" ] || die "$(files_msg "$path")"
      if [ ! -e "$path/.git" ]; then
        git submodule update -q --init -- "$path" || die "can't initialise $path from $url (see above)"
      fi
      fill_nested "$path"
      status_line "$name" "$path"
    done < <(scions_registered .gitmodules)
  fi
  bash "$here/refresh.sh" "$SCIONS" "$FLAKE" "$GRAFT_RUN" "$template"
  [ -z "$(try_state)" ] || echo "graft: note: graft try is on: graft try --stop turns it off before you commit"
}

cmd_check() {
  local repo="" committed="" rc=0 name path url link head v lt list n=0 d dirt nearest mode registered en err system why
  while [ $# -gt 0 ]; do
    case $1 in
      -C) if [ $# -lt 2 ] || [ -z "$2" ]; then die "-C needs a repository"; fi; repo=$2; shift 2 ;;
      --committed) committed=1; shift ;;
      *) die "usage: graft check [-C REPO] [--committed]" ;;
    esac
  done
  locate "$repo"
  bad() { echo "graft: $*" >&2; rc=1; }
  case $(try_state) in
    on) bad "graft try is on ($TRY): Nix builds read the working tree, scions included: graft try --stop turns it off" ;;
    committed) bad "$TRY is committed (graft try's marker): graft try --stop, then commit its removal" ;;
    left) bad "$TRY is left from graft try but untracked, so Nix builds read the recorded scions: graft try --stop removes it, or graft try turns try on again" ;;
  esac
  if [ -z "$SCIONS" ]; then
    echo "graft: no scions here (no README.md carrying graft's marker); graft add URL grafts one" >&2
  else
    registered=$(scions_registered .gitmodules)
    # the repo's own app knows mkFlake's dir: graft's README must be in it
    if [ -n "${GRAFT_IN_REPO:-}" ] && [ -n "${GRAFT_DIR:-}" ] && [ "${SCIONS##*/}" != "$GRAFT_DIR" ]; then
      bad "graft's README is in $SCIONS/ but mkFlake's dir is \"$GRAFT_DIR\": set dir = \"${SCIONS##*/}\"; in $FLAKE_NIX (or move the scions)"
    fi
    # every folder in the scions directory is a submodule
    for d in "$SCIONS"/*/ "$SCIONS"/.[!.]*/; do
      [ -d "$d" ] || continue
      d=${d%/}
      mode=$(git ls-files -s -- "$d" | awk 'NR == 1 { print $1 }')
      if [ "$mode" != 160000 ]; then
        # one .gitmodules registers is reported below
        awk -F'\t' -v p="$d" '$2 == p { f = 1 } END { exit !f }' <<<"$registered" || bad "$d is not a submodule: graft add URL, or remove it"
      elif ! awk -F'\t' -v p="$d" '$2 == p { f = 1 } END { exit !f }' <<<"$registered"; then
        bad "$d is a submodule .gitmodules doesn't register: git rm --cached $d, then graft add its URL"
      fi
    done
    # gitlinks git knows of whose folder is gone (committed mode: a clean checkout has every folder)
    while IFS= read -r d; do
      [ -n "$d" ] && [ ! -d "$d" ] && bad "$d is a submodule with no folder: git submodule update --init --recursive -- $d, or git rm $d"
    done < <(git ls-files -s -- "$SCIONS" | awk '$1 == "160000" { sub(/^[^\t]*\t/, ""); print }')
    while IFS=$'\t' read -r name path url; do
      [ -n "$name" ] || continue
      link=$(gitlink "$path")
      if [ -z "$link" ] && [ -n "$(git ls-files -- "$path")" ]; then
        bad "$(files_msg "$path")"
        continue
      elif [ -z "$link" ]; then
        bad ".gitmodules registers $path, which is not a submodule here: graft add its URL again, or remove its entry (git config -f .gitmodules --remove-section submodule.$path)"
        continue
      fi
      n=$((n + 1))
      if [ -z "$url" ]; then bad "$path has no URL in .gitmodules"; continue; fi
      if relative_url "$url"; then
        bad "$path: .gitmodules gives a relative URL ($url), which Nix can't resolve: give an absolute one (git config -f .gitmodules submodule.$path.url URL)"
      fi
      if [ -z "$committed" ]; then
        if [ ! -e "$path/.git" ]; then
          bad "$path is empty: git submodule update --init --recursive -- $path (or graft sync)"
        else
          dirt=$(git -C "$path" status --porcelain --untracked-files=no 2>/dev/null | head -5 | cut -c4- | paste -sd ' ' -)
          if [ -n "$dirt" ]; then
            if [ "$FLAKE" != . ]; then why="a path: flake's builds use them"
            elif [ "$(try_state)" = on ]; then why="graft try is on: Nix builds use them"
            else why="Nix builds ignore them while the rest of the tree is clean"; fi
            bad "uncommitted edits in $path: $dirt ($why)"
          fi
          head=$(git -C "$path" rev-parse -q --verify HEAD 2>/dev/null) || head=""
          if [ "$head" != "$link" ]; then
            bad "$path is checked out at $(short "$head") but the repository records $(short "$link"): git add $path to record it, or git submodule update --recursive -- $path to go back"
          fi
          nested "$name" "$path"
          en=$(empty_nested "$path")
          [ -z "$en" ] || bad "$path's own submodules are empty ($en): git -C $path submodule update --init --recursive (or graft sync); Nix fails on them while the repo's tree has a modified file"
          # a graft.nix that doesn't parse breaks every output of the flake, graft's app too
          if [ -f "$path/graft.nix" ] && ! err=$(nix_ eval --impure --expr "builtins.isFunction (import $(pwd -P)/$path/graft.nix)" 2>&1); then
            echo "$err" | tail -5 >&2
            bad "$path/graft.nix doesn't evaluate (above): every output of this flake fails until it does, graft's own app too; move the scion back (git submodule update --recursive -- $path), or from outside: nix run github:danielnaab/graft/v2.16.2 -- upgrade -C . $name@TAG"
          fi
        fi
      fi
      # the gitlink is a release: upstream, else (unreadable) the scion's own tags
      if list=$(tags_of "$url" 2>"$GRAFT_TMP/ls.err"); then
        v=$(awk -v c="$link" '$1 == c { print $2 }' <<<"$list" | sort -V | tail -1)
      else
        if [ -n "${POT:-}" ]; then
          # one note for the whole check, not one per scion
          [ -f "$GRAFT_TMP/sandbox-note" ] || { touch "$GRAFT_TMP/sandbox-note"; echo "graft: upstream tags are not reachable from this pot run: checked each scion's local tags" >&2; }
        else
          echo "graft: $path: $(sed 's/^graft: //' "$GRAFT_TMP/ls.err") (upstream unreadable: checked local tags)" >&2
        fi
        v=$(local_version_at "$path" "$link")
      fi
      if [ -z "$v" ]; then
        nearest="" lt=""
        if [ -e "$path/.git" ]; then
          lt=$(local_version_at "$path" "$link")
          nearest=$(git -C "$path" describe --tags "$link" 2>/dev/null) || nearest=""
        fi
        if [ -n "$lt" ]; then
          bad "$path: tag $lt exists locally but not at $url: push it (git -C $path push origin $lt)"
        else
          bad "$path at $(short "$link") is not a release tag of $url${nearest:+ (nearest: $nearest)}: release it, then graft upgrade $name@TAG (or graft upgrade $name, the newest release of its major)"
        fi
      fi
    done <<<"$registered"
    # a top-level git flake sees no scions without inputs.self.submodules
    if [ "$FLAKE" = . ] && [ -f "$FLAKE_NIX" ] && [ "$n" -gt 0 ] && ! declares_submodules; then
      bad "$FLAKE_NIX doesn't declare inputs.self.submodules = true;, so Nix sees no scions: add it (Nix 2.27 or later)"
    fi
  fi
  bash "$here/workflows.sh" check . "$template/.forgejo/workflows" "$GRAFT_RUN" "$FLAKE" ${SCIONS:+"$SCIONS"} || rc=1
  # every provider evaluates (a graft.nix that throws fails here, naming it): from the repo's own app,
  # which has evaluated the flake already (from outside, -C, nothing of the repo's is evaluated)
  if [ -z "$committed" ] && [ -n "${GRAFT_IN_REPO:-}" ] && [ "$rc" = 0 ] && [ -n "$SCIONS" ]; then
    system=$(nix_ eval --impure --raw --expr builtins.currentSystem)
    if ! err=$( { nix_ eval --raw "$FLAKE_REF#packages.$system.tools.drvPath" >/dev/null && nix_ eval --json "$FLAKE_REF#checks.$system" --apply builtins.attrNames >/dev/null; } 2>&1); then
      grep -v "^warning: Git tree" <<<"$err" | tail -15 >&2
      bad "the flake's tools or checks don't evaluate (above): a provider's graft.nix fails"
    fi
  fi
  if [ "$rc" = 0 ] && [ -n "$SCIONS" ]; then
    local noun=scions
    [ "$n" != 1 ] || noun=scion
    if [ -n "$committed" ]; then echo "graft: $n $noun registered and at release tags"
    else echo "graft: $n $noun clean and at release tags"; fi
  fi
  return "$rc"
}

# describe_commit PATH COMMIT: a scion's commit and the release tag it is, or the nearest before it
describe_commit() {
  local t n
  if t=$(git -C "$1" describe --tags --exact-match "$2" 2>/dev/null); then echo "$t ($(short "$2"))"
  elif t=$(git -C "$1" describe --tags --abbrev=0 "$2" 2>/dev/null); then
    n=$(git -C "$1" rev-list --count "$t..$2")
    if [ "$n" = 1 ]; then n="1 commit"; else n="$n commits"; fi
    echo "$(short "$2"), $n after $t"
  else echo "$(short "$2") (no tag before it)"; fi
}

# new_files PATH: the untracked files git doesn't ignore in the repo at PATH and, recursively, in its
# filled submodules, as REPO<TAB>FILE (REPO from the top)
new_files() {
  local sub f
  while IFS= read -r -d '' f; do
    [[ $f != */ && $f != *$'\n'* && $f != *$'\t'* ]] || continue
    printf '%s\t%s\n' "$1" "$f"
  done < <(git -C "$1" ls-files -z --others --exclude-standard)
  while IFS= read -r sub; do
    { [ -n "$sub" ] && [ -e "$1/$sub/.git" ]; } || continue
    new_files "$1/$sub"
  done < <(git -C "$1" ls-files -s | awk '$1 == "160000" { sub(/^[^\t]*\t/, ""); print }')
}

# try_line PATH KIND: what Nix builds of the scion at PATH: its checkout, against the gitlink, and its
# edits. KIND: git (a git flake sees tracked files only: untracked ones are counted apart) or path (a
# path: flake copies them)
try_line() {
  local p=$1 head link branch where rec mod new un parts=()
  head=$(git -C "$p" rev-parse -q --verify HEAD 2>/dev/null) || head=""
  if [ ! -e "$p/.git" ] || [ -z "$head" ]; then echo "graft: $p is empty: graft sync fills it"; return; fi
  link=$(gitlink "$p")
  if branch=$(git -C "$p" symbolic-ref -q --short HEAD); then where="branch $branch at"; else where="detached at"; fi
  if [ "$head" = "$link" ]; then rec="as recorded"; else rec="not the recorded $(describe_commit "$p" "$link")"; fi
  mod=$(git -C "$p" diff HEAD --name-status --no-renames | awk '$1 != "A"' | wc -l)
  new=$(git -C "$p" diff HEAD --name-status --no-renames | awk '$1 == "A"' | wc -l)
  un=$(new_files "$p" | wc -l)
  [ "$mod" = 0 ] || parts+=("$mod modified")
  if [ "$2" = path ]; then new=$((new + un)); un=0; fi
  [ "$new" = 0 ] || parts+=("$new new")
  [ "$un" = 0 ] || parts+=("$un new Nix doesn't see")
  [ ${#parts[@]} -gt 0 ] || parts=("no edits")
  echo "graft: $p: $where $(describe_commit "$p" "$head"), $rec: $(printf '%s, ' "${parts[@]}" | sed 's/, $//')"
}

# try_add_hint PATH: for each repo (the scion, or one of its own submodules) with new files, the
# command that stages them for real, so that a git flake sees them
try_add_hint() {
  local repo files
  while IFS= read -r repo; do
    [ -n "$repo" ] || continue
    files=$(new_files "$1" | awk -F'\t' -v r="$repo" '$1 == r { print substr($0, length(r) + 2) }' | while IFS= read -r f; do printf ' %q' "$f"; done)
    echo "graft: $repo: Nix sees a new file only once git tracks it: git -C $repo add --$files"
  done < <(new_files "$1" | cut -f1 | awk '!seen[$0]++')
}

cmd_try() {
  local repo="" stop="" names=() all name path url n super
  while [ $# -gt 0 ]; do
    case $1 in
      -C) if [ $# -lt 2 ] || [ -z "$2" ]; then die "-C needs a repository"; fi; repo=$2; shift 2 ;;
      --stop) stop=1; shift ;;
      --) shift ;;
      -h|--help) sed -n '/^#   try /,/^set /p' "${BASH_SOURCE[0]}" | sed '$d; s/^# \{0,1\}//'; exit 0 ;;
      -*) die "unknown option $1 (usage: graft try [-C REPO] [NAME...], or graft try [-C REPO] --stop)" ;;
      *) names+=("$1"); shift ;;
    esac
  done
  [ -z "$stop" ] || [ ${#names[@]} -eq 0 ] || die "try --stop takes no names: it undoes all that try did"
  locate "$repo"
  if [ -z "$SCIONS" ] && super=$(git rev-parse --show-superproject-working-tree 2>/dev/null) && [ -n "$super" ]; then
    die "$TOP is a submodule of $super: run graft from the repository's top (-C $super)"
  fi
  if [ -n "$stop" ]; then try_stop; return; fi
  all=$(scions_registered .gitmodules)
  [ -n "$all" ] || die "no scions here: graft add URL grafts one"
  for name in "${names[@]}"; do
    cut -f1 <<<"$all" | grep -qxF -- "$name" || die "no scion named $name"
  done
  if [ "$FLAKE" != . ]; then
    echo "graft: $FLAKE is a path: flake: every build reads its working tree, scions and new files included; try has nothing to turn on"
    while IFS=$'\t' read -r name path url; do try_line "$path" path; done <<<"$all"
    echo "graft: build from the repository's top: nix build 'path:$FLAKE#tools', nix shell 'path:$FLAKE#tools' -c TOOL, or nix flake check 'path:$FLAKE'"
    return
  fi
  # HEAD holding the marker: refuse before writing anything (marking it again would leave it committed)
  if git cat-file -e "HEAD:$TRY" 2>/dev/null; then
    if [ -n "$(git diff --cached --name-only --diff-filter=D -- "$TRY")" ]; then
      die "$TRY is committed and its removal is staged; nothing changed: commit the removal (git commit -m 'Remove graft try marker'), then graft try"
    fi
    die "$TRY is committed, so the tree is clean to Nix; nothing changed: graft try --stop, commit its removal, then graft try"
  fi
  # Nix reads every scion from the work tree while try is on, and fails on an empty one
  while IFS=$'\t' read -r name path url; do
    [ -e "$path/.git" ] || die "$path is empty, and Nix fails on an empty scion while try is on; nothing changed: git submodule update --init --recursive -- $path (or graft sync)"
    n=$(empty_nested "$path")
    [ -z "$n" ] || die "$path's own submodules are empty ($n), and Nix fails on them while try is on; nothing changed: git -C $path submodule update --init --recursive (or graft sync)"
  done <<<"$all"
  if [ ! -f "$TRY" ] || [ -n "$(git diff HEAD --name-only -- "$TRY")" ] || [ -z "$(git ls-files -- "$TRY")" ]; then
    printf '%s\n' "# graft try is on: Nix builds of this repository read its working tree, and its scions'." \
      "# graft try --stop turns it off (nix run .#graft -- try --stop); graft check fails until then. Never commit this file." > "$TRY"
  fi
  [ -n "$(git ls-files -- "$TRY")" ] || git add -N -f -- "$TRY"
  # a file marked with intent to add is a change against HEAD: Nix reads the work tree
  [ -n "$(git diff HEAD --name-only -- "$TRY")" ] || die "$TRY is marked but the tree is still clean to git: graft try --stop, then graft try"
  echo "graft: try is on: Nix builds of this repository read its working tree, and its scions' ($TRY marks it)"
  while IFS=$'\t' read -r name path url; do try_line "$path" git; done <<<"$all"
  while IFS=$'\t' read -r name path url; do
    [ ${#names[@]} -eq 0 ] || printf '%s\n' "${names[@]}" | grep -qxF -- "$name" || continue
    try_add_hint "$path"
  done <<<"$all"
  echo "graft: build from the repository's top: nix build .#tools, nix shell .#tools -c TOOL, or nix flake check"
  echo "graft: graft try --stop turns try off (before git stash or a pull with --autostash here), and graft check fails until then"
}

# try_stop: undo what try did: remove its marker (a committed one: the removal is staged). Never
# touches a scion.
try_stop() {
  local state name path url head link
  state=$(try_state)
  if [ -z "$state" ]; then echo "graft: try is not on; nothing to undo"; return 0; fi
  [ -z "$(git ls-files -- "$TRY")" ] || git update-index --force-remove -- "$TRY"
  rm -f -- "$TRY"
  echo "graft: try is off: Nix builds the recorded scions while the tree is clean"
  [ "$state" != committed ] || echo "graft: $TRY was committed: its removal is staged; commit it"
  while IFS=$'\t' read -r name path url; do
    { [ -n "$name" ] && [ -e "$path/.git" ]; } || continue
    head=$(git -C "$path" rev-parse -q --verify HEAD 2>/dev/null) || continue
    link=$(gitlink "$path")
    if [ -n "$(git -C "$path" status --porcelain --untracked-files=no)" ]; then
      echo "graft: $path stays as you left it: git submodule update --recursive -- $path goes back to its pin (commit or stash its edits first)"
    elif [ "$head" != "$link" ]; then
      echo "graft: $path stays as you left it: git submodule update --recursive -- $path goes back to its pin"
    fi
  done < <(scions_registered .gitmodules)
}

sub=${1:-}; shift || true
case $sub in
  add) cmd_add "$@" ;;
  sync) cmd_sync "$@" ;;
  check) cmd_check "$@" ;;
  try) cmd_try "$@" ;;
  *) die "usage: scions.sh add|sync|check|try" ;;
esac
