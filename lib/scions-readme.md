<!-- managed by graft: https://github.com/danielnaab/graft -->
# Scions

Each folder here is a git submodule: another repository, grafted at a release tag. Empty folders?
Run `git submodule update --init --recursive` (or `@RUN@ sync`).

- Read them freely. To fix one, see graft's "Fix a provider from a consumer" guide: edit, test, push to the
  provider, release, then `@RUN@ upgrade NAME`.
- `@RUN@ check` fails until every scion is clean and at a release tag.
- Versions: `@RUN@ outdated`.
