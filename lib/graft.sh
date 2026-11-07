#!/usr/bin/env bash
# graft.sh [COMMAND [ARGS...]]: the one command, `nix run .#graft` in a repo that uses graft and
# `nix run github:danielnaab/graft -- COMMAND` against any checkout. Its package (flake.nix) puts
# every tool the commands need on PATH.
#
#   sync      rebuild the scions directory from flake.lock (the default in a repo's own flake)
#   outdated  graft's and each scion's pinned tag against the newest upstream (lib/tools.sh)
#   upgrade   move graft and scions to newer tags, re-lock, rebuild (lib/tools.sh)
#   changelog what upstream changed between each pin and the tag upgrade would take (lib/tools.sh)
#   migrate   print a migration brief for an upgraded branch (lib/migrate.sh)
#   ci        the steps of graft's Forgejo workflows (lib/ci.sh)
#
# sync is built for one repo from its inputs (lib/mk-flake.nix), which hands it over as GRAFT_SYNC;
# the others read a repo as data, so they also run from graft's own flake with -C.
set -euo pipefail

here=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)

usage() {
  cat <<'USAGE'
usage: nix run .#graft [COMMAND] (a flag needs `--`: nix run .#graft -- outdated --json)

  sync                                  rebuild the scions directory from flake.lock (the default)
  outdated [-C REPO] [--json]           pinned tags against the newest upstream
  upgrade [-C REPO] [NAME[@TAG]...]     move graft and scions to newer tags, then rebuild
  changelog [-C REPO] [--json] [NAME[@TAG]...]
                                        what upstream changed between each pin and its target
  migrate [-C REPO] [--base REF] [--no-check] [--json]
                                        a migration brief for an upgraded branch
  ci check|upgrade|pr                   the steps of graft's Forgejo workflows
USAGE
}

sync=${GRAFT_SYNC:-}
unset GRAFT_SYNC
cmd=${1:-}
[ $# -eq 0 ] || shift
if [ -z "$cmd" ] && [ -n "$sync" ]; then cmd=sync; fi
case $cmd in
  sync)
    [ -n "$sync" ] || { echo "graft: sync runs from the repo's own flake: nix run .#graft" >&2; exit 2; }
    [ $# -eq 0 ] || { echo "graft: sync takes no arguments" >&2; exit 2; }
    exec "$sync" ;;
  outdated|upgrade|changelog) exec bash "$here/tools.sh" "$cmd" "$@" ;;
  migrate) exec bash "$here/migrate.sh" "$@" ;;
  ci) exec bash "$here/ci.sh" "$@" ;;
  help|-h|--help) usage ;;
  *) usage >&2; exit 2 ;;
esac
