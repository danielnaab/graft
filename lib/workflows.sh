#!/usr/bin/env bash
# workflows.sh check|refresh REPO EXAMPLES [RUN]
#
# graft's CI workflows in REPO: each file in REPO/.forgejo/workflows/ named like one in EXAMPLES
# (graft's template/.forgejo/workflows/, at the version the repo pins) whose text has graft's marker line.
# graft owns those and nothing else: a workflow without the marker is yours and is never touched,
# and graft never adds one you didn't copy in.
#
#   check    fail, naming each of graft's workflows that differs from EXAMPLES
#   refresh  replace each of graft's workflows that differs from EXAMPLES with EXAMPLES' copy
#
# mkFlake's check and `nix run .#graft` call this with their own examples; graft upgrade calls the
# copy in the version it moves to, with that version's examples. RUN is how the repo runs graft, for
# the message (default `nix run .#graft`).
set -euo pipefail

mode=$1 repo=$2 examples=$3 run=${4:-nix run .#graft}
marker='# managed by graft: '
rc=0
case $mode in check|refresh) ;; *) echo "usage: workflows.sh check|refresh REPO EXAMPLES [RUN]" >&2; exit 2 ;; esac

# A symlink could point anywhere: graft only writes regular files inside the repo.
for d in .forgejo .forgejo/workflows; do
  [ ! -L "$repo/$d" ] || { echo "graft: $d is a symlink; not reading workflows through it" >&2; exit 1; }
done

for src in "$examples"/*; do
  [ -f "$src" ] || continue
  wf=.forgejo/workflows/${src##*/}
  if [ ! -f "$repo/$wf" ] || [ -L "$repo/$wf" ] || ! grep -q "^$marker" "$repo/$wf"; then continue; fi
  if cmp -s "$src" "$repo/$wf"; then continue; fi
  case $mode in
    check)
      echo "graft: $wf differs from the copy this graft version ships; run: $run" >&2
      echo "graft: (to keep your own edits, delete its \"$marker\" line; graft then leaves it alone)" >&2
      rc=1 ;;
    refresh)
      cp --no-preserve=mode "$src" "$repo/$wf"
      echo "graft: refreshed $wf" ;;
  esac
done
exit "$rc"
