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

Since v2.14.0, the [sandbox launcher](#enforce-the-boundary-in-a-sandbox-not-in-rules) runs the command that a
person names, an agent or any other. graft still never chooses to run an agent, and names none.

## Declarations live in the files they describe

graft reads what is already in the repository instead of a list in a separate file: `.gitmodules` for the
scions and their URLs, the marker README for the scions directory and the flake, each provider's `graft.nix`
for its outputs, and marker lines in workflows (`# managed by graft`, `# graft ssh:`). A declaration in the
file cannot drift from it.

## Enforce the boundary in a sandbox, not in rules

Since v2.14.0, mkFlake gives every repository a launcher, `packages.<system>.sandbox`, which runs one command
in a sandbox. The command sees its closure read-only, the repository's working tree, and only the host paths,
hosts and environment variables that the repository declares. graft's base, each provider's `graft.nix` and
the consumer's `perSystem` declare the policy, and Nix merges them. A run is the impure counterpart of a
derivation: a build sees only its inputs, and a sandboxed run sees its closure and what it declared
([Sandbox](reference/sandbox.md)).

This revises two decisions above. [Scope](#scope-nix-git-submodules-a-tag-comparison-and-a-sandbox-policy)
gains "the sandbox policy their operations run under". [graft hands text to agents, and never runs
one](#graft-hands-text-to-agents-and-never-runs-one) still holds: graft never chooses to run an agent. The
launcher runs the command that a person names, and graft has no `graft sandbox` command.

Why:

- **A boundary must be enforced, not advised.** Write rules in `knowledge-base.yaml`, `AGENTS.md` and an agent
  tool's permission list are context: an agent can misread them, and a program that the agent starts never
  reads them. A sandbox stops a write or a connection whatever the agent read.
- **The closure is Nix's own idea of what a program may see.** graft already wires the tools as Nix packages,
  so the read-only part of the policy is their closure, with nothing to list by hand.
- **A provider may add to what the run can see, never to what it can reach.** A provider's `packages`,
  `env.set` and `protect` apply. Its `network.allow`, `paths.readOnly`, `paths.writable` and `env.pass` are
  requests, which apply only when the consumer names the scion in `sandbox.grant`. An upgrade pull request
  shows a provider's move as one gitlink, so a provider that widened the network there would otherwise arrive
  unreviewed. graft takes a provider's values as plain values, so no priority can change the consumer's
  policy, and `lib.mkForce` in a provider is an error that says so.
- **The sandbox never writes what decides the sandbox, or what the host runs.** The policy comes from
  `flake.nix`, `flake.lock` and each `graft.nix`, and all of them are in the writable repository. So graft's
  base protects them, at any depth, with `.gitmodules`, `.envrc`, `justfile`, `.pre-commit-config.yaml` (the
  pre-commit framework's hook runs the commands it names, at the host's next commit), and the git metadata
  that tells host git where the repository is and what to run (`.git` files, config, hooks). A command in the
  sandbox cannot widen the next run, and cannot plant a hook that the host's git runs later.
- **A pattern costs one walk.** `protect` takes a path, or `**/NAME` for a name at any depth, scions and
  untracked files included. The launcher finds both in the same walk of the working tree that finds the `.git`
  entries, and that walk is also what it compares after the run. Other glob shapes would need a second walk.
- **The terminal stays the caller's.** A command that shares a terminal can push input into it (`TIOCSTI`),
  which the caller's shell runs after the run. A new session (`setsid`) would stop that, but it also takes
  Ctrl-C and job control from the command. So a seccomp filter refuses `TIOCSTI` and `TIOCLINUX`, whatever
  the kernel's `dev.tty.legacy_tiocsti` says.
- **Declarations live in the files they describe.** A provider declares what its tools need in its
  `graft.nix`, beside the tools, and the consumer declares its own in `perSystem`. There is no new file.
  `--explain` names the file that set each value.
- **The renderer can be replaced.** graft wires the policy (`packages.<system>.sandbox.policy`); a renderer
  in `lib/sandbox/` turns it into the launcher. Bubblewrap renders it on Linux, and Seatbelt on macOS, with
  one runtime for both (the arguments, the pins, the checks before and after the run, the proxy) and a
  backend each that turns the run into a command. A systemd unit can render the same policy later.
- **On macOS, Seatbelt.** `sandbox-exec` with a profile that denies by default is the one sandbox that
  macOS gives an unprivileged process. Apple deprecated it, but Chromium, OpenAI's codex and Anthropic's
  sandbox-runtime use it, and Nix's own darwin builds do. A VM per run would cost seconds, not milliseconds.

Costs accepted:

- **The sandbox protects the host, not the repository's history.** A command in the sandbox can commit, so
  it can write objects, refs and the index. `rm -rf .git/*` inside deletes objects and refs; only the
  protected files stay. Push what you need to keep, or run the command in a separate clone.
- **A planted nested repository.** Any writable directory can receive a new repository (`dir/.git/` with
  `core.fsmonitor`, or a new gitlink with a `.git` file). Host git that runs there later (a shell prompt's
  `git status` after `cd`, an editor) runs what that repository's config names. Git's `safe.directory` check
  does not help: the owner is the same user. After each run, the launcher prints a warning that lists the
  new git metadata and gitlinks, and the next step. It reports them; it does not prevent them. Read it before
  you run git in a directory that it names.
- **New files with a protected name are reported, not prevented.** A pinned path must exist before the run,
  so a new `.envrc` or `justfile` (or a `config.worktree`) is in the warning after the run.
- **A git config entry can run a script from the working tree.** A filter, a `textconv` or
  `core.fsmonitor = ./x` in the protected config runs what the run wrote to that script. Add the script to
  `protect`. (`core.hooksPath` is protected.)
- **A symbolic link or a second hard link on a protected path stops the run.** bubblewrap binds what a link
  points to, not the link, so the launcher cannot pin it; and a file with another hard link can be written
  through that link. The launcher stops, and says how to replace the file, rather than leave the path
  writable.
- **Two runs at once can race, in theory.** The launcher checks the paths that it pins, then bubblewrap binds
  them. Between the two, another run in the same repository could replace a folder above a file that the
  other run created. Such a file is new, so the warning after that run reports it. Not exploited; a fix
  would resolve each path once and bind what it found.
- **The launcher can write to the host's git metadata.** Before the run, it creates an empty `hooks/` in a git
  directory that has none, and the `core.hooksPath` folder when it is missing, so that the run cannot create
  them.
- **Commits made inside are unsigned.** The signing key is not visible, so the launcher turns signing off for
  the run. The startup line says so when your git config signs.
- **Only proxy-aware HTTP(S) reaches the network.** `network.allow` works through a CONNECT proxy, so a
  program that ignores `HTTPS_PROXY` gets no network, and `git push` over ssh fails.
- **Linux needs unprivileged user namespaces.** Docker's default seccomp profile blocks them. The launcher
  never falls back to an unsandboxed run.
- **macOS has no mount or network namespace.** Seatbelt allows or denies paths, so the run reads the macOS
  files that a process needs (system libraries, `/private/etc`), `HOME` is a new, empty temporary folder at
  another path than your home's, and the proxy is a port on `127.0.0.1` that any local process
  could also connect to (it forwards only to `network.allow`). A run cannot bind a port, so a test that
  starts a local server fails. A program that verifies TLS with macOS's own trust service (Go programs, for
  example) cannot reach it: it must read `SSL_CERT_FILE`. A process that drops the run's environment and
  leaves its process group outlives the run, where on Linux the end of the process namespace stops it. The
  Seatbelt support is untested on a Mac yet ([Sandbox](reference/sandbox.md#on-macos)).
- **Commands come only from the closure.** A program installed on the host outside Nix is not visible. Add
  its package to `sandbox.packages`, or use `--with PKG` for one run.
- **A start costs about 50 ms, and about 100 ms with the network** (the proxy starts too), measured on one
  machine with a small repository. The launcher walks the working tree before and after the run. Nix computes
  the policy when it builds the launcher, not at each run.

Reopen when unprivileged user namespaces are no longer available on the machines that run graft, when Apple
removes `sandbox-exec`, or when a provider's real needs cannot be expressed as requests that a consumer
grants.

Not yet:

- A systemd renderer for deployed services.
- A microVM for code that no one has reviewed.
- `graft try` and CI upgrade jobs inside the sandbox.
- A sandbox for each tool, not only for the whole run.
- A clone mode, with results returned by `git fetch`, which would protect the repository's history.
- Credential brokers.
- Profiles: `sandbox.profiles.<name>`, each a launcher `packages.<system>.sandbox-<name>` (a review agent with
  a read-only repository, for example). They will be additive.

## Where graft v1's features went

graft v1 is on the read-only `v1` branch (2025-12 to 2026-10). Earlier experiments are on `v0.2` and
`v0.1`. None of these features return:

| v1 | Now |
|---|---|
| `changes:`, migration commands, atomic upgrade with rollback | The upgrade pull request is the transaction: a branch, checked by the whole `nix flake check`, merged or closed. A major version declares a break, the CHANGELOG says what to do, and `graft migrate` writes a brief. |
| `commands:`, `graft run dep:command` | A provider's `graft.nix`: its tools go on the consumer's `PATH` through `packages.<system>.tools`, its apps and checks into the consumer's flake. A tool that is not tied to a scion can be an ordinary flake input, as graft is. |
| State queries, run logs | The repository's own checks and CI. |
| Grove (a terminal UI for many repositories) | Out of scope. `nix run github:danielnaab/graft/v2.13.1 -- outdated -C <repo>` reads any checkout. |
| Scions as agent sessions | Agent tools outside graft. A scion is a grafted repository at a release tag. |
| Transitive dependencies (`.graft/` in `.graft/`) | One level. |
| Providers that place files anywhere in a consumer | Only graft's own workflows, by their marker. This waits for a second real case. |
