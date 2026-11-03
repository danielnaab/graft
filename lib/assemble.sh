#!/usr/bin/env bash
# assemble.sh OUT DIR NAME=PATH=REV...
#
# Builds a scions directory at OUT: one folder per scion, copied from its fetched export at PATH,
# and a README naming each pinned REV. DIR is the directory's path in the repo, for the README's
# heading. mkGraft's derivation (what `nix flake check` expects) and graft-upgrade (what an upgrade
# writes) both call this, so the two can't drift apart.
set -euo pipefail

out=$1 dir=$2
shift 2

mkdir -p "$out"
{
  echo "<!-- managed by graft: https://github.com/danielnaab/graft -->"
  echo "# $dir/"
  echo
  echo "Scions: each folder is another repository's export, grafted here by"
  echo "[graft](https://github.com/danielnaab/graft) at the commit pinned in \`flake.lock\`. Do not edit"
  echo "them. To upgrade: \`nix run .#graft-upgrade\` (or \`nix flake update <name>\`, then \`nix run .#graft\`),"
  echo "then commit. \`nix flake check\` fails if this directory drifts from the pins."
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
