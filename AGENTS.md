# Agent entrypoint

graft grafts other repositories into a repository as git submodules at release tags, and wires what they ship
into its Nix flake: a flake library and one command. Read [README.md](README.md), then
[Concepts](docs/concepts.md).

## Orientation

- **Configuration:** [knowledge-base.yaml](knowledge-base.yaml)
- **Docs:** [guides](docs/guides/), [reference](docs/reference/), [decisions](docs/decisions.md)
- **Code:** [lib/](lib/) (see [Development](docs/development.md#layout)), tests in [tests/run.sh](tests/run.sh)
- **Grafted:** `scions/knowledge-base-kit`, a submodule: the conventions these docs follow. If it is empty,
  run `git submodule update --init`.

## Rules

1. `scions/knowledge-base-kit` is another repository. Fix a bug in it only as a change to knowledge-base-kit:
   a branch in the scion, its own tests, a pull request there. Take the release with
   `nix run .#graft upgrade knowledge-base-kit`. Never commit the scion away from a release tag:
   `nix run .#graft check` fails.
2. Behaviour and docs change together. A change to a command, argument, message or file format updates its
   reference page in the same commit.
3. Keep graft in [scope](docs/decisions.md#scope-nix-git-submodules-and-a-tag-comparison).
4. Do not change `CHANGELOG.md` sections of released versions.

## Verify

1. `nix flake check '.?submodules=1'`: bash -n and shellcheck, and the docs check. Nix sees an uncommitted
   edit in the scion only while another tracked file is modified.
2. `nix run .#graft check`: the scion is clean and at a release tag.
3. `tests/run.sh` after a change to `lib/` or `template/`. It takes minutes; do not run two at once.
