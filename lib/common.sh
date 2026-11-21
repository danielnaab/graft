#!/usr/bin/env bash
# common.sh: functions graft's commands share. Sourced, never run; defines only (and one temporary
# directory, GRAFT_TMP, removed on exit: scripts that source this put their temporary files there
# and set no EXIT trap of their own).
#
#   locate                  the repo's top, its scions directory and the flake beside it
#   scions_registered       the scions .gitmodules registers there: name, path, url
#   gitlink, tags_of, version_at
#                           a scion's pin, its upstream's release tags, and the tag a commit is
#   lock_node, is_semver    a flake.lock's root inputs (graft's own pin), read as data
#   full_check              the repo's whole `nix flake check`, each check by name
#   upstream_input          what changed upstream for graft's pin between two locks
#   upstream_range          what changed upstream between two commits or tags (CHANGELOG sections
#                           or commits), fetched with plain git and returned as JSON
#   changelog_between, cap  the text helpers under them

# graft's markers: the scions README's first line (lib/scions-readme.md; also mkFlake's), and the
# first line of each of graft's workflows (lib/workflows.sh holds its own copy: graft upgrade runs
# another version's).
MARKER="<!-- managed by graft: https://github.com/danielnaab/graft -->"
# shellcheck disable=SC2034 # used by the scripts that source this file
WORKFLOW_MARKER='# managed by graft: '
nix_() { nix --extra-experimental-features 'nix-command flakes' "$@"; }
# nix_clean ARGS: nix_ through an empty fetch cache of this run's own (XDG_CACHE_HOME in GRAFT_TMP), and without the
# global flake registry (which an empty cache would download again; graft names every input in full). A pin it
# locks is the tag's own tree: never one that another fetch of that commit left in the person's cache (a git+file:
# or . fetch keeps the files a provider export-ignores, and a github: input of the commit then locks their hash,
# which a machine without that cache, CI, refuses: NAR hash mismatch).
nix_clean() {
  mkdir -p "$GRAFT_TMP/nix-cache"
  XDG_CACHE_HOME=$GRAFT_TMP/nix-cache nix_ --option flake-registry '' "$@"
}
# git whose output parses the same whatever the user's config: no colour, no grep column
git() { command git -c color.ui=never -c grep.column=false "$@"; }
# graft's lib/, for jq's -L: upstream.jq's helpers (clean: upstream text as plain lines)
GRAFT_LIB=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
GRAFT_TMP=$(mktemp -d)
trap 'rm -rf "$GRAFT_TMP"' EXIT

die() { echo "graft: $*" >&2; exit 1; }

# graft try's marker, at the repo's top: a file marked with intent to add, so that Nix reads the
# working tree (a tracked file differs from HEAD)
TRY=.graft-try
# try_state (from the top): "on" while the index holds the marker and HEAD doesn't; "committed" when
# HEAD holds it and the index still does (a staged removal is off); "left" when it is only an
# untracked file (after a reset): Nix doesn't read the working tree for it; nothing when try is off
try_state() {
  local indexed
  indexed=$(git ls-files -- "$TRY")
  if [ -n "$indexed" ]; then
    if git cat-file -e "HEAD:$TRY" 2>/dev/null; then echo committed; else echo on; fi
  elif [ -e "$TRY" ]; then echo left; fi
}

# The copy-mode README graft v2.12 and earlier wrote: a table of copied scions
copy_mode_readme() { grep -qsF -e '| Scion | Commit |' -e '<!-- graft flake: ' "$1"; }
COPY_MODE="is graft's copy-mode README (v2.12 or earlier): graft v2.13.0 pins scions with git submodules; follow the Upgrading steps of v2.13.0 in graft's CHANGELOG.md (https://github.com/danielnaab/graft/blob/v2/CHANGELOG.md)"

