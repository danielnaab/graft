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
  [ -z "$subs" ] || echo "graft: note: $2 has its own submodules ($subs); Nix fetches them when evaluating"
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
files_msg() { echo "$1 is tracked as files, not as a submodule: git rm -r -q --cached $1 && git add $1 (git then records the submodule's commit)"; }

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
}

cmd_check() {
  local repo="" committed="" rc=0 name path url link head v lt list n=0 d dirt nearest mode registered en err system
  while [ $# -gt 0 ]; do
    case $1 in
      -C) if [ $# -lt 2 ] || [ -z "$2" ]; then die "-C needs a repository"; fi; repo=$2; shift 2 ;;
      --committed) committed=1; shift ;;
      *) die "usage: graft check [-C REPO] [--committed]" ;;
    esac
  done
  locate "$repo"
  bad() { echo "graft: $*" >&2; rc=1; }
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
      [ -n "$d" ] && [ ! -d "$d" ] && bad "$d is a submodule with no folder: git submodule update --init -- $d, or git rm $d"
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
            bad "uncommitted edits in $path: $dirt (Nix builds ignore them while the rest of the tree is clean)"
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
            bad "$path/graft.nix doesn't evaluate (above): every output of this flake fails until it does, graft's own app too; move the scion back (git submodule update --recursive -- $path), or from outside: nix run github:danielnaab/graft/v2.13.0 -- upgrade -C . $name@TAG"
          fi
        fi
      fi
      # the gitlink is a release: upstream, else (unreadable) the scion's own tags
      if list=$(tags_of "$url" 2>"$GRAFT_TMP/ls.err"); then
        v=$(awk -v c="$link" '$1 == c { print $2 }' <<<"$list" | sort -V | tail -1)
      else
        echo "graft: $path: $(sed 's/^graft: //' "$GRAFT_TMP/ls.err") (upstream unreadable: checked local tags)" >&2
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

sub=${1:-}; shift || true
case $sub in
  add) cmd_add "$@" ;;
  sync) cmd_sync "$@" ;;
  check) cmd_check "$@" ;;
  *) die "usage: scions.sh add|sync|check" ;;
esac
