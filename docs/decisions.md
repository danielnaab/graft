---
status: living
---
# Decisions

Why graft is the shape it is. Each section is a decision that still holds. Change one only with a new
section that says what replaces it and why.

## Scope: Nix, a copy, a diff and a tag comparison

graft has no configuration file, no lock other than `flake.lock`, and no command other than
`nix run .#graft`. graft provides commands only. Automation, such as the weekly upgrade, is a workflow in
the repository or a central job that runs those commands. A feature that cannot be expressed as Nix plus
a copy, a diff and a tag comparison does not belong in graft.

Why: graft v1 grew into a workflow runner as well as a dependency manager, and became hard to change.

## Commit the copy

Scions are committed files, not links or fetched-on-demand directories. A fresh clone or a new worktree
has them at once. Nix is needed only to change or check a pin.

Why not the alternatives:

- **git submodules** need an extra fetch in every clone and worktree, and pin a whole repository, not a
  chosen export.
- **git subtree and plain vendoring** copy files, but record no pin that a tool can check, and apply no
  export rules.
- **Package managers** are for one language's code, and keep the files outside the repository.

## Let Nix fetch and pin

Flake inputs already fetch, pin by commit and hash, apply export rules, and share inputs with
`follows`. graft adds only what Nix does not have: the copy, the check, and moving tags.

There is one binding, Nix. A second binding would need its own lock, which graft v1 had and v2 dropped.
Reopen this when two real consumers cannot run Nix.

## One level

By convention, a provider does not export its own scions. A consumer grafts what it needs directly, so it holds one copy of
each dependency and sees every pin in its own `flake.lock`.

## A major version is a person's decision

graft never crosses a major version by itself. A provider declares a break with a new major version and
says what to do in its CHANGELOG. The weekly upgrade continues to take fixes, and the break waits for a
person.

## graft hands text to agents, and never runs one

`graft migrate` prints a brief, and the upgrade pull request carries the provider's notes. Any agent, or
a person, can take them, and graft does not change when agent tools change.

## Declarations live in the files they describe

graft recognizes marker lines in files (`# managed by graft`, `# graft ssh:`, the scions README's
marker) instead of a list in a separate file. A declaration in the file cannot drift from it.

## Where graft v1's features went

graft v1 is on the read-only `v1` branch (2025-12 to 2026-10). Earlier experiments are on `v0.2` and
`v0.1`. None of these features return:

| v1 | Now |
|---|---|
| `changes:`, migration commands, atomic upgrade with rollback | The upgrade pull request is the transaction: a branch, checked by the whole `nix flake check`, merged or closed. A major version declares a break, the CHANGELOG says what to do, and `graft migrate` writes a brief. |
| `commands:`, `graft run dep:command` | A provider's code is part of its export. Consumers load it from the same input (`import "${inputs.<name>}/…"`). A tool that is not tied to a scion can be an ordinary flake input, as graft is. |
| State queries, run logs | The repository's own checks and CI. |
| Grove (a terminal UI for many repositories) | Out of scope. `nix run github:danielnaab/graft -- outdated -C <repo>` reads any checkout. |
| Scions as agent sessions | Agent tools outside graft. A scion is now a grafted export only. |
| Transitive dependencies (`.graft/` in `.graft/`) | One level. |
| Providers that place files anywhere in a consumer | Only graft's own workflows, by their marker. This waits for a second real case. |
