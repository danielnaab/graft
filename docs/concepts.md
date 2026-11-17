---
status: working
---
# Concepts

graft grafts other repositories into yours as git submodules, pinned to release tags, and wires what they
ship into your Nix flake. This page defines the terms, then shows how they map to git and Nix. For why graft
works this way, see [Decisions](decisions.md).

## What a provider publishes

- **Provider** and **consumer** are roles, not kinds of repository. A provider is grafted by others. A
  consumer grafts others. Most repositories are both.
- **Version:** a semver tag, such as `v1.2.3` or `1.2.3`. A new major version means that the provider changed
  something incompatibly. For `0.y.z`, each `0.y` series counts as a major version. Other tags, such as
  `v1.3.0-rc.1`, are not versions.
- **Changelog:** the provider's `CHANGELOG.md`, with one section per version. A major version's section has
  **Upgrading** steps that a person or agent can follow with knowledge of the consumer only.
- **`graft.nix`:** an optional file at the top of the provider's repository. It tells a consumer's flake
  which tools, apps, checks and packages the provider contributes, and which other providers it needs
  ([Write a provider's graft.nix](guides/write-graft-nix.md)). A provider without one (rules and docs only)
  is still a provider.

## What a consumer holds

- **Scion:** a git submodule in the scions directory, at `<dir>/<name>`. It holds the provider's whole
  repository, checked out at a release tag. The folder name is the provider's name.
- **Scions directory:** one folder, `scions/` by default, beside the consumer's `flake.nix`. It holds the
  submodules and graft's **marker README**, `<dir>/README.md`. graft writes that README, and finds the
  scions directory and the flake by it. Its text is the same in every repository.
- **Pin:** the **gitlink**, the commit that the consumer's tree records for the submodule. `.gitmodules`
  holds the provider's URL. There is nothing else: no lock file of graft's own, and no provider entry in
  `flake.lock`.
- **Pinned version:** the highest semver tag upstream that points at the gitlink. graft works it out each
  time, and never records it.
- **At a release:** a scion is at a release when its gitlink is the commit of a version tag. `graft check`
  fails on a scion that is not, so an unreleased fix cannot merge by accident.
- **graft's own pin:** a consumer also pins graft, as an ordinary flake input, `inputs.graft`, in
  `flake.lock`. It moves the same way as the scions.
- **One level:** a consumer grafts every provider that it uses directly. A provider may graft others, but Nix
  then fetches those too when it evaluates the consumer, so a provider must not graft a repository that its
  consumers cannot read.

## How things change

- **Add:** `graft add URL` makes a new scion at the newest release, and writes the marker README.
- **Sync:** `graft sync` fills empty scions (`git submodule update --init --recursive`), writes the marker README, and
  refreshes graft's workflows. It never moves a pin.
- **Check:** `graft check` fails unless every scion is registered, filled, clean, checked out at its gitlink,
  and at a release.
- **Upgrade:** a pin moves to a newer tag; within its major version unless a person names the tag. In CI,
  the consumer's checks run on the result in a pull request, and a person merges it. graft's own pin moves
  first, so the rest of the upgrade runs the new version.
- **Migration brief:** after an upgrade, `graft migrate` prints what moved, what the provider's changelog
  says, which of your files name a moved path, and which checks fail. A person or an agent does the
  migration. graft never chooses to run an agent.
- **Fix in place:** a scion is a real clone. You or an agent can fix a provider bug in it, push the fix to
  the provider, and take it as a release ([Fix a provider from a consumer](guides/fix-a-provider.md)).

## What runs in a sandbox

pot, graft's sandbox, runs one command as a **run**, under one policy. Its terms are defined in
[pot](reference/pot.md#the-model); these are the ones that touch a consumer's or a provider's files:

- **Kit:** what a run needs, declared as `kit = { … };`: packages and fixed values that it sees, paths that it
  protects, and requests. A provider's kit is in its `graft.nix` (a **scion kit**); a consumer's own is in mkFlake's
  `perSystem` (its **repo kit**). mkFlake hands them, unread, to pot's library, which writes them as
  `packages.<system>.pot-kits` ([pot's contracts](../pot/contracts/README.md)). pot finds that flake from
  `.gitmodules`: a repository with no submodules has no repo kit from its flake.
- **Request:** a kit's ask for reach: a host path, a network host, a variable from the caller, persistent state,
  or a **capability** such as `nix` or `git.fetch`. A request does nothing on its own.
- **Root and grant:** a person's **root**, such as a home-manager configuration, holds their **grants**. `pot grant`
  approves a kit's request set, by the kit's identity, in the root's grants file. A repository cannot grant its
  own requests.
- **Context:** where a run starts from: `work` writes the repository in place, `inspect` reads the machine and
  changes nothing, `host` has no sandbox.
- **Protected path:** a path in the repository that a run sees read-only, such as `flake.nix`, each `graft.nix`
  and the git metadata: what decides the sandbox, or what the host runs.

graft has no command of its own for it, and never starts a run by itself: a person, a recipe or a scheduler runs
`pot`.

## The git and Nix binding

| Concept | In git and Nix |
|---|---|
| Scion | A submodule: an entry in `.gitmodules` (path and URL) and a gitlink (mode `160000`) in the tree. |
| Pin | The gitlink. `git ls-files -s scions/NAME` prints it. |
| Version | A semver git tag on the provider. graft reads tags with `git ls-remote --tags URL`. |
| Scions in a build | A git flake with `inputs.self.submodules = true;` has each submodule in its source (`${inputs.self}/scions/NAME`). Nix fetches **every** submodule in the repository then, not only scions, so each must be readable. A `path:` flake copies the working tree, submodules included. |
| A scion in the Nix store | mkFlake copies each scion alone (`scion-NAME`), so a tool's store path does not change when an unrelated file of the consumer changes. |
| Provider outputs | Each scion's `graft.nix`, called once per system. Its `tools` become `packages.<system>.tools`, one environment; its checks become `checks.<system>.<scion>-<name>`. |
| Scions directory | `<dir>/`, where `dir` is a `mkFlake` argument, default `scions`, beside `flake.nix`. |
| graft's pin | The flake input named `graft`. |
| Kit | A provider's `graft.nix` and `perSystem` can return `kit`. mkFlake passes them to `graft.lib.pot.kitsFor`, which writes the workspace's kits and tools as `packages.<system>.pot-kits`; `pot` builds it with `path:`. |

Nix and git see a scion differently, and one difference matters every day: a git flake's build reads a scion
at its **committed** gitlink while the rest of your tree is clean, and from your working tree once any tracked
file is modified ([Nix behaviours](reference/nix-behaviours.md)). So test a provider fix with the provider's
own tests, inside the scion; build your repository with it under [`graft try`](reference/commands.md#try),
which makes every build read the working tree until `graft try --stop`; and let `graft check` keep
unreleased scions (and try's marker) from merging.

graft adds, checks, moves tags, wires outputs and kits, starts a sandboxed run when a person asks, and does
nothing more. The limits and their reasons are in [Decisions](decisions.md#scope-nix-git-submodules-a-tag-comparison-and-a-sandbox-policy).
