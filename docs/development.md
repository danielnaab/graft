---
status: working
---
# Development

How to change graft, test it, and release it.

## Layout

| Path | Contents |
|---|---|
| `flake.nix` | `lib.mkFlake`, the `graft` package, the template, and graft's own checks. graft grafts its own scions with this checkout's `mkFlake`. |
| `lib/mk-flake.nix` | `mkFlake`: the app, the drift check and the expected scions directory. |
| `lib/graft.sh` | The command: dispatches to the scripts below. |
| `lib/assemble.sh` | Builds a scions directory. The check and `upgrade` both use it, so they cannot disagree. |
| `lib/tools.sh` | `outdated`, `changelog`, `upgrade`, and the annotated-tag repair. |
| `lib/common.sh` | Shared helpers: finds scions directories, reads `flake.lock` as data. |
| `lib/export-ignored.sh` | Fails when a fetched export still holds a path that its export rules exclude. |
| `lib/migrate.sh`, `lib/migrate.jq` | `migrate`. |
| `lib/ci.sh`, `lib/upstream.jq` | `ci`, and the provider text in the pull request. |
| `lib/workflows.sh` | graft's workflow files. |
| `lib/retarget.jq` | Moves a tag in a flake URL. |
| `template/` | What `nix flake init -t` writes. Its workflows are the copies that consumers must match. |
| `tests/run.sh`, `tests/stub-api.py` | The end-to-end test, and a stub of Forgejo's API. |
| `tests/fixtures.sh` | The repositories that a guide is read against. |
| `docs/`, `README.md`, `AGENTS.md` | Documentation, checked against knowledge-base-kit's conventions. |
| `scions/` | graft's own scions. Never edit. |
| `.forgejo/workflows/` | graft's own CI: the template's workflows, kept current by `nix run .#graft`. |

## Test

```
nix flake check          # shellcheck, the docs check, and the drift check of graft's own scions
tests/run.sh             # the whole cycle against throwaway provider repositories
KEEP=1 tests/run.sh      # the same, and keep the work directory to debug a failure
```

`tests/run.sh` covers sync, check and drift; `outdated`, `changelog` and `upgrade`, in the repository and
with `-C`; major versions and `0.y`; graft upgrading its own pin and workflows; the `paths-ignore` and ssh
lines; `graft ci` against a local origin and a stub API; `migrate`; annotated tags; the `dir` cases; and a
flake in a subdirectory, including a check that no private file reaches the Nix store. It takes several
minutes and fetches nixpkgs on its first run. Do not run several copies in parallel.

## Documentation

The docs follow knowledge-base-kit's conventions, grafted in `scions/knowledge-base-kit/`: its
[style policy](https://github.com/danielnaab/knowledge-base-kit/blob/v2.1.0/docs/policies/style.md)
(plain language, ASD-STE100 procedures, one term per concept, Diátaxis document types), and its
[linking](https://github.com/danielnaab/knowledge-base-kit/blob/v2.1.0/docs/policies/linking.md) and
[lifecycle](https://github.com/danielnaab/knowledge-base-kit/blob/v2.1.0/docs/policies/lifecycle.md)
policies. `nix flake check` runs the kit's checker on them, with the scopes in `knowledge-base.yaml`.

graft's own rules:

- Use the terms that [Concepts](concepts.md) defines.
- Reference pages end with a `## Sources` section that links the code they describe.
- A guide is one user goal: a title that names the goal or symptom, numbered steps with the exact command
  and its expected output, a runnable "Check it worked" block, the two to four real failures, and Sources.
  "Not yet: …" marks a known gap.
- Read a new or changed guide literally, against a fixture: `tests/fixtures.sh NAME OUT` builds one
  (`check-fails`, `first-pattern`, `private`). A reader that sees only the page and its links follows each step and says
  what it expects first. Then a reviewer checks its claims against the code.

These links cite the kit by URL, because graft does not export `scions/`. When you upgrade the kit with
`nix run .#graft upgrade`, change their tag too.

## Release

1. Add a section at the top of `CHANGELOG.md`: `## vX.Y.Z - YYYY-MM-DD`. A major version needs an
   `### Upgrading` subsection.
2. Change the graft tag in `template/flake.nix`, `README.md` and the docs to the new version.
3. Run `nix flake check` and `tests/run.sh`.
4. Commit.
5. Tag the commit `vX.Y.Z`.
6. Push the `v2` branch and the tag.
