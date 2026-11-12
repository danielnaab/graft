# graft

graft copies what other repositories publish into your repository, at a version that you pin. It checks
that copy, and upgrades it when they release.

You commit the copy. Every clone, worktree, agent and forge web view then reads the same files, with
nothing to fetch or set up. The repository that publishes decides what it exports. Your repository
decides which version it takes, and takes upgrades as reviewed pull requests.

graft is a Nix flake library and one command, `graft`. Nix fetches and pins. graft copies, checks and
upgrades.

> This is graft v2. It does not read `graft.yaml`, `graft.lock` or `.graft/`. Those belong to graft v1,
> on the read-only `v1` branch ([decisions](docs/decisions.md#where-graft-v1s-features-went)).

## The metaphor

In horticulture, a **scion** is a cut shoot of one plant, joined to the roots of another. The scion
keeps its own identity and grows as part of the new plant.

In graft, a **provider** repository publishes an **export**: its files at a version tag, minus the paths
that it excludes. Your repository, the **consumer**, joins a copy of that export to its own tree, as a
scion. The scion is real files in your repository, not a link. Your repository keeps its own history and
its own roots. To graft is to make this join, and the `graft` command makes it and keeps it.

## How it works

1. You declare each provider as a flake input with `flake = false`, pinned to a tag.
2. Nix fetches the export and records its commit and hash in `flake.lock`.
3. `nix run .#graft` writes each export to `scions/<name>/`. You commit `scions/`.
4. `nix flake check` fails if `scions/` differs from what `flake.lock` pins.
5. `nix run .#graft upgrade` moves pins to newer tags in the same major version. A weekly CI job can
   do this and open a pull request.

## Quick start

```
nix flake init -t github:danielnaab/graft
```

This writes a `flake.nix` and graft's CI workflows. Edit the inputs:

```nix
{
  description = "What this repo is";

  inputs = {
    graft.url = "github:danielnaab/graft/v2.11.0";
    knowledge-base-kit = { url = "github:danielnaab/knowledge-base-kit/v2.1.0"; flake = false; };
  };

  outputs = inputs: inputs.graft.lib.mkFlake { inherit inputs; };
}
```

Then run `nix run .#graft`, and commit `flake.nix`, `flake.lock`, `scions/` and `.forgejo/`. Next:
[Graft a repository](docs/guides/graft-a-repository.md).

## Documentation

Start with [Concepts](docs/concepts.md). It defines every term and shows how graft maps to Nix.

| To | Read |
|---|---|
| Graft a repository, sync and check | [Graft a repository](docs/guides/graft-a-repository.md) |
| Take newer versions, and cross a major version | [Upgrade](docs/guides/upgrade.md) |
| Keep private content out of the Nix store | [A flake in a subdirectory](docs/guides/flake-in-a-subdirectory.md) |
| Publish a repository for others to graft | [Publish](docs/guides/publish.md) |
| Graft a private repository | [Private providers](docs/guides/private-providers.md) |
| Check and upgrade in CI | [CI](docs/guides/ci.md), and why it is safe: [CI security](docs/ci-security.md) |
| Look up a command, flag or `mkFlake` argument | [Commands](docs/reference/commands.md) |
| Look up a file graft reads or writes | [Files](docs/reference/files.md) |
| Know which Nix behaviours graft works around | [Nix behaviours](docs/reference/nix-behaviours.md) |
| Know why graft is this small | [Decisions](docs/decisions.md) |
| Change graft itself | [Development](docs/development.md) |

Releases and upgrade steps are in the [CHANGELOG](CHANGELOG.md).
