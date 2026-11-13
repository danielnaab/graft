---
status: working
---
# Concepts

graft gives a repository a pinned, committed copy of what other repositories publish, and keeps that
copy in step with the pins. This page defines the terms, then shows how they map to Nix. For why graft
works this way, see [Decisions](decisions.md).

## What a provider publishes

- **Provider** and **consumer** are roles, not kinds of repository. A provider is grafted by others. A
  consumer grafts others. Most repositories are both.
- **Export rules:** the paths that the provider's `.gitattributes` marks `export-ignore`.
- **Export:** the provider's files at a version, minus the paths that its export rules exclude. By
  convention a provider excludes its own scions, so a consumer takes the provider's content and never
  its dependencies. graft does not enforce this ([Publish a pattern](guides/publish.md#decide-what-you-export)).
- **Version:** a semver tag, such as `v1.2.3`. A new major version means that the provider changed
  something incompatibly. For `0.y.z`, each `0.y` series counts as a major version.
- **Changelog:** the provider's `CHANGELOG.md`, with one section per version. A major version's section
  has **Upgrading** steps that a person or agent can follow with knowledge of the consumer only.

## What a consumer holds

- **Pin:** a name, the tag requested, and the commit and content hash that the tag resolved to. The name
  is also the scion's folder name. `flake.lock` is the only lock file: graft has none of its own.
- **Scion:** the export that a pin names, committed in the **scions directory** (`scions/<name>/` by
  default). Never edit a scion: graft replaces it.
- **Scions README:** `scions/README.md`, written by graft. It lists each scion and its pinned commit.
  graft's commands read it to find the scions.
- **Drift:** the scions directory differs from what the pins produce. Drift always fails the check.
- **One level:** a consumer grafts every provider that it uses directly, so it holds one copy of each
  dependency.

graft writes the scions directory, its own CI workflow files, and the pins in `flake.nix` and
`flake.lock`. It stages the scions directory with `git add`. It refuses to replace a directory that it
did not write.

## How things change

- **Sync:** `nix run .#graft` writes the scions directory again from the pins.
- **Upgrade:** a pin moves to a newer tag in the same major version. In CI, the consumer's checks run on
  the result in a pull request, and a person merges it. graft never crosses a major version unless a
  person names the new tag.
- **graft's own pin:** a consumer also pins graft, and upgrades it the same way. graft's pin moves first,
  so the rest of the upgrade runs the new version.
- **Migration brief:** after an upgrade, `graft migrate` prints what moved, what the provider's changelog
  says, and which checks fail. A person or an agent does the migration. graft never runs an agent.

## The Nix binding

graft uses Nix to fetch, to pin and to apply the export rules. Each concept maps to one Nix feature.

| Concept | In Nix and git |
|---|---|
| Export | The provider's tree at a tag, minus its export rules. Nix applies the rules for `github:` inputs and for `git+…?exportIgnore=1` inputs. |
| Version | A semver git tag. |
| Pin | A flake input pinned to a tag. `flake.lock` holds its commit and `narHash`. A scion's input also has `flake = false`. |
| Fetched export | `${inputs.<name>}`, a path in the Nix store. Build code reads this, not the scion. |
| Scions directory | `<dir>/`, where `dir` is a `mkFlake` argument, default `scions`. |
| Drift check | `checks.<system>.graft` (`nix flake check`), or `nix run .#graft check`. |
| graft's pin | The flake input named `graft`. |

The fetched export and the scion hold the same bytes when there is no drift. Code that a Nix build runs
loads the fetched export. Tools that run in the working tree, and people, read the scion.

graft copies, compares and moves tags, and does nothing more. The limits and their reasons are in
[Decisions](decisions.md#scope-nix-a-copy-a-diff-and-a-tag-comparison).
