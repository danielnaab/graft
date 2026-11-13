# graft

Apply maintained patterns to your repository, and take each improvement as a pull request you review.

Agents now write much of a team's knowledge base, faster than people can review it. The knowledge stays
trustworthy only if every change follows good rules, and the rules keep improving. A well-made set of
rules is worth reusing, but a copy stops improving. When its author tightens a rule or fixes the checker,
your copy never hears about it.

graft calls such a set of rules, agent instructions and checks a **pattern**. A **provider** repository
publishes it, with version tags. graft commits a copy of the pattern into your repository, pinned to one
version, and keeps the copy true to the pin. That copy is a **scion**. In gardening, a scion is a shoot
grafted onto another plant: it keeps its own identity and grows as part of its new host.

## An example: a knowledge base that agents write

[knowledge-base-kit](https://github.com/danielnaab/knowledge-base-kit) publishes one such pattern:

- **Policies.** A claim that drives a decision cites its sources. Every page has a status, such as draft,
  stable or deprecated.
- **Playbooks for agents.** Plan the change, make the smallest patch, then verify it.
- **A checker.** `kb_linter` fails a page that has no status, or that cites no sources in a folder that
  needs them. `link_validator` fails a link to a missing page or heading.

Graft the kit, then point your own files at the scion:

```
team-kb/
├── flake.nix                     # pins knowledge-base-kit, and adds its checker to `nix flake check`
├── scions/knowledge-base-kit/    # the pattern at one version, committed: never edit it
├── AGENTS.md                     # points agents at the kit's policies and playbooks
├── knowledge-base.yaml           # which folders need sources, which statuses a page may have
└── .forgejo/workflows/           # graft's check, and yours: `nix flake check` on every pull request
```

## While pinned, the pattern governs each change

graft does not run the pattern's checks on your changes. Your CI does, and your forge can require them to
pass before a merge ([Make a pattern's checks gate your merges](docs/guides/gate-merges.md)). With that in
place, each change is a transaction against the pattern, as a database write is against its schema. It lands whole when the checks pass, and not at all when they fail.

Say an agent writes up last week's outage. The provenance policy tells it to cite the incident ticket and
the runbook it relied on. If the page cites neither, or links to a runbook heading that someone renamed,
the checker fails and the page does not merge. The agent fixes the page, and it lands.

graft's part is to keep the rules fixed while you work. Its check fails when the committed scion differs
from its pin, so every clone, worktree and agent session works by the same version of the pattern.

## When the pattern improves, taking it is a transaction too

The provider releases new versions on its own schedule, and says what changed in its CHANGELOG.

- **A minor or patch version** should not break your repository. Each week, a Forgejo job moves your pins
  to the newest version in each major version. It runs `nix flake check` and opens a pull request that
  quotes each provider's CHANGELOG. If a check fails, the pull request says which one. You review the
  scion's diff, and merge.
- **A major version** changes something that you may rely on, so graft holds it back until a person takes
  it. Its CHANGELOG has **Upgrading** steps for your files. `graft migrate` turns them into a brief, with
  the checks that now fail, for a person or an agent to work from.

Suppose version 3 of the kit requires each superseded decision to link to the decision that replaced it.
On a branch, you run `nix run .#graft upgrade knowledge-base-kit@v3.0.0`, then `nix run .#graft migrate`.
An agent works from the brief and adds the links until the checker passes. You open a pull request,
review it and merge.

When the provider fixes a rule or the checker, every repository that grafts the pattern can take the fix
the same way: as one reviewed change, not a rediscovery.

## What graft does, and what it does not

- Nix fetches each provider, and pins its commit and hash in `flake.lock`.
- graft copies each provider's export into `scions/`. Its check fails when a copy differs from its pin.
- graft moves pins to newer versions, and never to a new major version unless you name it.
- In CI, graft runs your `nix flake check` on an upgrade, and opens the pull request.
- graft never runs an agent. `graft migrate` prints a brief that you can give to one.

graft needs Nix with flakes. Its CI workflows are for Forgejo Actions. On GitHub, run the same commands
from workflows of your own. Why graft stops here: [Decisions](docs/decisions.md).

## Compared with copying, submodules, templates and bots

- **Copying files, or git subtree,** records no pin that a tool can check, so nothing notices when a copy
  falls behind or is edited.
- **git submodules** need a fetch in every clone and worktree, and pin a whole repository, not just what
  the provider exports.
- **Templates** (Copier, cookiecutter) generate files that you then own, and `copier update` merges later
  changes into your edits. You never edit a scion, so each version replaces it cleanly. Use a template for
  the pages that you own, and a scion for the rules that they follow.
- **Dependency bots** (Renovate, Dependabot) move pins and quote release notes. graft also puts the
  provider's files in your tree, so the diff shows exactly what you take.

## Quick start

```
nix flake init -t github:danielnaab/graft
```

This writes a `flake.nix` and graft's CI workflows. Edit the inputs:

```nix
{
  description = "What this repo is";

  inputs = {
    graft.url = "github:danielnaab/graft/v2.12.0";
    # A provider: knowledge-base policies, agent playbooks and a checker.
    knowledge-base-kit = { url = "github:danielnaab/knowledge-base-kit/v2.1.0"; flake = false; };
  };

  outputs = inputs: inputs.graft.lib.mkFlake { inherit inputs; };
}
```

Run `nix run .#graft` to lock the inputs and write `scions/`. Commit `flake.nix`, `flake.lock`, `scions/`
and `.forgejo/`. Then point your `AGENTS.md`, checks and CI at what you grafted. Next:
[Graft your first pattern](docs/guides/graft-a-repository.md).

- `nix run .#graft` copies the files that git tracks in your repository into the Nix store, which every
  user of the machine can read. To keep private files out of it, see
  [Keep private files out of the Nix store](docs/guides/flake-in-a-subdirectory.md).
- On GitHub, run `nix run .#graft ci check` and `ci upgrade` from your own workflows, and open the pull
  request yourself ([Run graft in CI and review its upgrade pull requests](docs/guides/ci.md)).

## Publish a pattern

Any repository with semver tags can be a provider. Make it easy to follow:

- Mark what consumers should not get (tests, your own scions) `export-ignore` in `.gitattributes`.
- Tag each release with a semver tag, and give it a `CHANGELOG.md` section. Prefer a lightweight tag. A
  signed or annotated tag works too: on Nix 2.24, a consumer with a `git+` input sees one re-pin on its
  first run ([Nix behaviours](docs/reference/nix-behaviours.md#annotated-tags-and-exportignore)).
- Release a breaking change as a new major version.
- In that version's CHANGELOG section, write **Upgrading** steps that a person or agent can follow without
  knowing your repository's internals. `graft changelog` and `graft migrate` show them first.

See [Publish a pattern](docs/guides/publish.md), and [Ship and take an incompatible change](docs/guides/incompatible-change.md).

## Documentation

| To | Read |
|---|---|
| Learn the terms and how they map to Nix | [Concepts](docs/concepts.md) |
| Graft your first pattern, in a new or existing repository | [Graft your first pattern](docs/guides/graft-a-repository.md) |
| Run a pattern's checks in CI, and require them before a merge | [Make a pattern's checks gate your merges](docs/guides/gate-merges.md) |
| Find and fix what fails `graft check` | [`graft check` fails](docs/guides/check-fails.md) |
| Change a grafted file that does not fit, or has a mistake | [Change a grafted file](docs/guides/change-a-grafted-file.md) |
| Take newer versions outside CI | [Take a newer version by hand](docs/guides/upgrade.md) |
| Release an incompatible change, or take one (a new major version) | [Ship and take an incompatible change](docs/guides/incompatible-change.md) |
| Keep private files out of the Nix store, with a flake in a subdirectory | [Keep private files out of the Nix store](docs/guides/flake-in-a-subdirectory.md) |
| Release a repository for others to graft | [Publish a pattern](docs/guides/publish.md) |
| Ship a tool that consumers build, or files that they copy once | [Ship a tool or seed files with a pattern](docs/guides/ship-tools.md) |
| Graft a provider that only an ssh key can read, and check it in CI | [Graft a private repository](docs/guides/private-providers.md) |
| Check in CI, and review graft's weekly upgrade pull request | [Run graft in CI and review its upgrade pull requests](docs/guides/ci.md), and why it is safe: [CI security](docs/ci-security.md) |
| Look up a command, flag or `mkFlake` argument | [Commands](docs/reference/commands.md) |
| Look up a file graft reads or writes | [Files](docs/reference/files.md) |
| Know which Nix behaviours graft works around | [Nix behaviours](docs/reference/nix-behaviours.md) |
| Know why graft is this small, and where graft v1 went | [Decisions](docs/decisions.md) |
| Change graft itself | [Development](docs/development.md) |

Releases and upgrade steps are in the [CHANGELOG](CHANGELOG.md).