# locate [REPO]: cd to the repo's top (REPO's, with -C, else the current directory's) and set:
#   SCIONS      the scions directory, from the top: the one folder whose README.md carries graft's
#               marker (git grep, which never looks inside a submodule, so a provider's own scions
#               README is never ours), or "" when there is none yet
#   SCIONS_NEW  where a first scion goes when there is none: GRAFT_DIR (mkFlake's dir, set by a
#               repo's own app) beside the flake the app was run from, else `scions` at the top
#   FLAKE       the flake's directory: the scions directory's parent (`.` at the top)
#   FLAKE_REF   the reference that evaluates it: `.`, or `path:/abs/top/DIR` for a flake in a
#               subdirectory, which copies only that directory to the store (absolute, since Nix
#               won't write the lock of a relative path: flake; percent-encoded)
#   FLAKE_NIX FLAKE_LOCK   its flake.nix and flake.lock, from the top
#   GRAFT_RUN   how a person runs its app from the top: `nix run .#graft`, `nix run 'path:DIR#graft'`
#               (quoted for zsh)
# Fails on a copy-mode scions README (graft v2.12), and on two scions directories.
# shellcheck disable=SC2034 # set for the scripts that source this
locate() {
  local out rc=0 d prefix n
  if [ -n "${1:-}" ]; then cd "$1" || die "can't cd to $1"; fi
  git rev-parse --is-inside-work-tree >/dev/null 2>&1 || die "not inside a git work tree: $(pwd)"
  prefix=$(git rev-parse --show-prefix)
  TOP=$(git rev-parse --show-toplevel)
  cd "$TOP" || exit 1
  out=$(git grep -l -F -e "$MARKER" -e '| Scion | Commit |' -- '*/README.md') || rc=$?
  [ "$rc" -le 1 ] || die "can't search $(pwd) with git (exit $rc; see above)"
  SCIONS=""
  n=0
  while IFS= read -r d; do
    [ -n "$d" ] || continue
    ! copy_mode_readme "$d" || die "$d $COPY_MODE"
    grep -qsF -- "$MARKER" "$d" || continue
    n=$((n + 1))
    if [ -n "$SCIONS" ]; then die "two scions directories: $SCIONS, ${d%/README.md}; graft supports one per repository"; fi
    SCIONS=${d%/README.md}
  done <<<"$out"
  if [ -n "$SCIONS" ]; then
    if [[ $SCIONS == */* ]]; then FLAKE=${SCIONS%/*}; else FLAKE=.; fi
    SCIONS_NEW=$SCIONS
  else
    FLAKE=.
    prefix=${prefix%/}
    if [ -n "${GRAFT_DIR:-}" ] && [ -n "$prefix" ] && [ -f "$prefix/flake.nix" ]; then FLAKE=$prefix; fi
    if [ "$FLAKE" = . ]; then SCIONS_NEW=${GRAFT_DIR:-scions}; else SCIONS_NEW=$FLAKE/${GRAFT_DIR:-scions}; fi
  fi
  flake_vars
}

# flake_vars: FLAKE_REF FLAKE_NIX FLAKE_LOCK GRAFT_RUN from FLAKE
# shellcheck disable=SC2034 # set for the scripts that source this
flake_vars() {
  if [ "$FLAKE" = . ]; then
    FLAKE_REF=. FLAKE_NIX=flake.nix FLAKE_LOCK=flake.lock GRAFT_RUN="nix run .#graft"
  else
    FLAKE_REF="path:$(jq -rn --arg p "$(pwd -P)/$FLAKE" '$p | split("/") | map(@uri) | join("/")')"
    FLAKE_NIX=$FLAKE/flake.nix FLAKE_LOCK=$FLAKE/flake.lock GRAFT_RUN="nix run 'path:$FLAKE#graft'"
  fi
}

# kits_just_wanted: whether this repository uses graft's generated kits.just (v2.18.0): the file is there, or
# the repository's justfile imports it. Only then do sync, upgrade and check evaluate it.
kits_just_wanted() {
  [ -n "${SCIONS:-}" ] || return 1
  [ -f "$SCIONS/kits.just" ] && return 0
  [ -f justfile ] && grep -qF "$SCIONS/kits.just" justfile
}

# kits_just_text: the kits.just the flake's pot-kits generates (pot-lib/1's passthru.kitsJust), by evaluation
# alone: no build, nothing copied that the repository's own app has not evaluated already
kits_just_text() {
  local system
  system=$(nix_ eval --impure --raw --expr builtins.currentSystem) || return 1
  nix_ eval --raw "$FLAKE_REF#packages.$system.pot-kits.kitsJust"
}

# kits_just_write: graft sync's and upgrade's step: write SCIONS/kits.just when it is wanted and differs
kits_just_write() {
  local want
  kits_just_wanted || return 0
  if ! want=$(kits_just_text 2>"$GRAFT_TMP/kj.err"); then
    tail -3 "$GRAFT_TMP/kj.err" >&2
    echo "graft: $SCIONS/kits.just: the flake's pot-kits doesn't evaluate (above); left as it is" >&2
    return 0
  fi
  if [ -L "$SCIONS/kits.just" ]; then
    echo "graft: $SCIONS/kits.just is a symbolic link; graft writes only a file there: remove it, then graft sync" >&2
    return 1
  fi
  if [ ! -f "$SCIONS/kits.just" ] || [ "$(cat "$SCIONS/kits.just")" != "$want" ]; then
    if [ -n "${POT:-}" ]; then
      echo "graft: $SCIONS/kits.just is stale: run graft sync on the host (the scions folder is read-only in a run)" >&2
      return 0
    fi
    write_file "$SCIONS/kits.just" "$want" || return 1
    git add -- "$SCIONS/kits.just" 2>/dev/null || true
    echo "graft: wrote $SCIONS/kits.just"
  fi
  kits_just_clashes "$want" >&2 || true
}

# write_file PATH TEXT: TEXT and a newline into PATH, never through a link (F3): a temporary file beside it, moved over
write_file() {
  local tmp
  [ ! -L "$1" ] || { echo "graft: $1 is a symbolic link; graft writes only a file there: remove it, then run graft again" >&2; return 1; }
  tmp=$(mktemp "$(dirname "$1")/.graft-XXXXXX") || return 1
  if ! { printf '%s\n' "$2" > "$tmp" && mv -fT -- "$tmp" "$1"; }; then
    rm -f -- "$tmp"
    return 1
  fi
}

# kits_just_clashes TEXT: the generated recipes that the repository's own justfile also defines (F1: a kit recipe never
# replaces the person's own, even under set allow-duplicate-recipes). Prints one line each; fails when there is one.
kits_just_clashes() {
  local mine theirs n rc=0
  [ -f justfile ] || return 0
  mine=$(sed -nE 's/^@?([A-Za-z][A-Za-z0-9_-]*)( [^:=]*)?:([^=].*)?$/\1/p' justfile | sort -u)
  theirs=$(sed -nE 's/^([A-Za-z][A-Za-z0-9_-]*) \*args:$/\1/p' <<<"$1" | sort -u)
  for n in $(comm -12 <(echo "$mine") <(echo "$theirs")); do
    echo "graft: $SCIONS/kits.just's recipe $n has the name of the justfile's own $n: rename one (a kit's recipe never replaces yours)"
    rc=1
  done
  return "$rc"
}

# flake_update NAME...: re-lock inputs of the flake locate found, as a path: flake when it's in a
# subdirectory (needs Nix 2.19 or later for --flake), so the repo's top is never copied; through an empty
# fetch cache (nix_clean), so the lock holds each input's own tree.
flake_update() {
  if [ "$FLAKE" = . ]; then nix_clean flake update "$@"; else nix_clean flake update --flake "$FLAKE_REF" "$@"; fi
}

# scions_registered [GITMODULES]: name<TAB>path<TAB>url for each submodule that .gitmodules (or the
# file GITMODULES) registers directly in the scions directory, sorted by name. The submodule's name
# in .gitmodules doesn't matter; its folder is the scion's name.
scions_registered() {
  local f=${1:-.gitmodules} key p sub
  [ -n "$SCIONS" ] && [ -f "$f" ] || return 0
  git config -f "$f" --get-regexp '^submodule\..*\.path$' 2>/dev/null | while read -r key p; do
    [[ $p == "$SCIONS"/* && ${p#"$SCIONS"/} != */* ]] || continue
    sub=${key#submodule.}; sub=${sub%.path}
    printf '%s\t%s\t%s\n' "${p#"$SCIONS"/}" "$p" "$(git config -f "$f" --get "submodule.$sub.url" 2>/dev/null || true)"
  done | LC_ALL=C sort
}

# gitlink PATH [TREE]: the commit the repo records for submodule PATH: in the index, or in TREE (a
# commit); nothing when it isn't a submodule there
gitlink() {
  if [ -n "${2:-}" ]; then git ls-tree "$2" -- "$1" | awk '$1 == "160000" { print $3 }'
  else git ls-files -s -- "$1" | awk '$1 == "160000" { print $2 }'; fi
}

# A relative URL (../x.git) resolves against the repo's own remote for git, never for Nix
relative_url() { [[ $1 == ./* || $1 == ../* ]]; }

is_semver() { [[ $1 =~ ^v?[0-9]+\.[0-9]+\.[0-9]+$ ]]; }
semver_key() { sed -E 's/^v//' <<<"$1"; }
# A version's major series: X for X.y.z, and 0.Y for 0.Y.z (semver's 0.x: anything may break).
major_of() {
  local v
  v=$(semver_key "$1")
  if [[ $v == 0.* ]]; then cut -d. -f1-2 <<<"$v"; else cut -d. -f1 <<<"$v"; fi
}
newer() { [ "$(printf '%s\n%s\n' "$(semver_key "$1")" "$(semver_key "$2")" | sort -V | tail -1)" != "$(semver_key "$1")" ]; }

# tags_of URL: "COMMIT TAG" for each semver tag at URL (an annotated tag by the commit it peels to),
# oldest first; listed once per URL per run. Fails, with git's error on stderr, when URL can't be
# listed (tried once per run too: tags_error gives the message again). http.extraHeader is cleared: a
# header the repo configured for its own server (a CI checkout's token) must never be sent to a provider's.
tags_of() {
  local url=$1 key out
  key=$GRAFT_TMP/tags.$(printf '%s' "$url" | cksum | cut -d' ' -f1)
  if [ -f "$key.fail" ]; then echo "graft: $(cat "$key.fail")" >&2; return 1; fi
  # inside a pot run, only what the run reaches is tried: no refusal left behind
  if [ ! -f "$key" ] && [ -n "${POT:-}" ] && ! pot_reaches "$url"; then
    echo "$url is not reachable from this pot run" > "$key.fail"
    echo "graft: $(cat "$key.fail")" >&2
    return 1
  fi
  if [ ! -f "$key" ]; then
    out=$(git -c http.extraHeader= ls-remote --tags -- "$url" 2>&1) \
      || { echo "can't list tags at $url: $(tail -1 <<<"$out")" > "$key.fail"; echo "graft: $(cat "$key.fail")" >&2; return 1; }
    awk '$2 ~ /^refs\/tags\// { t = substr($2, 11); if (t ~ /\^\{\}$/) { peeled[substr(t, 1, length(t) - 3)] = $1 } else { obj[t] = $1 } }
         END { for (t in obj) print (t in peeled ? peeled[t] : obj[t]), t }' <<<"$out" \
      | while read -r c t; do is_semver "$t" && printf '%s %s\n' "$c" "$t"; done \
      | sort -t' ' -k2,2V > "$key.part"
    mv "$key.part" "$key"
  fi
  cat "$key"
}

# tags_error URL: why tags_of URL failed in this run (its message, without `graft: `), or nothing
tags_error() {
  local key
  key=$GRAFT_TMP/tags.$(printf '%s' "$1" | cksum | cut -d' ' -f1)
  [ ! -f "$key.fail" ] || cat "$key.fail"
}

# pot_reaches URL: inside a pot run (POT set), whether the run reaches URL, as pot answers it
# (`pot why URL --json`, pot's contract run-env/1: {"reached": true|false}). No usable answer (an
# older pot, or none on PATH): yes, so the fetch is tried.
pot_reaches() {
  local out r
  out=$(pot why "$1" --json 2>/dev/null) || true   # exit 1 is "not reached"; the JSON says which
  r=$(jq -r 'if (.reached | type) == "boolean" then .reached else "?" end' <<<"$out" 2>/dev/null) || r="?"
  [ "$r" != false ]
}

# version_at URL COMMIT: the highest semver tag at URL that is COMMIT, or nothing. Fails as tags_of.
version_at() {
  local list
  list=$(tags_of "$1") || return 1
  awk -v c="$2" '$1 == c { print $2 }' <<<"$list" | sort -V | tail -1
}

# local_version_at PATH COMMIT: the same from the tags in the scion's own clone at PATH
local_version_at() {
  git -C "$1" tag --points-at "$2" 2>/dev/null | while read -r t; do is_semver "$t" && echo "$t"; done | sort -V | tail -1
}

# tag_commit URL TAG: the commit TAG is at URL, or nothing. Fails as tags_of.
tag_commit() {
  local list
  list=$(tags_of "$1") || return 1
  awk -v t="$2" '$2 == t { print $1 }' <<<"$list"
}

# scion_node URL TAG: a flake.lock-like node for a scion, for upstream_range: github: when URL is
# GitHub's (a compare link), else git
scion_node() {
  jq -n -c --arg url "$1" --arg ref "$2" '
    ([$url | capture("^https://github\\.com/(?<o>[A-Za-z0-9._-]+)/(?<r>[A-Za-z0-9._-]+?)(\\.git)?/?$")] | .[0]) as $g
    | if $g then {original: {type: "github", owner: $g.o, repo: $g.r, ref: $ref}}
      else {original: {type: "git", url: $url, ref: $ref}} end'
}

# graft outdated's rows, which graft status reads too (remote, row, scion_pin, report)

remote() {  # the git URL of a locked input's upstream
  jq -r 'if .original.type=="github" then "https://github.com/\(.original.owner)/\(.original.repo).git"
         elif .original.type=="git" then .original.url
         else empty end' <<<"$1"
}

# row NAME DIR URL PINNED: name<TAB>dir<TAB>pinned<TAB>latest<TAB>status<TAB>major. PINNED is a tag,
# or "untagged:WHAT" (shown as WHAT), or "unknown". latest is the newest tag of the pinned major;
# status one of current|outdated|untagged|unknown, against it; major the newest tag of a later
# major series, or -.
row() {
  local name=$1 dir=$2 url=$3 ref=$4 tags t series latest="" major="" status
  case $ref in
    untagged:*:*)
      # an untagged scion with a release behind it (git describe): its major's newest is the target
      # `graft upgrade NAME` takes; the status stays untagged
      t=${ref#untagged:}
      row "$name" "$dir" "$url" "${t#*:}" | awk -F'\t' -v OFS='\t' -v p="${t%%:*}" '{ $3 = p; $5 = "untagged"; print }'
      return ;;
    untagged:*) printf '%s\t%s\t%s\t-\tuntagged\t-\n' "$name" "$dir" "${ref#untagged:}"; return ;;
    unknown:*) printf '%s\t%s\t%s\t-\tunknown\t-\n' "$name" "$dir" "${ref#unknown:}"; return ;;
  esac
  if ! tags=$(tags_of "$url" | awk '{ print $2 }'); then
    printf '%s\t%s\t%s\t-\tunknown\t-\n' "$name" "$dir" "$ref"; return
  fi
  # spelled like the pinned tag (with or without the v)
  if [[ $ref == v* ]]; then tags=$(grep '^v' <<<"$tags" || true); else tags=$(grep -v '^v' <<<"$tags" || true); fi
  series=$(major_of "$ref")
  while IFS= read -r t; do
    [ -n "$t" ] || continue
    if [ "$(major_of "$t")" = "$series" ]; then latest=$t
    elif newer "$ref" "$t"; then major=$t; fi
  done <<<"$tags"
  status=current
  if [ -z "$tags" ]; then status=unknown; latest=-
  elif [ -z "$latest" ]; then latest=$ref
  elif newer "$ref" "$latest"; then status=outdated
  else latest=$ref
  fi
  printf '%s\t%s\t%s\t%s\t%s\t%s\n' "$name" "$dir" "$ref" "$latest" "$status" "${major:--}"
}

# scion_pin PATH URL: the pinned tag of the scion at PATH (the gitlink's version upstream), or
# untagged:SHA12, or unknown:SHA12 when upstream can't be listed
scion_pin() {
  local link v
  link=$(gitlink "$1")
  [ -n "$link" ] || { echo "unknown:-"; return; }
  if ! v=$(version_at "$2" "$link"); then
    # upstream unreadable: the scion's own tags say what it is, if it's filled
    v=$(local_version_at "$1" "$link"); echo "unknown:${v:-${link:0:12}}"; return
  fi
  if [ -n "$v" ]; then echo "$v"; return; fi
  # the nearest release behind it, from the scion's own clone (filled only)
  v=$(git -C "$1" describe --tags --abbrev=0 "$link" 2>/dev/null) || v=""
  if is_semver "$v"; then echo "untagged:${link:0:12}:$v"; else echo "untagged:${link:0:12}"; fi
}

# Rows for graft's own input (DIR `-`), when the flake has one, then for every scion.
report() {
  local n ref url name path
  n=$(lock_node "$FLAKE_LOCK" graft)
  if [ -n "$n" ] && ! scions_registered .gitmodules | cut -f1 | grep -qxF graft; then
    ref=$(jq -r '.original.ref // empty' <<<"$n"); url=$(remote "$n")
    if [ -z "$url" ] || ! is_semver "$ref"; then
      ref="untagged:${ref:-$(jq -r '.locked.rev[0:12] // .original.type // "-"' <<<"$n")}"
    fi
    row graft - "$url" "$ref"
  fi
  while IFS=$'\t' read -r name path url; do
    [ -n "$name" ] || continue
    row "$name" "$SCIONS" "$url" "$(scion_pin "$path" "$url")"
  done < <(scions_registered .gitmodules)
}

# archive_hash URL TAG REV [EXPORT]: the NAR hash (SRI) of `git archive` of commit REV, fetched from URL by its tag
# TAG (else by the commit) into a bare repository of graft's own (no checkout: no hooks or filters run), without
# http.extraHeader. With EXPORT, under the provider's export-ignore and export-subst rules: the tree of GitHub's
# tarball, and of a git input with exportIgnore; without, a git input's whole tree. Hashed by Nix's own tarball
# unpacking, through an empty fetch cache. Fails, with the reason on stderr, when it can't.
archive_hash() {
  local url=$1 tag=$2 rev=$3 ei=${4:-} g out
  g=$(mktemp -d -p "$GRAFT_TMP") && git init -q --bare "$g/git" || return 1
  out=$(git -C "$g/git" -c http.extraHeader= fetch -q --depth 1 --no-tags -- "$url" "refs/tags/$tag" 2>&1) || true
  if ! git -C "$g/git" cat-file -e "$rev^{commit}" 2>/dev/null; then
    out=$(git -C "$g/git" -c http.extraHeader= fetch -q --depth 1 --no-tags -- "$url" "$rev" 2>&1) \
      || { tail -1 <<<"$out" >&2; return 1; }
  fi
  if [ -z "$ei" ]; then mkdir -p "$g/git/info" && printf '* -export-ignore -export-subst\n' > "$g/git/info/attributes"; fi
  git -C "$g/git" archive --format=tar --prefix=source/ -o "$g/archive.tar" "$rev" || return 1
  nix_clean flake prefetch --json "tarball+file://$g/archive.tar" 2>"$g/err" | jq -er .hash \
    || { tail -1 "$g/err" >&2; return 1; }
}

# pin_hash_problems: graft check's message (one line, without `graft: `) for each root input of the flake's lock,
# pinned at a version tag (github:, or git), whose narHash is not the NAR hash of its tag's `git archive`
# (archive_hash): the sign of a lock written through a polluted fetch cache. A pin it can't fetch gets a note on
# stderr, and no message.
pin_hash_problems() {
  local name n type url ref rev nar ei sub want err upd
  [ -f "$FLAKE_LOCK" ] || return 0
  if [ "$FLAKE" = . ]; then upd="nix flake update"; else upd="nix flake update --flake $FLAKE_REF"; fi
  while IFS= read -r name; do
    n=$(lock_node "$FLAKE_LOCK" "$name")
    [ -n "$n" ] || continue
    IFS=$'\x1f' read -r type ref rev nar ei sub < <(jq -r '[.locked.type // "", .original.ref // .locked.ref // "",
      .locked.rev // "", .locked.narHash // "", (if (.locked.exportIgnore // .original.exportIgnore // false) then "1" else "" end),
      (if (.locked.submodules // .original.submodules // false) then "1" else "" end)] | join("\u001f")' <<<"$n")
    case $type in github) ei=1 ;; git) ;; *) continue ;; esac
    if ! is_semver "$ref" || ! [[ $rev =~ ^[0-9a-f]{40}$ ]] || [ -z "$nar" ] || [ -n "$sub" ]; then continue; fi
    url=$(remote "$n")
    [ -n "$url" ] || continue
    if [ -n "${POT:-}" ] && ! pot_reaches "$url"; then
      echo "graft: $url is not reachable from this pot run: $FLAKE_LOCK's narHash for $name not compared with its tag's git archive" >&2
      continue
    fi
    if ! want=$(archive_hash "$url" "$ref" "$rev" "$ei" 2>"$GRAFT_TMP/ah.err"); then
      err=$(tail -1 "$GRAFT_TMP/ah.err")
      echo "graft: can't fetch $name's $ref from $url${err:+ ($err)}: $FLAKE_LOCK's narHash for it not compared with its tag's git archive" >&2
      continue
    fi
    [ "$want" = "$nar" ] && continue
    echo "$FLAKE_LOCK pins $name at $ref (${rev:0:7}) with narHash $nar, but its tag's git archive hashes to $want: Nix's fetch cache probably gave the lock a tree that another fetch of that commit left there (a git+file: or . fetch keeps the files a provider export-ignores), and a machine without that cache, such as CI, fails with NAR hash mismatch: re-lock $name through an empty fetch cache: XDG_CACHE_HOME=\$(mktemp -d) $upd $name"
  done < <(jq -r '.nodes.root.inputs // {} | keys[]' "$FLAKE_LOCK")
}

# One root input's node in the flake.lock at $1, as JSON, or nothing.
lock_node() { [ -f "$1" ] || return 0; jq -c --arg n "$2" '.nodes.root.inputs[$n] as $k | if ($k|type)=="string" then .nodes[$k] else empty end' "$1"; }

# log_tail LOG: the last 30 lines of a check's log that a reader of the PR can use: not Nix's
# progress lines (a dirty tree, what will be built or fetched, the builder's log repeated, where the
# full log is, the output paths: as Nix 2.24 and 2.34 print them), store paths shortened to
# /nix/store/…-NAME, and no path of the runner's checkout
log_tail() {
  grep -vE "^warning: Git tree '.*' is dirty\$|^(this derivation|these [0-9]+ derivations) will be built:\$|^(this path|these [0-9]+ paths) will be fetched|^ +/nix/store/[^ ]+\$|^(building|copying path) '/nix/store/|^ +[Ll]ast [0-9]+ log lines:\$|^ +> |^ +For full logs, run|^ +nix log /nix/store/|^ +Output paths:\$" "$1" \
    | sed -E 's#/nix/store/[0-9a-z]{32}-#/nix/store/…-#g' \
    | awk -v p="$(pwd -P)/" -v q="$PWD/" '{ while ((i = index($0, p)) > 0) $0 = substr($0, 1, i - 1) substr($0, i + length(p))
                                         while ((i = index($0, q)) > 0) $0 = substr($0, 1, i - 1) substr($0, i + length(q)); print }' \
    | tail -n 30 || true
}

# full_check REPORT: the repo's whole check (of the flake locate found): each of checks.<system>
# built on its own (so a failure has a name), then `nix flake check` for the rest (evaluation, other
# systems' outputs, mkFlake's errors). A git flake reads the committed scions while the tree is
# clean, so ci upgrade commits first. Logs go to stdout as they run. Fails if anything did, after
# writing to REPORT one line "nix flake check failed: NAME..." and the last lines of each failing log.
full_check() {
  local report=$1 system names n log failed=() tails=""
  system=$(nix_ eval --impure --raw --expr builtins.currentSystem)
  log=$(mktemp -p "$GRAFT_TMP")
  if names=$(nix_ eval --json "$FLAKE_REF#checks.$system" --apply builtins.attrNames 2>"$log" | jq -r '.[]'); then
    while IFS= read -r n; do
      [ -n "$n" ] || continue
      echo "graft: checks.$system.$n"
      [[ $n =~ ^[A-Za-z0-9_][A-Za-z0-9_\'-]*$ ]] || { echo "graft: (an odd name: left to nix flake check)"; continue; }
      if ! nix_ build --no-link -L "$FLAKE_REF#checks.$system.$n" 2>&1 | tee "$log"; then
        failed+=("$n"); tails+="$n:"$'\n'"$(log_tail "$log")"$'\n\n'
        echo "::error::check $n failed on the upgraded tree"
      fi
    done <<<"$names"
  else
    cat "$log" >&2
    failed+=("checks"); tails+="checks (evaluating checks.$system):"$'\n'"$(log_tail "$log")"$'\n\n'
  fi
  if ! nix_ flake check -L "$FLAKE_REF" 2>&1 | tee "$log"; then
    if [ ${#failed[@]} -eq 0 ]; then
      failed+=("flake-check"); tails+="nix flake check:"$'\n'"$(log_tail "$log")"$'\n\n'
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
  upstream_range "$name" "$o" "$ov" "$nv" "$(lock_ref "$o")" "$(lock_ref "$nw")" "$status"
}

# lock_ref NODE: the ref a lock node pins: flake.nix's (original.ref), or under
# `nix flake lock --override-input` the override's (locked.ref, when it differs: a provider's branch,
# say). Nothing for an input flake.nix names without a ref, whose locked.ref is only Nix's default
# branch (refs/heads/main).
lock_ref() { jq -r '.original.ref as $o | if $o and .locked.ref and .locked.ref != $o then .locked.ref else $o // empty end' <<<"$1"; }

# upstream_range NAME NODE FROM_REV TO_REV FROM_REF TO_REF [STATUS]: what changed upstream for an
# input (NODE, its flake.lock node, says where it comes from) between two commits, as one JSON
# object. TO_REV may be empty when TO_REF is a version tag: the tag is fetched and its commit used
# (graft changelog, which has no lock for the target). A rev must be a full commit hash and a ref a
# version tag before git sees it; a ref that isn't one is never fetched: a plain branch name is only
# shown (`lifecycle at COMMIT`), and when FROM_REF is a version tag, the CHANGELOG.md sections above
# it at TO_REV are taken (a provider's unreleased major on a branch). STATUS, if given, is reported
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
  local name=$1 o=$2 ov=$3 nv=$4 ot=$5 nt=$6 status=${7:-} url cmp tmp hist=false branch="" upto
  is_semver "$ot" || ot=""
  if ! is_semver "$nt"; then
    [[ ! $nt =~ ^[A-Za-z0-9][A-Za-z0-9._/-]{0,99}$ ]] || branch=$nt
    nt=""
  fi
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
      # up to the new tag; at a commit that isn't one, every version above the old tag
      upto=${nt:-999999999.0.0}
      if [ -n "$ot" ] && [ -s "$tmp/cl.md" ]; then
        changelog_between "$ot" "$upto" all < "$tmp/cl.md" > "$tmp/changelog" || : > "$tmp/changelog"
        changelog_between "$ot" "$upto" upgrading < "$tmp/cl.md" > "$tmp/upgrading" || : > "$tmp/upgrading"
        changelog_between "$ot" "$upto" rest < "$tmp/cl.md" > "$tmp/rest" || : > "$tmp/rest"
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
  if [ -z "$nt" ] && [ -n "$branch" ]; then nt="$branch at ${nv:0:12}"; fi
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
# Upgrading subsections (a `### Upgrading…` or `### Migration…` heading, to the next `###` or `##`), under their
# version's heading; `rest` everything else. In either part, a version with nothing left in it
# prints nothing, not even its heading.
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
    # for; in `upgrading` and `rest`, its version heading goes first, once, held until the version
    # has a line for that part (in `rest`, a line that is not blank: the blank lines after the
    # heading are held with it)
    function emit(l, raw) {
      if (part == "all" || (part == "upgrading") == upg) {
        if (part == "upgrading" && pending != "") { if (printed) print ""; print pending; print ""; pending = "" }
        if (part == "rest" && pending != "") {
          if (l ~ /^[ \t]*$/) { pending = pending "\n" l; return }
          print pending; pending = ""
        }
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
          upg = 0; pending = ""
          if (keep && part != "all") { pending = shift(l); continue }
        }
        if (l ~ /^# /) { keep = 0; pending = "" }
        if (!keep) continue
        if (l ~ /^### /) upg = (tolower(l) ~ /^### +(upgrading|migration)/)
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
