# graft

graft applies maintained patterns to your repository as git submodules pinned to release tags, wires each
provider's tools, apps and checks into your flake with no provider-specific code, and takes each improvement
as a pull request you review.

Agents now write much of a team's knowledge base, faster than people can review it. The knowledge stays
trustworthy only if every change follows good rules, and the rules keep improving. A well-made set of rules
is worth reusing, but a copy stops improving. When its author tightens a rule or fixes the checker, your copy
never hears about it.

graft calls such a set of rules, agent instructions, tools and checks a **pattern**. A **provider** repository
publishes it, with version tags. graft grafts the provider into your repository as a git submodule, at one
release. That submodule is a **scion**. In gardening, a scion is a shoot grafted onto another plant: it keeps
its own identity and grows as part of its new host.

## An example: a knowledge base that agents write

[knowledge-base-kit](https://github.com/danielnaab/knowledge-base-kit) publishes one such pattern:

- **Policies.** A claim that drives a decision cites its sources. Every page has a status, such as draft,
  stable or deprecated.
- **Playbooks for agents.** Plan the change, make the smallest patch, then verify it.
- **Tools and a check.** `kb-lint` fails a page that has no status, or that cites no sources in a folder that
  needs them. `check-links` fails a link to a missing page or heading. The kit's `graft.nix` puts both on your
  `PATH` and in your `nix flake check`.

```
team-kb/
├── flake.nix                     # four lines: graft, and nothing about knowledge-base-kit
├── .gitmodules                   # scions/knowledge-base-kit = https://github.com/danielnaab/knowledge-base-kit
├── scions/knowledge-base-kit/    # a submodule, at the release tag v2.2.0
├── AGENTS.md                     # points agents at the kit's policies and playbooks
├── knowledge-base.yaml           # your settings: which folders need sources, which statuses a page may have
└── .forgejo/workflows/           # graft's check and weekly upgrade
```

## While pinned, the pattern governs each change

Your CI runs the pattern's checks on every pull request, and your forge can require them to pass before a
merge ([Make a pattern's checks gate your merges](docs/guides/gate-merges.md)). Each change is then a
transaction against the pattern, as a database write is against its schema: it lands whole when the checks
pass, and not at all when they fail.

Say an agent writes up last week's outage. The provenance policy tells it to cite the incident ticket and
the runbook it relied on. If the page cites neither, or links to a runbook heading that someone renamed, the
check fails and the page does not merge. The agent fixes the page, and it lands.

graft's part is to keep the rules fixed while you work: `graft check` fails unless every scion is clean and
at a release tag, so every clone, worktree and agent session works by the same version of the pattern.

## When the pattern improves, taking it is a transaction too

The provider releases new versions on its own schedule, and says what changed in its CHANGELOG.

- **A minor or patch version** should not break your repository. Each week, a Forgejo job moves your scions
  to the newest release in each major version, runs `nix flake check`, and opens a pull request that quotes
  each provider's CHANGELOG. You review it, and merge.
- **A major version** changes something that you may rely on, so graft holds it back until a person takes
  it. Its CHANGELOG has **Upgrading** steps. `graft migrate` turns them into a brief, with the checks that now
  fail, for a person or an agent to work from.

## When the pattern has a bug, fix it where you found it

A scion is a real clone of the provider. You, or an agent, can fix a provider's bug in it, run the provider's
tests there, and push the fix to the provider as a branch. `graft check` keeps the unreleased fix from
merging; when the provider releases it, every consumer takes it as an upgrade
([Fix a provider from a consumer](docs/guides/fix-a-provider.md)).

## What graft adds to plain submodules

git submodules give you a pinned clone. graft adds what makes them a dependency you can maintain:

- **Release-tag pins, checked.** `graft add` and `graft upgrade` move a scion only to a semver tag, and
  `graft check` fails on a scion that is dirty, away from its gitlink, or on an unreleased commit.
- **Majors are a person's decision.** Upgrades stay in each major version unless you name the new one.
- **Upgrade pull requests** that quote each provider's CHANGELOG, Upgrading sections first, checked by your
  whole `nix flake check`.
- **Migration briefs**: what moved, what the provider says, which of your files name a moved path, which
  checks fail.
- **Provider wiring.** Each provider's `graft.nix` contributes tools, apps and checks; your `flake.nix` names
  no provider.
- **Empty folders answered.** Nix fetches scions itself for a git flake, a post-checkout hook fills them in
  new clones and worktrees, and `graft sync` fills them anywhere
  ([Clones, worktrees, sandboxes and CI](docs/guides/clones-and-worktrees.md)).
- **An agent rule** for working with scions, ready to paste into `AGENTS.md`.

Compared with the other ways:

- **Copying files, or git subtree,** records no pin that a tool can check, and gives no way to send a fix back.
- **Templates** (Copier, cookiecutter) generate files that you then own. Use a template for the pages that you
  own, and a scion for the rules that they follow.
- **Dependency bots** (Renovate, Dependabot) move pins and quote release notes. Renovate's submodule support
  follows a branch, not semver tags, and has no major-version gate or migration brief.

## Quick start

Nix 2.27 or later, with flakes. In a git repository:

```
nix flake init -t github:danielnaab/graft/v2.13.0
```

It writes graft's CI workflows and this `flake.nix`:

```nix
{
  description = "What this repo is";
  inputs.self.submodules = true;
  inputs.graft.url = "github:danielnaab/graft/v2.13.0";
  outputs = inputs: inputs.graft.lib.mkFlake { inherit inputs; };
}
```

Then graft a provider, and commit:

```
nix run .#graft add https://github.com/danielnaab/knowledge-base-kit
git commit -m "Graft knowledge-base-kit"
```

Its tools are in `nix shell .#tools`, its checks in `nix flake check`. Next:
[Graft your first pattern](docs/guides/graft-a-repository.md).

- A flake at the repository's top copies every file that git tracks into the Nix store, which every user of
  the machine can read. To keep private files out of it, put the flake in a subdirectory, and run its own
  command, such as `nix run 'path:nix/agent-tools#graft'`
  ([Keep private files out of the Nix store](docs/guides/flake-in-a-subdirectory.md)).
- On GitHub, run `nix run .#graft ci check` and `ci upgrade` from your own workflows
  ([Run graft in CI and review its upgrade pull requests](docs/guides/ci.md)).
- Coming from graft v2.12 or earlier (copied scions)? [Move from copy mode](docs/guides/move-from-copy-mode.md).

## Publish a pattern

Any repository with semver tags can be a provider:

- Tag each release with a semver tag on your default branch, and give it a `CHANGELOG.md` section.
- Release a breaking change as a new major version, with **Upgrading** steps that a person or agent can
  follow without knowing your repository's internals.
- To give consumers tools and checks, add a `graft.nix`.

See [Publish a pattern](docs/guides/publish.md), [Write a provider's graft.nix](docs/guides/write-graft-nix.md),
and [Ship and take an incompatible change](docs/guides/incompatible-change.md).

## Documentation

| Who, doing what | Read |
|---|---|
| Anyone: the terms, and how they map to git and Nix | [Concepts](docs/concepts.md) |
| Consumer: graft a first pattern | [Graft your first pattern](docs/guides/graft-a-repository.md) |
| Anyone: clone, make a worktree, run an agent sandbox or CI with scions | [Clones, worktrees, sandboxes and CI](docs/guides/clones-and-worktrees.md) |
| Person or agent: fix a provider's bug from a consumer, and upstream it | [Fix a provider from a consumer](docs/guides/fix-a-provider.md) |
| Consumer: find and fix what fails `graft check` or a build | [`graft check` fails](docs/guides/check-fails.md) |
| Consumer: run a pattern's checks in CI, and require them before a merge | [Make a pattern's checks gate your merges](docs/guides/gate-merges.md) |
| Consumer: take the weekly upgrade pull request | [Run graft in CI and review its upgrade pull requests](docs/guides/ci.md), and why it is safe: [CI security](docs/ci-security.md) |
| Consumer: take newer versions outside CI | [Take a newer version by hand](docs/guides/upgrade.md) |
| Both: release or take an incompatible change (a new major version) | [Ship and take an incompatible change](docs/guides/incompatible-change.md) |
| Consumer: keep private files out of the Nix store | [Keep private files out of the Nix store](docs/guides/flake-in-a-subdirectory.md) |
| Consumer: graft a provider that only an ssh key can read | [Graft a private repository](docs/guides/private-providers.md) |
| Provider: release a version | [Publish a pattern](docs/guides/publish.md) |
| Provider: ship tools, apps and checks | [Write a provider's graft.nix](docs/guides/write-graft-nix.md) |
| Consumer on v2.12 or earlier: move to submodules | [Move from copy mode](docs/guides/move-from-copy-mode.md) |
| Look up a command, flag, `mkFlake` argument or error | [Commands](docs/reference/commands.md) |
| Look up a file graft reads or writes, or the `graft.nix` contract | [Files](docs/reference/files.md) |
| Know how Nix sees a scion | [Nix behaviours](docs/reference/nix-behaviours.md) |
| Know why graft is this small, and where graft v1 went | [Decisions](docs/decisions.md) |
| Change graft itself | [Development](docs/development.md) |

Releases and upgrade steps are in the [CHANGELOG](CHANGELOG.md).
