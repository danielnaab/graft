<!-- managed by graft: https://github.com/danielnaab/graft -->
# Scions

Each folder here is a git submodule: another repository, grafted at a release tag. Empty folders?
Run `git submodule update --init` (or `nix run .#graft sync`).

- Read them freely. To fix one, see graft's "Fix a provider from a consumer" guide: edit, test, push to the
  provider, release, then `nix run .#graft upgrade NAME`.
- `nix run .#graft check` fails until every scion is clean and at a release tag.
- Versions: `nix run .#graft outdated`.
