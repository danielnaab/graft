# graft

Apply well-defined patterns to your repository, and get each improvement to them as a pull request you review.

Your repository gets better when the patterns it follows do.

A **pattern** is a way of working that someone writes down and keeps improving. It defines how a
repository changes: instructions that tell a person or agent what to do, tools that make the change, and
checks that decide whether the result is valid. Copy one into your repository and it stops improving
there. Upgrading it later is detective work: what changed upstream, and what must change here?

graft applies a pattern as a **scion**: a committed copy of what the pattern's repository exports, pinned
to a release. graft checks that the scion matches its pin. It proposes each newer release as a pull
request that quotes the pattern's CHANGELOG, where the pattern says what to change. It is built on Nix
flakes.

## How it works

1. **Graft a pattern.** Pin the repository that publishes it, its **provider**, at a tag. graft writes
   the export to `scions/<name>/`, and you commit it. Every clone, worktree and forge view then has the
   same files. `nix flake check` fails if `scions/` differs from the pin.
2. **Wire it in.** Point your own files at the scion:

   ```
   your-repo/
   ├── flake.nix                     # pins each provider at a tag
   ├── scions/knowledge-base-kit/    # the committed copy: never edit it
   ├── AGENTS.md                     # "Follow scions/knowledge-base-kit/docs/playbooks/agent-workflow.md"
   ├── justfile                      # import "scions/notebook-kit/tools/kit.just": move, index, check
   └── .forgejo/workflows/ci.yaml    # PYTHONPATH=scions/knowledge-base-kit/src python3 -B -m kb_linter .
   ```

   Every change in your repository then starts from the same agent instructions, runs the same tools and
   passes the same checks. The scion steers your repository only where your files point at it.
3. **The pattern improves.** Its provider releases on its own schedule. A release that asks consumers to
   change something says how in an **Upgrading** section of its CHANGELOG. A breaking release is a new
   major version.
4. **Take the improvement.** Each week, a CI job moves your pins to the newest release in their major version,
   runs your whole `nix flake check`, and opens a pull request that quotes each provider's CHANGELOG. A
   person, or an agent given the brief that `graft migrate` prints, edits your files to match. Your checks,
   many of them from the scion, show when the migration is done. A new major version waits until a person
   names it: `nix run .#graft upgrade NAME@TAG`.

The pin records which release of the pattern your repository follows. The pattern evolves on its own, and
your repository takes each improvement by a reviewed change. Apply one pattern to many repositories, and
they improve together.

Nix fetches and pins. graft copies, checks the copy against the pin, and moves pins. In CI it also runs
your repository's own checks on the upgrade and opens the pull request. graft never runs an agent.

## Every change is a transaction

Under a pattern, each change to your repository is a transaction. It starts from a state that passes the
pattern's checks. A person or agent makes the change with the pattern's instructions and tools. It lands
as one commit or pull request when the checks pass again, and does not land when they fail. Your hooks
and CI enforce this by running the pattern's checks.

A knowledge-base pattern, for example, tells an agent how to record a session, gives it a command that
moves a note and rewrites every link to it, and has checks that fail on a broken link.

There are two kinds of transaction:

| | A change under the pinned pattern | An upgrade to a newer release of the pattern |
|---|---|---|
| What changes | Your content: code, docs, notes, records | The scion, and your files to match it |
| What says how | The pattern's instructions, skills and tools | The provider's **Upgrading** steps |
| Who makes it | A person or agent, with the pattern's tools | A person or agent, from `graft migrate`'s brief |
| What accepts it | The pattern's checks, and your review | The checks at the new pin, and your review of the pull request |
| What graft does | Keeps the pattern pinned and unedited, so every clone and agent session works by the same rules | Moves the pin, runs the checks, and opens the pull request |

graft runs neither kind. It keeps the definition of a valid change pinned, identical everywhere and
versioned, and it makes changing that definition a transaction of its own
([Decisions](docs/decisions.md#where-graft-v1s-features-went)).

## How it compares

- **Copying files** records no version, and nothing notices when a copy changes.
- **git submodules** need a fetch in every clone and worktree, and pin a whole repository, not a chosen
  export.
- **Templates** (Copier, cookiecutter) give you files to own, once. A pattern also defines how those
  files change afterwards, and stays the provider's, replaced at each release. Use a template for the code
  that you own, and a scion for the rules that it follows.
- **Dependency bots** (Renovate, Dependabot) move pins. graft also brings the provider's files into your
  tree and its CHANGELOG into the pull request.

More in [Decisions](docs/decisions.md).

## Quick start

You need Nix with flakes. graft needs nothing else installed.

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

- `nix run .#graft` copies the repository into the Nix store. To keep private files out of it, see
  [A flake in a subdirectory](docs/guides/flake-in-a-subdirectory.md).
- graft's workflows are for Forgejo Actions. On GitHub, run `nix run .#graft ci check` and `ci upgrade`
  from your own workflows, and open the pull request yourself ([CI](docs/guides/ci.md)).

## Publish a pattern

Any repository with semver tags can be grafted. A pattern is well defined when consumers can tell what
they get, when it breaks, and how to keep up:

- Mark what consumers should not get (tests, your own scions) `export-ignore` in `.gitattributes`.
- Release a breaking change as a new major version.
- Write **Upgrading** steps in the CHANGELOG that a person or agent can follow without knowing your
  repository's internals. `graft changelog` and `graft migrate` show them first, and upgrade pull requests
  quote the CHANGELOG sections that hold them.

See [Publish](docs/guides/publish.md).

## The name

In horticulture, a scion is a shoot cut from one plant and joined to the roots of another. It keeps its
own identity and grows as part of the new plant. [Concepts](docs/concepts.md) defines every term that
graft uses and maps each one to Nix.

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
| Know why graft is this small | [Decisions](docs/decisions.md) |
| Change graft itself | [Development](docs/development.md) |

Releases and upgrade steps are in the [CHANGELOG](CHANGELOG.md). This is graft v2. graft v1
(`graft.yaml`, `graft.lock`, `.graft/`) is on the read-only `v1` branch
([what happened to its features](docs/decisions.md#where-graft-v1s-features-went)).
