#!/usr/bin/env bash
# refresh.sh SCIONS FLAKE RUN TEMPLATE: the files graft writes for its version, run from the repo's
# top: SCIONS/README.md (scions-readme.md beside this script, naming RUN, how the repo runs graft),
# when SCIONS is a directory, and graft's workflows (workflows.sh, against TEMPLATE/.forgejo/workflows).
# `graft sync` runs its own; `graft upgrade` runs the one of the graft version it moves to, so
# what it writes is what that version's check expects. Prints a line per file it changed.
set -euo pipefail
here=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
scions=$1 flake=$2 run=$3 template=$4
if [ -n "$scions" ] && [ -d "$scions" ]; then
  want=$(sed "s|@RUN@|$run|g" "$here/scions-readme.md")
  if [ -L "$scions/README.md" ]; then
    echo "graft: $scions/README.md is a symbolic link; graft writes only a file there: remove it, then graft sync" >&2
    exit 1
  fi
  if [ ! -f "$scions/README.md" ] || [ "$(cat "$scions/README.md")" != "$want" ]; then
    # F3: never through a link: a temporary file beside it, moved over
    tmp=$(mktemp "$scions/.graft-XXXXXX")
    printf '%s\n' "$want" > "$tmp"
    mv -fT -- "$tmp" "$scions/README.md"
    git add -- "$scions/README.md" 2>/dev/null || true
    echo "graft: wrote $scions/README.md"
  fi
fi
bash "$here/workflows.sh" refresh . "$template/.forgejo/workflows" "$run" "$flake" ${scions:+"$scions"}
