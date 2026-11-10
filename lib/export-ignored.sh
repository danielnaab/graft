#!/usr/bin/env bash
# export-ignored.sh RUN NAME=PATH...: fail, naming them, for each fetched export at PATH (a git
# input fetched with exportIgnore) that still holds a path its own .gitattributes marks
# export-ignore. That is Nix's exportIgnore having ignored nothing, which is what an annotated
# tag's object in flake.lock does (lib/common.sh, "Annotated tags"); RUN is how the repo runs graft,
# which re-pins the tag's commit. Fails too, saying so, when it can't tell.
#
# Each path is asked about as Nix's filter asks: by its own name, with no trailing slash, so a
# pattern only a directory matches (`tests/`) matches nothing here, as in Nix's fetch from a bare
# repo, and case-sensitively. Only the export's own .gitattributes files count: no global, system,
# template or info/attributes, whatever the user's git config says.
# Bash, git, find and coreutils: mkFlake's check and app put those on PATH.
set -euo pipefail

run=$1
shift
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
# git with none of the user's config
git_() {
  HOME=$tmp XDG_CONFIG_HOME=$tmp GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null GIT_ATTR_NOSYSTEM=1 \
    git -c core.attributesFile=/dev/null -c core.ignoreCase=false "$@"
}
rc=0
for spec in "$@"; do
  name=${spec%%=*} tree=${spec#*=}
  rm -rf "$tmp/r" && mkdir -p "$tmp/r"
  git_ init -q --template= "$tmp/r"
  # only the .gitattributes files, where they sit
  (cd "$tree" && find . -name .gitattributes -type f -printf '%P\0') > "$tmp/attrs"
  while IFS= read -r -d '' f; do
    mkdir -p "$tmp/r/$(dirname "$f")" && cp "$tree/$f" "$tmp/r/$f"
  done < "$tmp/attrs"
  (cd "$tree" && find . -mindepth 1 -printf '%P\0') > "$tmp/paths"
  git_ -C "$tmp/r" check-attr -z --stdin export-ignore < "$tmp/paths" > "$tmp/out" \
    || { echo "graft: $name: can't read its .gitattributes with git check-attr (see above)" >&2; rc=1; continue; }
  hits=()
  while IFS= read -r -d '' path && IFS= read -r -d '' _ && IFS= read -r -d '' value; do
    if [ "$value" = set ]; then hits+=("$path"); fi
  done < "$tmp/out"
  [ ${#hits[@]} -gt 0 ] || continue
  rc=1
  echo "graft: $name: its fetched export still holds what its .gitattributes marks export-ignore: ${hits[*]:0:5}$([ ${#hits[@]} -le 5 ] || echo " ...")" >&2
  echo "graft: (Nix ignored nothing: flake.lock pins an annotated tag's object, not its commit. $run re-pins the commit; see graft's README, \"Annotated tags\")" >&2
done
exit "$rc"
