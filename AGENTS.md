# Agent entrypoint

graft copies pinned exports of other repositories into a repository, as a Nix flake library and one
command. Read [README.md](README.md), then [Concepts](docs/concepts.md).

## Orientation

- **Configuration:** [knowledge-base.yaml](knowledge-base.yaml)
- **Docs:** [guides](docs/guides/), [reference](docs/reference/), [decisions](docs/decisions.md)
- **Code:** [lib/](lib/) (see [Development](docs/development.md#layout)), tests in [tests/run.sh](tests/run.sh)
- **Grafted, read-only:** [knowledge-base-kit](scions/knowledge-base-kit/README.md), the conventions
  these docs follow

## Rules

1. Never edit `scions/`. graft replaces it. To change a pin: `nix run .#graft upgrade`.
2. Behaviour and docs change together. A change to a command, argument or file format updates its
   reference page in the same commit.
3. Keep graft in [scope](docs/decisions.md#scope-nix-a-copy-a-diff-and-a-tag-comparison).
4. Do not change `CHANGELOG.md` sections of released versions.

## Verify

1. `nix flake check`: bash -n and shellcheck, the docs check, and the drift check.
2. `tests/run.sh` after a change to `lib/` or `template/`. It takes minutes; do not run two at once.
