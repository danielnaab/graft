---
status: working
---
# Development

How to change graft, test it, and release it.

## Layout

graft and pot, graft's sandbox, share one repository and one flake, with a one-way boundary between them: graft
calls pot's library, and pot never reads graft's files or conventions. graft's command is built from `lib/` and
`template/` alone, so a change under `pot/` leaves it unchanged (`tests/run.sh own` proves it).

| Path | Contents |
|---|---|
| `flake.nix` | `lib.mkFlake`, `lib.pot.kitsFor`, `lib.mkRoot`, `kits.claude-code`, `homeManagerModules.pot`, the `graft` package (`packages.default`), graft's default pot root (`packages.pot`), graft's own app (`apps.graft`), the template, and the checks. It does not call `mkFlake` on itself and does not declare `inputs.self.submodules` ([below](#test)). |
| `lib/mk-flake.nix` | `mkFlake`: discovery of scions, `graft.nix` calls, the merge of outputs, the evaluation errors, and the call to `kitsFor`. It reads no kit field. |
| `lib/graft.sh` | The command: dispatches to the scripts below; `sync` and `check`. |
| `lib/tools.sh` | `add`, `outdated`, `changelog`, `upgrade`. |
| `lib/common.sh` | Shared helpers: finds the marker README and the flake, reads `.gitmodules` and tags, and asks `pot why URL --json` inside a pot run. |
| `lib/assemble.sh` | A stub: it fails, with a link to the v2.13.0 Upgrading steps, when a v2.12 `upgrade` runs this version. |
| `lib/migrate.sh`, `lib/migrate.jq` | `migrate`. |
| `lib/ci.sh`, `lib/upstream.jq` | `ci`, and the provider text in the pull request. |
| `lib/workflows.sh` | graft's workflow files. |
| `template/` | What `nix flake init -t` writes. Its workflows are the copies that consumers must match. |
| `pot/core/` | pot's Python: the command (`pot.py`), composition and checks (`compose.py`), the bubblewrap backend, the proxy, the broker and the shims. |
| `pot/nix/` | `root.nix` (`mkRoot`, the standard set) and `hm.nix` (`homeManagerModules.pot`). |
| `pot/lib/` | `kits-for.nix` (`kitsFor`, the seam that graft calls) and `kit.nix` (a kit as a manifest). |
| `pot/kits/` | The agent kits: `claude-code.nix`. |
| `pot/contracts/` | [pot's contracts](../pot/contracts/README.md): the formats pot shares, each versioned. |
| `pot/tests/` | pot's unit tests, its fixture bundles, and the golden grant test (`golden.json`). |
| `dev/flake.nix` | Development-only inputs (knowledge-base-kit) and the docs check, kept out of graft's lock and so out of consumers' locks. |
| `tests/run.sh`, `tests/stub-api.py` | The end-to-end test, and a stub of Forgejo's API. |
| `tests/fixtures.sh` | The repositories that a guide is read against. |
| `docs/`, `README.md`, `AGENTS.md` | Documentation, checked against knowledge-base-kit's conventions. |
| `scions/` | graft's own scion, knowledge-base-kit, a submodule, for reading. The docs check reads the same commit from `dev/flake.nix`'s `knowledge-base-kit` input. |
| `.forgejo/workflows/` | graft's own CI: the template's workflows, kept current by `nix run .#graft sync`. |

## Test

```
git submodule update --init --recursive      # graft's own scion, to read its conventions
nix flake check                  # bash -n and shellcheck, and pot's checks
nix flake check ./dev            # the docs check
nix run .#graft check            # the scion is clean and at a release tag
tests/run.sh                     # the whole cycle against throwaway provider repositories
tests/run.sh mkflake kits pot    # some sections only
KEEP=1 tests/run.sh              # the same, and keep the work directory to debug a failure
```

`nix flake check` runs `shellcheck`, pot's unit tests (`pot-unit`), the golden grant test (`pot-golden`: the
canonical grant items and digests of the claude-code instances, a standing kit and a repo kit, as v2.15.0 wrote
them; never regenerate its fixture to make it pass), and checks that roots and the home-manager module build
(`pot-root`, `pot-root-claude`, `pot-hm`).

graft's flake is other flakes' input (`github:` for consumers, `path:` and a tarball in the tests), so it must
not declare `inputs.self.submodules`: Nix applies the attribute to that fetch, and those schemes reject it
(`error: input attribute 'submodules' not supported by scheme 'github'`; graft v2.13.0 had this bug). The
`own` section of `tests/run.sh` fails if the line comes back. Without it, graft's own scion is absent from
its flake's source, so graft does not call `mkFlake` on itself:

- The docs check is in `dev/flake.nix`, so that knowledge-base-kit is in no consumer's lock. It reads the whole
  repository, so run it from the checkout as a git flake, `nix flake check ./dev`, never as `path:`. Like any git
  flake it sees only what git tracks: `git add` a new page before you check it. It reads knowledge-base-kit's
  checker from the `knowledge-base-kit` input (`flake = false`), at the scion's gitlink, and `tests/run.sh own`
  fails when the two differ. When you move the scion (`nix run .#graft upgrade knowledge-base-kit`, or the weekly
  job), move the input too: set its tag in `dev/flake.nix`, run `nix flake update knowledge-base-kit --flake
  ./dev`, and commit both. To check the docs against an unreleased edit in the scion:
  `nix flake check ./dev --override-input knowledge-base-kit path:$PWD/scions/knowledge-base-kit`.
- `apps.graft` is graft's own app, as `mkFlake` gives a consumer one (`nix run .#graft check`, `sync`,
  `ci …`); `packages.tools` is an empty environment, which `graft check` evaluates.
- `flake.nix` carries the comment `# graft: no self.submodules`, so `graft check` accepts it.

To try a checkout of graft in another repository: `--override-input graft path:/ABSOLUTE/PATH/TO/graft` (its
working tree) or `git+file:///ABSOLUTE/PATH/TO/graft` (what git tracks). `tests/run.sh` uses a copy of the
checkout's working tree as a `path:` input, and a tarball of it as a `tarball+file:` input.

`tests/run.sh` builds local bare provider repositories with tags and `file://` submodule URLs, and covers
mkFlake (discovery, isolation, siblings, collisions, the evaluation errors), the Nix fetch rules that graft
depends on, the seam between mkFlake and `kitsFor` (`kits`), pot against fixture bundles (`pot`), `add`, `sync`,
`check` (one fixture per rule), `outdated`, `changelog`, `upgrade`, `migrate`, `ci` against a local origin and a
stub API, and the workflows. `own` checks graft's own flake: no `inputs.self.submodules`, the command unchanged by
an edit under `pot/`, and the docs check. It takes several minutes and fetches nixpkgs on its first run. Do not
run several copies in parallel.

## Documentation

The docs follow knowledge-base-kit's conventions, grafted in `scions/knowledge-base-kit/`: its
[style policy](https://github.com/danielnaab/knowledge-base-kit/blob/v2.1.0/docs/policies/style.md)
(plain language, ASD-STE100 procedures, one term per concept, Diátaxis document types), and its
[linking](https://github.com/danielnaab/knowledge-base-kit/blob/v2.1.0/docs/policies/linking.md) and
[lifecycle](https://github.com/danielnaab/knowledge-base-kit/blob/v2.1.0/docs/policies/lifecycle.md)
policies. `nix flake check ./dev` runs the kit's checker on them, with the scopes in `knowledge-base.yaml`, which
cover `pot/contracts/` too.

graft's own rules:

- Use the terms that [Concepts](concepts.md) defines.
- Reference pages end with a `## Sources` section that links the code they describe.
- A guide is one user goal: a title that names the goal or symptom, numbered steps with the exact command
  and its expected output, a runnable "Check it worked" block, the two to four real failures, and Sources.
  "Not yet: …" marks a known gap.
- A page whose goal has two actors, such as a provider and a consumer, can have two parts, each with its own
  steps (at most ten) and its own check under a heading that names it ([example](guides/incompatible-change.md)).
- Read a new or changed guide literally, against a fixture: `tests/fixtures.sh NAME OUT` builds one
  (`first-pattern`, `check-fails`, `fix-a-provider`, `gate-merges`, `private`, `ci`, `breaking-change`,
  `upgrade`, `publish`, `worktrees`, `copy-mode`). A
  reader that sees only the page and its links follows each step and says what it expects first. Then a
  reviewer checks its claims against the code.

These links cite the kit by URL at a tag, so that they work on GitHub, where a submodule is a link. When you
upgrade the kit with `nix run .#graft upgrade`, change their tag too.

## Release

1. Add a section at the top of `CHANGELOG.md`: `## vX.Y.Z - YYYY-MM-DD`. A major version needs an
   `### Upgrading` subsection.
2. Change the graft tag in `template/flake.nix`, `README.md` and the docs to the new version.
3. Run `nix flake check`, `nix flake check ./dev` and `tests/run.sh`.
4. Commit.
5. Tag the commit: `git tag -a -m vX.Y.Z vX.Y.Z`.
6. Push the `v2` branch and the tag.
