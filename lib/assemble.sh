#!/usr/bin/env bash
# assemble.sh OUT DIR NAME=PATH=REV...
#
# Builds a scions directory at OUT: one folder per scion, copied from its fetched export at PATH,
# and a README naming each pinned REV. DIR is the directory's path in the repo, for the README's
# heading. mkFlake's derivation (what `nix flake check` expects) and graft upgrade (what an upgrade
# writes) both call this, so the two can't drift apart.
#
# GRAFT_FLAKE_DIR is where the repo's graft flake sits under its top (mkFlake's flakeDir): unset or
# `.` for the top. A subdirectory is recorded in the README's second line,
# `<!-- graft flake: DIR -->`, which is how graft's commands and workflows find the flake, and the
# README's commands name it.
set -euo pipefail

out=$1 dir=$2
shift 2
fd=${GRAFT_FLAKE_DIR:-.}
if [ "$fd" = . ]; then
  # shellcheck disable=SC2016 # backticks here are markdown
  run='nix run .#graft' check='`nix flake check`' lock='flake.lock' nixf='flake.nix'
else
  run="nix run path:$fd#graft" check="\`nix run path:$fd#graft check\`" lock="$fd/flake.lock" nixf="$fd/flake.nix"
fi
# a hand-edited ref: Nix won't write the lock of a relative path: flake, so re-lock it by its full path
relock=""
if [ "$fd" != . ]; then relock=", \`nix flake lock \"path:\$PWD/$fd\"\`"; fi

mkdir -p "$out"
{
  echo "<!-- managed by graft: https://github.com/danielnaab/graft -->"
  if [ "$fd" != . ]; then echo "<!-- graft flake: $fd -->"; fi
  echo "# $dir/"
  echo
  echo "Scions: each folder is another repository's export, grafted here by"
  echo "[graft](https://github.com/danielnaab/graft) at the commit pinned in \`$lock\`. Do not edit"
  echo "them. To upgrade: \`$run upgrade\` (or change the tag in \`$nixf\`$relock, then"
  echo "\`$run\`), then commit. $check fails if this directory drifts from the pins."
  echo
  echo "| Scion | Commit |"
  echo "|---|---|"
  for spec in "$@"; do
    name=${spec%%=*}
    rev=${spec##*=}
    echo "| [$name]($name/) | \`$rev\` |"
  done
} > "$out/README.md"

for spec in "$@"; do
  name=${spec%%=*}
  path=${spec#*=}
  path=${path%=*}
  cp -R --no-preserve=mode "$path" "$out/$name"
done

# git can't hold an empty directory, so a checkout never has one: drop them here too
find "$out" -mindepth 1 -type d -empty -delete
