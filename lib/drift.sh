#!/usr/bin/env bash
# drift.sh SEEN PIN COPY DIR LOCK RUN: the drift report of `graft check` (SEEN=git) and of
# `checks.<system>.graft` (SEEN=nix). PIN is the scions directory flake.lock's pins make (mkFlake's
# `expected`), COPY the repo's, as the check sees it: the files git would commit (git), or the
# flake's source (nix), which holds only what git tracks. DIR is the scions directory as the repo
# names it, LOCK its flake.lock and RUN how the repo runs graft, all for the messages.
#
# Exits 0, printing nothing, when the two hold the same files with the same contents (and the same
# symbolic links). Otherwise prints to stderr, naming each file as DIR/NAME/FILE: each edited file
# with a unified diff, each file only in the copy, each file only in the pin; then one line naming
# the fix, and what each kind of file usually is. Exits 1. Directories count only by what they
# hold, as in git.
# Bash, coreutils, findutils and diffutils: mkFlake's check and app put those on PATH.
set -euo pipefail

seen=$1 pin=$2 copy=$3 dir=$4 lock=$5 run=$6
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

# files and symbolic links under a directory, relative to it, sorted, NUL-separated
files() { (cd "$1" && find . -mindepth 1 \( -type f -o -type l \) -printf '%P\0') | LC_ALL=C sort -z; }
files "$pin" > "$tmp/pin"
files "$copy" > "$tmp/copy"

drift=0 onlypin="" onlycopy=0
while IFS= read -r -d '' f; do
  l=$pin/$f r=$copy/$f
  if [ -L "$l" ] || [ -L "$r" ]; then
    if [ -L "$l" ] && [ -L "$r" ] && [ "$(readlink "$l")" = "$(readlink "$r")" ]; then continue; fi
    drift=1
    echo "graft: $dir/$f: edited (a symbolic link, in the pin or in your copy)" >&2
  elif ! cmp -s "$l" "$r"; then
    drift=1
    echo "graft: $dir/$f: edited" >&2
    diff -u --label "$dir/$f (pin)" --label "$dir/$f (your copy)" "$l" "$r" >&2 || true
  fi
done < <(LC_ALL=C comm -z -12 "$tmp/pin" "$tmp/copy")
while IFS= read -r -d '' f; do
  drift=1 onlycopy=1
  echo "graft: $dir/$f: only in your copy, not in the pin" >&2
done < <(LC_ALL=C comm -z -13 "$tmp/pin" "$tmp/copy")
while IFS= read -r -d '' f; do
  drift=1
  [ -n "$onlypin" ] || onlypin=$dir/$f
  echo "graft: $dir/$f: only in the pin, not in your copy" >&2
done < <(LC_ALL=C comm -z -23 "$tmp/pin" "$tmp/copy")

[ "$drift" = 1 ] || exit 0
echo "graft: $dir/ differs from the pins in $lock; run: $run" >&2
if [ "$onlycopy" = 1 ]; then
  echo "graft: a file only in your copy was written by a tool (__pycache__/, a build) or added by someone; $run deletes it" >&2
fi
if [ -n "$onlypin" ]; then
  case $seen in
    git) echo "graft: a file only in the pin was deleted, or git ignores it: git check-ignore -v $(printf '%q' "$onlypin") names the rule" >&2 ;;
    *) echo "graft: a file only in the pin was deleted, is not added to git (nix flake check sees only what git tracks), or git ignores it: git check-ignore -v $(printf '%q' "$onlypin") names the rule" >&2 ;;
  esac
fi
exit 1
