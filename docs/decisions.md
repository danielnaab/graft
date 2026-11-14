---
status: living
---
# Decisions

Why graft is the shape it is. Each section is a decision that still holds. Change one only with a new
section that says what replaces it and why.

## Scope: Nix, git submodules, and a tag comparison

graft has no configuration file and no lock of its own. A scion's pin is its gitlink, and graft's own pin is
in `flake.lock`. graft provides commands only. Automation, such as the weekly upgrade, is a workflow in the
repository or a central job that runs those commands. A feature that cannot be expressed as Nix, git
submodules and a tag comparison does not belong in graft.

Why: graft v1 grew into a workflow runner as well as a dependency manager, and became hard to change.

Until v2.13.0 this section read "Nix, a copy, a diff and a tag comparison": the copy and the diff went with
copy mode ([below](#pin-with-git-submodules-let-providers-wire-themselves)).

## Pin with git submodules; let providers wire themselves

A scion is a git submodule at a provider's release tag. The gitlink is the pin, and `.gitmodules` names the
source. Nix reads scions through `inputs.self.submodules = true` (or from the working tree, for a `path:`
flake). Each provider's `graft.nix` contributes its tools, apps and checks, so a consumer's `flake.nix` names
no provider.

Why: in the twelve days of v2 copy mode, the measured cost was Nix's export hashing, not the copy. Lock
hashes that differed between Nix 2.24 and 2.34 broke checks in about ten repositories, and every fix to a
provider needed a release before any consumer could try it. Submodules remove the hash pin, and give agents an
edit-in-place loop: a scion is a clone that can be fixed, tested and pushed upstream. graft v1's submodule
failures (empty directories in new clones, worktrees that could not be removed) are answered by Nix fetching
scions itself, a post-checkout hook, `graft check`, and `git worktree remove --force`
([Clones, worktrees, sandboxes and CI](guides/clones-and-worktrees.md)).

Costs accepted:

- A scion is the provider's whole repository, tests included: export rules no longer apply. The marker README
  keeps knowledge-base checkers out of the scions directory.
- Nix fetches nested submodules, so [One level](#one-level) is now a rule, not only a convention.
- Nix 2.27 or later, for `inputs.self.submodules`.
- A git flake sees a scion's uncommitted edits only while another tracked file is modified
  ([Nix behaviours](reference/nix-behaviours.md)).

Reopen when Nix can apply a provider's export rules to a submodule (`exportIgnore` with `submodules`), or a
real consumer cannot use git.

This replaces two decisions of v2.0.0 to v2.12.0:

- **Commit the copy:** scions were committed files, the provider's export copied into the tree, so that a
  fresh clone had them at once. Replaced because the copy needed a hash pin and a drift check, and gave no
  way to fix a provider in place. A git flake now fetches scions itself, so a fresh clone builds anyway.
- **Let Nix fetch and pin:** each provider was a `flake = false` input, pinned by commit and `narHash` in
  `flake.lock`. Replaced because the `narHash` of an export differed between Nix versions and fetch caches.
  graft itself is still an ordinary flake input.

## One level

A consumer grafts every provider that it uses directly, so it sees each pin in its own tree. A provider may
graft another provider for its own use, but Nix fetches every nested submodule when it evaluates a consumer,
and ignores `update = none` and `active = false`. So **a provider must not graft a repository that its
consumers cannot read**: the consumer's evaluation would fail. `graft add` and `graft check` say when a scion
has submodules of its own.

Until v2.13.0, the rule was only that a provider did not export its own scions.

## A major version is a person's decision

graft never crosses a major version by itself. A provider declares a break with a new major version and
says what to do in its CHANGELOG. The weekly upgrade continues to take fixes, and the break waits for a
person.

## graft hands text to agents, and never runs one

`graft migrate` prints a brief, and the upgrade pull request carries the provider's notes. Any agent, or
a person, can take them, and graft does not change when agent tools change.

## Declarations live in the files they describe

graft reads what is already in the repository instead of a list in a separate file: `.gitmodules` for the
scions and their URLs, the marker README for the scions directory and the flake, each provider's `graft.nix`
for its outputs, and marker lines in workflows (`# managed by graft`, `# graft ssh:`). A declaration in the
file cannot drift from it.

## Where graft v1's features went

graft v1 is on the read-only `v1` branch (2025-12 to 2026-10). Earlier experiments are on `v0.2` and
`v0.1`. None of these features return:

| v1 | Now |
|---|---|
| `changes:`, migration commands, atomic upgrade with rollback | The upgrade pull request is the transaction: a branch, checked by the whole `nix flake check`, merged or closed. A major version declares a break, the CHANGELOG says what to do, and `graft migrate` writes a brief. |
| `commands:`, `graft run dep:command` | A provider's `graft.nix`: its tools go on the consumer's `PATH` through `packages.<system>.tools`, its apps and checks into the consumer's flake. A tool that is not tied to a scion can be an ordinary flake input, as graft is. |
| State queries, run logs | The repository's own checks and CI. |
| Grove (a terminal UI for many repositories) | Out of scope. `nix run github:danielnaab/graft/v2.13.0 -- outdated -C <repo>` reads any checkout. |
| Scions as agent sessions | Agent tools outside graft. A scion is a grafted repository at a release tag. |
| Transitive dependencies (`.graft/` in `.graft/`) | One level. |
| Providers that place files anywhere in a consumer | Only graft's own workflows, by their marker. This waits for a second real case. |
