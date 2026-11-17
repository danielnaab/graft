# Agent entrypoint

graft grafts other repositories into a repository as git submodules at release tags, and wires what they ship
into its Nix flake: a flake library and one command. Read [README.md](README.md), then
[Concepts](docs/concepts.md).

## Orientation

- **Configuration:** [knowledge-base.yaml](knowledge-base.yaml)
- **Docs:** [guides](docs/guides/), [reference](docs/reference/), [decisions](docs/decisions.md)
- **Code:** [lib/](lib/), graft's; [pot/](pot/), pot's, with its contracts in `pot/contracts/` (see
  [Development](docs/development.md#layout)); tests in [tests/run.sh](tests/run.sh) and `pot/tests/`
- **Grafted:** `scions/knowledge-base-kit`, a submodule: the conventions these docs follow. If it is empty,
  run `git submodule update --init --recursive`.

## Rules

1. `scions/knowledge-base-kit` is another repository. Ask before you edit it, and fix a bug in it only as a
   change to knowledge-base-kit: a branch in the scion, its own tests, a pull request there. To check graft's
   docs with the fix: `nix flake check ./dev --override-input knowledge-base-kit path:$PWD/scions/knowledge-base-kit`.
   Take the release with `nix run .#graft upgrade knowledge-base-kit`, and move the `knowledge-base-kit` input
   in `dev/flake.nix` to the same tag ([Development](docs/development.md#test)). Never commit the scion away from a release tag:
   `nix run .#graft check` fails.
2. Behaviour and docs change together. A change to a command, argument, message or file format updates its
   reference page in the same commit.
3. Keep graft in [scope](docs/decisions.md#scope-nix-git-submodules-a-tag-comparison-and-a-sandbox-policy).
4. Do not change `CHANGELOG.md` sections of released versions.

## Verify

1. `nix flake check`: bash -n and shellcheck, and pot's checks (its unit tests, the golden grant test, its
   roots). `flake.nix` must not declare `inputs.self.submodules`: graft is other flakes' input.
   `nix flake check ./dev`: the docs check (knowledge-base-kit from `dev/flake.nix`'s input, not the scion;
   tests/run.sh `own` runs it too). Run it from this checkout, never as `path:`.
2. `nix run .#graft check`: the scion is clean and at a release tag.
3. `tests/run.sh` after a change to `lib/`, `pot/` or `template/`. It takes minutes; do not run two at once.
