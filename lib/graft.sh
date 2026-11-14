#!/usr/bin/env bash
# graft.sh [COMMAND [ARGS...]]: the one command, `nix run .#graft` in a repo that uses graft (its
# own app, from mkFlake: GRAFT_IN_REPO and GRAFT_DIR set) and `nix run github:danielnaab/graft --
# COMMAND -C REPO` against any checkout. Its package (flake.nix) puts every tool the commands need
# on PATH, and GRAFT_TEMPLATE names this version's workflows.
#
#   add       graft another repository as a submodule at a release tag (lib/scions.sh)
#   sync      fill empty scions, write graft's README and workflows (the default in a repo's own app)
#   check     every scion registered, initialised, clean and at a release tag (lib/scions.sh)
#   try       make Nix builds read the scions' working trees, new files included, and undo it
#             (lib/scions.sh)
#   outdated  graft's and each scion's tag against the newest upstream (lib/tools.sh)
#   upgrade   move graft and scions to newer tags (lib/tools.sh)
#   changelog what upstream changed between each pin and the tag upgrade would take (lib/tools.sh)
#   migrate   print a migration brief for an upgraded branch (lib/migrate.sh)
#   ci        the steps of graft's Forgejo workflows (lib/ci.sh)
set -euo pipefail

here=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)

usage() {
  cat <<'USAGE'
usage: nix run .#graft [COMMAND] (a flag needs `--`: nix run .#graft -- outdated --json)
   or: nix run github:danielnaab/graft -- COMMAND -C REPO

  add [-C REPO] [--scions PATH] URL[@TAG] [NAME]
                                        graft URL as a submodule at its newest (or TAG) release
  sync                                  fill empty scions; write graft's README and workflows (the
                                        default; a repo's own app only)
  check [-C REPO] [--committed]         every scion registered, initialised, clean, at a release tag
  try [-C REPO] [NAME...]               make Nix builds read the scions' working trees until
                                        try --stop; say how to make the named ones' (default: all)
                                        new files visible
  try [-C REPO] --stop                  undo what try did
  outdated [-C REPO] [--json]           tags against the newest upstream
  upgrade [-C REPO] [NAME[@TAG]...]     move graft and scions to newer tags
  changelog [-C REPO] [--json] [NAME[@TAG]...]
                                        what upstream changed between each pin and its target
  migrate [-C REPO] [--base REF] [--no-check] [--json]
                                        a migration brief for an upgraded branch
  ci check|upgrade|pr                   the steps of graft's Forgejo workflows
USAGE
}

cmd=${1:-}
[ $# -eq 0 ] || shift
if [ -z "$cmd" ] && [ -n "${GRAFT_IN_REPO:-}" ]; then cmd=sync; fi
case $cmd in
  add|sync|check|try) exec bash "$here/scions.sh" "$cmd" "$@" ;;
  outdated|upgrade|changelog) exec bash "$here/tools.sh" "$cmd" "$@" ;;
  migrate) exec bash "$here/migrate.sh" "$@" ;;
  ci) exec bash "$here/ci.sh" "$@" ;;
  help|-h|--help) usage ;;
  *) usage >&2; exit 2 ;;
esac
