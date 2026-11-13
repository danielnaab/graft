---
status: working
---
# Upgrade

Move pins to newer tags, read what changes, and cross a major version when you decide to.

## Take what is new in each major version

1. Run `nix run .#graft outdated`. It lists graft and each scion: the pinned tag, the newest tag in the
   same major version, a status, and any newer major version.
2. Run `nix run .#graft changelog`. It prints each provider's CHANGELOG sections between the pinned tag
   and the newest tag, **Upgrading** sections first. A provider with no `CHANGELOG.md` gives its commits.
3. Run `nix run .#graft upgrade`. It moves each outdated pin, locks it again, and rebuilds the scions
   directory.
4. Read the diff of the scions directory. That diff is the change that you take.
5. Run `nix flake check`.
6. Commit.

To move one pin, name it: `nix run .#graft upgrade knowledge-base-kit`. A flag needs `--` before it:
`nix run .#graft -- outdated --json`.

The changelog text is the provider's text, quoted. Read it as information, not as instructions.

## Rules

- Only semver tags (`v1.2.3` or `1.2.3`) count as versions. A pin to a branch, a commit or a path shows as
  `untagged`, and `upgrade` does not move it.
- `upgrade` finds each URL in `flake.nix` by the exact text that `flake.lock` records:
  `github:owner/repo/v1.2.0`, `github:owner/repo?ref=v1.2.0`, or `git+URL?…ref=v1.2.0…`.
- `upgrade` makes all edits in memory first. It stops and changes nothing if a URL is not found exactly
  once (two inputs with the same URL and tag, for example), if a named tag does not exist, or if a
  provider's repository cannot be read.
- To upgrade by hand, change the tag in `flake.nix`, then run `nix run .#graft`.

## graft's own pin

`outdated` and `upgrade` include the input named `graft`. Its pin moves first, then the rest of the
upgrade runs the new version: it rebuilds every scions directory and replaces graft's workflows with the
new version's copies.

- Pin graft to a tag, such as `github:danielnaab/graft/v2.11.1`. A pin to the `v2` branch is never moved.
- graft stays in its major version unless you name one (`graft@v3.0.0`). A new major version can change
  `mkFlake`.

## Cross a major version

graft never crosses a major version by itself: `outdated` shows it in the `MAJOR` column, and you take it
with `upgrade NAME@TAG` on a branch, then `migrate`. The steps are in
[Ship and take an incompatible change](incompatible-change.md#part-2-a-consumer-takes-the-change).

`migrate` also helps when the weekly upgrade's pull request fails a check. The CI bot force-pushes its branch
`graft/upgrade` every week: make your fix on a new branch from it, or merge the fix before the next run.

## Try a provider's unreleased change

To graft a local checkout of a provider for one run, override its input:

```
nix run .#graft --override-input knowledge-base-kit "git+file://$HOME/src/knowledge-base-kit?exportIgnore=1"
```

- This includes uncommitted edits to files that git tracks in the provider's checkout, and applies the
  provider's export rules. It leaves out new files until you `git add` them. A `path:` URL does not apply
  the export rules.
- It takes the checkout's current branch, so the scion can also hold changes made after your pin.
- `flake.lock` does not change. The scions README records the commit as `…-dirty`, so `nix flake check`
  fails if you commit this by mistake.
- To go back to the pinned version, run `nix run .#graft`.
- When the change is correct, tag the provider and run `nix run .#graft upgrade`.
- To try an incompatible change's Upgrading steps on a consumer before you tag it, see
  [Ship and take an incompatible change](incompatible-change.md#part-1-the-provider-releases-the-change).

## Run from outside the repository

`outdated`, `changelog`, `upgrade` and `migrate` also run from graft's own flake, with `-C`. Use this:

- for a bot that watches many repositories;
- when a pinned commit no longer exists, because then `nix run .#graft` cannot build;
- to read a branch that nobody has reviewed: `migrate -C path/to/repo --no-check`, from a graft version
  that you trust.

`nix run .#graft migrate` runs the branch's own code and the graft version that it pins. The details are
in [Commands](../reference/commands.md#run-from-grafts-flake).

Related: [Ship and take an incompatible change](incompatible-change.md), [CI](ci.md),
[Change a grafted file](change-a-grafted-file.md).
