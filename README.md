# graft

Apply well-defined patterns to your repository, and get each improvement to them as a pull request you review.

Say your team keeps its knowledge in Markdown, and agents now write much of it. You have worked out how to
keep it trustworthy. A page that makes claims cites where they came from. A page says whether it is a draft
or settled. No link points at a heading that someone renamed. An `AGENTS.md` teaches agents these rules,
and a checker rejects a page that breaks them.

Other teams want the same, so you copy it all into their repositories. A month later you have tightened a
rule and fixed a bug in the checker, and their copies have neither. Catching them up means working out
what changed, and what each copy must change in return.

graft treats that bundle of rules, agent instructions and checks as a **pattern**. A **provider**
repository publishes the pattern with version tags. Your repository commits a copy pinned to one version,
under `scions/`. That copy is a **scion**, after the shoot that a gardener grafts onto another plant.

graft needs Nix with flakes. Its upgrade pull requests come from Forgejo Actions workflows. On GitHub, you
run the same commands from your own workflows.

## A pattern defines how your repository changes

Point your own files at the scion:

```
team-kb/
├── flake.nix                     # pins knowledge-base-kit at a version
├── scions/knowledge-base-kit/    # the pattern, committed: never edit it
├── AGENTS.md                     # "Follow scions/knowledge-base-kit/docs/playbooks/agent-workflow.md"
├── knowledge-base.yaml           # which folders need sources, which statuses a page may have
└── .forgejo/workflows/ci.yaml    # runs the kit's checker: kb_linter and link_validator
```

From then on, each change to the knowledge base is a transaction against the pattern, the way a database
write must satisfy its schema. The change follows the pattern's instructions, and merges only when its
checks pass. Say an agent writes up last week's outage. The playbook tells it to cite the incident ticket
and the runbook that it relied on. If the page has no `## Sources` section, or links to a runbook heading
that no longer exists, the checker fails and the page does not merge.

graft does not run these checks. Your hooks and CI do. graft keeps the pattern that they run the same in
every clone, worktree and agent session: `nix flake check` fails if anyone edits the scion.

## Upgrading is a change too

The provider releases new versions on its own schedule. Taking one is a migration, the second kind of
transaction.

Each week, a CI job moves your pin to the newest version in its major version. It runs your
`nix flake check` and opens a pull request that quotes the provider's CHANGELOG. When a version needs
your files to change, its CHANGELOG says how, in an **Upgrading** section. A person, or an agent given
the brief that `graft migrate` prints, makes those changes until the checks pass at the new pin. Then
you merge.

A new major version waits until a person takes it. Suppose version 3 of the pattern requires each
superseded decision to link to the decision that replaced it, so that no reader, and no agent, acts on a
dead decision. You run `nix run .#graft upgrade knowledge-base-kit@v3.0.0`. The Upgrading section says how
to find the decisions to fix, and the new checker fails on each one. A person, or an agent working from
the brief, adds the links until the checks pass. Then you merge.

Fix a rule or the checker once, in the provider, and every knowledge base that follows the pattern gets
the fix as a pull request that it reviews.

## What graft does

Nix fetches and pins each provider. graft copies the scions, fails the check when they differ from the
pins, and moves pins. In CI, it also runs your checks on an upgrade and opens the pull request. graft never
runs an agent. Why it stops there: [Decisions](docs/decisions.md).

## Compared with copying, submodules, templates and bots

- **Copying files** records no version, and nothing notices when a copy changes.
- **git submodules** need a fetch in every clone and worktree, and pin a whole repository, not just the
  files you need.
- **Templates** (Copier, cookiecutter) give you files to own, once. Use a template for the pages that you
  own, and a scion for the rules that they follow.
- **Dependency bots** (Renovate, Dependabot) move pins. graft also puts the provider's files in your tree,
  and its CHANGELOG in the pull request.

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
    # A provider: knowledge-base conventions, agent playbooks and a checker.
    knowledge-base-kit = { url = "github:danielnaab/knowledge-base-kit/v2.1.0"; flake = false; };
  };

  outputs = inputs: inputs.graft.lib.mkFlake { inherit inputs; };
}
```

Run `nix run .#graft` to lock the inputs and write `scions/`. Commit `flake.nix`, `flake.lock`, `scions/`
and `.forgejo/`. Then point your `AGENTS.md`, justfile or CI at what you grafted. Next:
[Graft a repository](docs/guides/graft-a-repository.md).

- `nix run .#graft` copies the files that git tracks in your repository into the Nix store, which every
  user of the machine can read. To keep private files out of it, see
  [A flake in a subdirectory](docs/guides/flake-in-a-subdirectory.md).
- On GitHub, run `nix run .#graft ci check` and `ci upgrade` from your own workflows, and open the pull
  request yourself ([CI](docs/guides/ci.md)).

## Publish a pattern

Any repository with semver tags can be a provider. A pattern is well defined when consumers can tell what
they get, when it breaks, and how to keep up:

- Mark what consumers should not get (tests, your own scions) `export-ignore` in `.gitattributes`.
- Release a breaking change as a new major version.
- Write **Upgrading** steps in the CHANGELOG that a person or agent can follow without knowing your
  repository's internals. `graft changelog` and `graft migrate` show them first, and upgrade pull requests
  quote the CHANGELOG sections that hold them.

See [Publish](docs/guides/publish.md).

## Documentation

| To | Read |
|---|---|
| Learn the terms and how they map to Nix | [Concepts](docs/concepts.md) |
| Graft a repository, sync and check | [Graft a repository](docs/guides/graft-a-repository.md) |
| Take newer versions, and cross a major version | [Upgrade](docs/guides/upgrade.md) |
| Keep private content out of the Nix store | [A flake in a subdirectory](docs/guides/flake-in-a-subdirectory.md) |
| Publish a repository for others to graft | [Publish](docs/guides/publish.md) |
| Graft a private repository | [Private providers](docs/guides/private-providers.md) |
| Check and upgrade in CI | [CI](docs/guides/ci.md), and why it is safe: [CI security](docs/ci-security.md) |
| Look up a command, flag or `mkFlake` argument | [Commands](docs/reference/commands.md) |
| Look up a file graft reads or writes | [Files](docs/reference/files.md) |
| Know which Nix behaviours graft works around | [Nix behaviours](docs/reference/nix-behaviours.md) |
| Know why graft is this small, and where graft v1 went | [Decisions](docs/decisions.md) |
| Change graft itself | [Development](docs/development.md) |

Releases and upgrade steps are in the [CHANGELOG](CHANGELOG.md).
