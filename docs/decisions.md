---
status: living
---
# Decisions

Why graft is the shape it is. Each section is a decision that still holds. Change one only with a new
section that says what replaces it and why.

## Scope: Nix, git submodules, a tag comparison, and a sandbox policy

graft has no configuration file and no lock of its own. A scion's pin is its gitlink, and graft's own pin is
in `flake.lock`. graft provides commands only. Automation, such as the weekly upgrade, is a workflow in the
repository or a central job that runs those commands. A feature that cannot be expressed as Nix, git
submodules, a tag comparison, and the sandbox policy their operations run under does not belong in graft.

Why: graft v1 grew into a workflow runner as well as a dependency manager, and became hard to change.

Since v2.15.0, the sandbox policy is composed at each start by pot, graft's sandbox, from the kits that providers and
consumers declare and the grants that a person's root holds
([Roots grant reach; kits only ask](#roots-grant-reach-kits-only-ask)). graft ships the shape of a kit, the
composition, the bubblewrap backend, agent kits, and a home-manager module for a root. It holds no grants, and its
own configuration is still none: the grants file is the person's, in their root.

Until v2.13.0 this section read "Nix, a copy, a diff and a tag comparison": the copy and the diff went with
copy mode ([below](#pin-with-git-submodules-let-providers-wire-themselves)). Until v2.14.0 it read "Nix, git
submodules, and a tag comparison": the sandbox policy came with
[the sandbox](#enforce-the-boundary-in-a-sandbox-not-in-rules).

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
  ([Nix behaviours](reference/nix-behaviours.md)). `graft try` turns that on and off on purpose, with a marker
  file that `graft check` refuses, so it never merges.

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

Since v2.14.0, graft's sandbox (pot, since v2.15.0) runs the command that a person names, an agent or any other. Since v2.15.0,
graft also ships agent kits, such as `graft.kits.claude-code`: data that says what an agent needs inside a run. A
root chooses to install one, and `pot` runs it only when a person, a recipe or a scheduler names it. graft still
never chooses to run an agent.

## Declarations live in the files they describe

graft reads what is already in the repository instead of a list in a separate file: `.gitmodules` for the
scions and their URLs, the marker README for the scions directory and the flake, each provider's `graft.nix`
for its outputs, and marker lines in workflows (`# managed by graft`, `# graft ssh:`). A declaration in the
file cannot drift from it.

## Enforce the boundary in a sandbox, not in rules

Since v2.14.0, graft runs agents and other commands in a sandbox that enforces what they may read, write and
reach. Rules in `knowledge-base.yaml`, `AGENTS.md` and an agent tool's permission list are context: an agent can
misread them, and a program that the agent starts never reads them. A sandbox stops a write or a connection
whatever the agent read.

How graft does it changed in v2.15.0: [Roots grant reach; kits only ask](#roots-grant-reach-kits-only-ask)
replaces v2.14.0's launcher. These parts of v2.14.0's decision still hold there:

- **A run never writes what decides the sandbox, or what the host runs.** `flake.nix`, each `graft.nix`,
  `.gitmodules`, `.envrc`, `justfile`, `.pre-commit-config.yaml`, the git metadata and agent configuration are
  read-only inside, at any depth.
- **The terminal stays the caller's.** A command that shares a terminal could push input into it (`TIOCSTI`).
  A seccomp filter refuses it, whatever the kernel's `dev.tty.legacy_tiocsti` says.
- **Every value names its source.** v2.14.0's `--explain` became `pot explain` and `pot why`.
- **Never fall back unsandboxed.** `pot host` and a granted `unsandboxed` request are the only ways out, and both
  are explicit.

## Roots grant reach; kits only ask

Since v2.15.0, `pot CMD` runs a command as a run, under a policy that it composes at each start from **kits** and
the **grants** in the person's **root** ([pot](reference/pot.md#the-model)). A repository, a provider and
an agent each ship a kit: what a run sees, its limits, and the reach it requests. A request does nothing until the
person grants it with `pot grant`, at a terminal, into a grants file that no run can see. A **context** (`work`,
`inspect`, `host`) is where a run starts from.

The design's principles:

1. **Anyone may add see and limits; only the root adds reach.** Reach enters a run through a grant that the root
   holds, or a flag that the person typed for one run. A binding carries out a granted capability; it is not a
   third source.
2. **Approval is fixed to what was approved.** A grant records a kit's request set by digest, and for powerful
   items on code that the root does not pin, its source. Any change stops that kit's runs until it is granted
   again. Asking for less needs no approval.
3. **Nothing a run writes widens a later run, or is taken by the host as fact.** A run never writes a root, its
   grants file or `pot`'s state. `pot` decides who a kit is from host facts (the root's store, a verified release
   tag, the repository's origin), never from what a repository says about itself.
4. **Output is untrusted until it crosses as data.** In place, protected paths are defence in depth: the tree
   after a run is like a stranger's pull request.
5. **Delegation only narrows,** except by an allowance that the root granted.
6. **One policy, many backends, guarantees probed, never weaker in silence.** A context may require guarantees.
   Nothing falls back to a weaker policy, backend or no sandbox unless the root or the person says so.
7. **Every value and denial names its source and its fix.**

Why:

- **A repository granted itself.** In v2.14.0, a consumer's `flake.nix` named the scions whose requests applied
  (`sandbox.grant`). The repository that a run works on decided its own reach, and an agent on the host could add
  a grant in a commit. Now only the person's root grants, and a repository's own needs are a kit like any other.
- **A grant by name followed every upgrade.** In v2.14.0, an upgrade pull request that moved a granted scion
  applied its new requests too, shown as one gitlink. Now a grant is fixed to a digest, and a kit that asks for
  more stops until it is granted again.
- **Reach belongs to the command, not the repository.** One `NODE_OPTIONS` or `ANTHROPIC_BASE_URL` from a kit
  would steer the agent that holds the reach. So what configures a program follows that program's command, and a
  repo kit's packages are its only see items that reach every run.
- **The agent's needs were declared in every repository.** v2.14.0's guide made each repository list Claude
  Code's paths, hosts and token, and made `~/.claude` writable, with every project's transcripts. Now the root
  defines the agent kit once, and each run gets its own agent home, with state scoped per account, workspace and
  context.
- **Agents could not see a denial.** Most clients print only `CONNECT tunnel failed, response 403`. `pot` writes
  each denial to a file that the run can read, gives it to Claude through a hook, and shows it on the person's
  tmux pane. Writing it to the run's terminal corrupts a full-screen program.
- **Diagnosis needed a host shell.** `inspect` reads the machine except secrets and changes nothing, so an agent
  can diagnose a failed switch without unrestricted access.

Costs accepted:

- **Experimental, and Linux only, in v2.15.0.** On macOS, `pot` refuses with a message. v2.14.0 is the last
  release with a Seatbelt sandbox. v2.15.0, a minor version, breaks v2.14.0's sandbox, and its CHANGELOG says
  so.
- **Kit tools run on the host in v2.15.0.** A kit's commands are not wrapped and sandboxed on their own until a
  later release. Inside `pot claude`, they run under the run's policy.
- **A person needs a root:** a home-manager module, graft's default root, or one that `pot init` writes.
- **Grants are readable.** Hidden means unmounted and unwritable. A grants file is readable through the root's git
  history wherever that `.git` is readable, `inspect` included. Grants hold no secrets.
- **`pot grant` is a speed bump.** It refuses agents and reads from `/dev/tty`, but an unrestricted host agent can
  edit the grants file or script a terminal. Plain `claude` stays unrestricted by choice.
- **The repository's history is not protected.** In place, a run can commit, and can delete refs. Push what you
  need to keep. New git metadata, gitlinks and changed host-run files are reported after the run, not prevented.
- **A root repository cannot be changed in a run.** In v2.15.0, `work` refuses there; use plain `claude` or a
  person, and review before the switch. Clone mode, which returns root changes as a branch, comes later.
- **The `nix` capability depends on experimental Nix features,** and shows the run the whole store.
- **Linux needs unprivileged user namespaces.** Docker's default seccomp profile blocks them.

Reopen when unprivileged user namespaces are no longer available on the machines that run graft, or when a kit's
real needs cannot be expressed as requests that a root grants.

Not yet: clone mode (`dispatch`) and `pot review`; nesting and wrapped kit commands; generated `kits.just`; the
Seatbelt backend; brokers for `git.push` and proxy credentials; widening delegation through a supervisor; a VM
backend for code that no one has reviewed.

## Where graft v1's features went

graft v1 is on the read-only `v1` branch (2025-12 to 2026-10). Earlier experiments are on `v0.2` and
`v0.1`. None of these features return:

| v1 | Now |
|---|---|
| `changes:`, migration commands, atomic upgrade with rollback | The upgrade pull request is the transaction: a branch, checked by the whole `nix flake check`, merged or closed. A major version declares a break, the CHANGELOG says what to do, and `graft migrate` writes a brief. |
| `commands:`, `graft run dep:command` | A provider's `graft.nix`: its tools go on the consumer's `PATH` through `packages.<system>.tools`, its apps and checks into the consumer's flake. A tool that is not tied to a scion can be an ordinary flake input, as graft is. |
| State queries, run logs | The repository's own checks and CI. |
| Grove (a terminal UI for many repositories) | Out of scope. `nix run github:danielnaab/graft/v2.14.0 -- outdated -C <repo>` reads any checkout. |
| Scions as agent sessions | Agent tools outside graft. A scion is a grafted repository at a release tag. |
| Transitive dependencies (`.graft/` in `.graft/`) | One level. |
| Providers that place files anywhere in a consumer | Only graft's own workflows, by their marker. This waits for a second real case. |
