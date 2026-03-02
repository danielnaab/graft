---
status: working
purpose: "Design session: scion orchestration architecture — runtime abstraction, grove switchboard, worker handoff"
---

# Scion Orchestration Design

How graft launches and manages long-running workers in scions, and how grove
provides a managed observation and review experience. Builds on the
[scion lifecycle](2026-03-01-shoot-lifecycle-design.md) implementation and
revisits the [agentic orchestration](2026-02-18-grove-agentic-orchestration.md)
proposals in light of what scions now provide.

## Key Decisions

### Graft owns the runtime abstraction

Graft already defines commands (`run`, `env`, `working_dir`) and executes them
as subprocesses via `ProcessConfig`. A detached execution mode — where the
process outlives the graft invocation — is a small extension to this existing
model, not a new domain.

The alternative was putting runtime management in grove. This was rejected
because:

- **Command definitions live in graft.yaml.** Grove would need to re-derive
  command resolution, env setup, and working directory — reimplementing graft's
  command execution with a different backend.
- **CLI users get nothing without grove.** `graft scion create` from the
  terminal would produce a bare worktree with no worker started.
- **Graft already runs processes.** Adding "detached" to the execution modes
  (alongside "blocking with timeout") is incremental, not architectural.

The runtime starts with tmux. Docker, SSH, and cloud VMs are future backends
for the same abstraction. This parallels how graft already abstracts over git
remote protocols — a consistent interface over varying backends.

### Grove is a switchboard, not an orchestrator

Grove observes and connects. It does not launch, manage, or define workers.

The switchboard pattern is the proven approach for TUI + agent session
management (Agent Deck, tmuxwatch, Gas Town's `gt feed`). The TUI shows
session state and lets the human jump into any session. Agent interaction
happens in the native terminal session, not embedded in the TUI.

What grove does:

- **Detect sessions** — check if a runtime session exists for each scion
- **Show state** — artifact-derived (ahead/behind, dirty, last commit) plus
  session presence (active / no session)
- **Attach** — suspend grove TUI, connect human to the session, resume grove
  on detach
- **Review** — show diff against main, verify results, commit log
- **Trigger lifecycle** — `:scion create`, `:scion fuse`, `:scion prune` call
  graft, which handles everything including runtime

What grove does NOT do:

- Define or resolve commands (graft's job)
- Create runtime sessions directly (graft's job via runtime backend)
- Parse or embed agent terminal output (that's what attach is for)
- Manage agent lifecycle beyond triggering graft operations

### Hooks prepare the workspace, not the runtime

Scion lifecycle hooks (`on_create`, `pre_fuse`, etc.) are short-lived,
synchronous, gating operations. They run to completion and exit. They are
for workspace preparation — writing context files, configuring the environment,
installing dependencies.

Hooks do NOT launch the worker agent. The runtime abstraction handles that
separately, after hooks complete.

This means software-factory (the workflow package) stays runtime-agnostic.
Its hooks write files and run checks. They don't know about tmux, docker,
or any execution environment.

### The prompt is the handoff

The worker agent's initial context comes from a prompt, not from generated
configuration files. The earlier Layer 0 proposal (generating
`.claude/rules/work-assignment.md` via hooks) was rejected because:

- Rules files are ambient behavioral modifiers, not directional assignments
- Generated files are static snapshots that go stale
- Copying skills into every worktree is duplication
- The approach assumes the scion name maps to a known work unit

Instead, the `on_create` hook chain prepares context *files* (plan references,
state snapshots, CLAUDE.md adjustments) and the runtime launch assembles the
actual prompt from graft state dynamically. The prompt can include output from
`graft status`, relevant plan files, and assignment context.

How prompt assembly works is a workflow-package concern. Graft provides the
command definition and the runtime. The command's `run:` field can include
prompt construction: `run: "claude -p \"$(graft context)\""`.

## Architecture

```
graft.yaml                     grove
  commands:                      (switchboard TUI)
    worker:                        |
      run: "claude -p ..."         | observes:
  scions:                          |   scion state (via graft)
    start: worker                  |   session presence (via runtime)
         |                         |
         v                         | actions:
    graft engine                   |   :scion create → graft
      |                            |   :attach → runtime
      |-- scion lifecycle          |   :scion fuse → graft
      |     create worktree        |
      |     run hooks (sync)       |
      |                            |
      |-- runtime backend          |
      |     tmux (today)       <---+--- attach/detach
      |     docker (future)
      |     ssh (future)
      |
      v
    worker process
      (in worktree, managed by runtime)
```

### Layer responsibilities

| Layer | Owns | Does NOT own |
|---|---|---|
| **Graft** | Worktree lifecycle, command definitions, hook execution, runtime dispatch, state queries | UI, session observation, review workflow |
| **Workflow package** | Hook scripts (file prep, context assembly), command definitions, prompt templates | Runtime details, process management, UI |
| **Grove** | Human interface, session observation, attach/detach, review UI, lifecycle triggers | Command resolution, runtime creation, hook execution |
| **Runtime backend** | Session creation/destruction, process isolation, attach mechanism | What runs inside the session |

## Runtime Abstraction

### Interface

A runtime backend needs four capabilities, defined by the `SessionRuntime`
trait in `graft-common/src/runtime.rs`:

| Capability | tmux (implemented) | docker (future) | ssh (future) |
|---|---|---|---|
| **Launch** | `tmux new-session -d -s <id> -c <dir> <cmd>` | `docker run -d --name <id> -w <dir> <img> <cmd>` | `ssh <host> tmux new-session -d ...` |
| **Detect** | `tmux has-session -t =<id>` | `docker ps --filter name=<id>` | `ssh <host> tmux has-session ...` |
| **Attach** | `tmux attach-session -t =<id>` | `docker exec -it <id> bash` | `ssh -t <host> tmux attach ...` |
| **Stop** | `tmux kill-session -t =<id>` | `docker stop <id>` | `ssh <host> tmux kill-session ...` |

Note: tmux's `-t` flag uses `=<id>` prefix for exact matching, preventing
fnmatch/prefix collisions (e.g., `graft-scion-api` matching `graft-scion-api-v2`).

The session ID follows a naming convention owned by graft:
`graft-scion-<name>` (e.g., `graft-scion-retry-logic`). The `graft-` prefix
namespaces sessions to avoid collisions with other tools using tmux.

### Configuration (implemented)

The runtime backend is tmux (hardcoded default; the `SessionRuntime` trait
supports future backends). The worker command is named in `scions.start`,
not marked per-command:

```yaml
# graft.yaml — start names the command to launch as a runtime session
commands:
  worker:
    run: "claude -p '...'"

scions:
  start: worker           # single command name (not a list)
  on_create: setup-env    # hooks still run synchronously before start
```

`graft scion start <name>` resolves `scions.start` to a command, then
launches it via the runtime backend in a detached session. `graft scion stop
<name>` terminates the session. The `session: detached` per-command field
from the original design was not implemented — the simpler `scions.start`
approach avoids coupling runtime mode to command definitions.

### Graceful degradation

- If tmux is not installed, `graft scion start` returns a clear error.
- `graft scion list` detects runtime availability; when tmux is unavailable,
  the session indicator column is absent but all artifact state is shown.
- Grove detects runtime availability and adjusts its UI — no attach option
  when there's no session to attach to.

## Scion Worker Lifecycle

The complete flow from creation to fusion:

```
1. Human (via grove or CLI):
     graft scion create retry-logic

2. Graft:
     a. Validate scion name
     b. git worktree add .worktrees/retry-logic -b feature/retry-logic
     c. Run on_create hook chain (sync, blocking)
        → software-factory hooks: write context files, adjust CLAUDE.md
        → project hooks: install deps, configure tooling
     d. Return worktree path

2b. Human (optional, separate step):
      graft scion start retry-logic
      → Resolves scions.start command, launches via runtime backend
      → Session ID: graft-scion-retry-logic

3. Worker (in worktree, running in runtime session):
     - Reads CLAUDE.md, context files, plan references
     - Runs graft commands (status, verify) as needed
     - Makes commits on feature/retry-logic
     - Graft is unaware of the worker's internals

4. Human monitors (via grove):
     - Scion list shows artifact state (ahead/behind, dirty, last commit)
     - Session indicator shows whether runtime session is active
     - :attach retry-logic → suspends grove, connects to session
     - Detach from session → returns to grove

5. Human reviews (via grove):
     - :review retry-logic → shows diff against main, verify results
     - Diff, commit log, state query results in grove's scroll buffer
     - Decision: fuse, request changes, or continue

6. Human fuses (via grove or CLI):
     graft scion fuse retry-logic
     a. Check for uncommitted changes (reject if dirty)
     b. Run pre_fuse hooks (verify, lint — sync, blocking)
     c. Merge feature/retry-logic into main via temp ref
     d. Fast-forward main, sync worktree
     e. Clean up temp ref
     f. Run post_fuse hooks
     g. Kill runtime session (if active)
     h. Remove worktree + branch
```

Steps 1, 2 (including 2b), and 6 are implemented today (minus session
cleanup in 6g). Steps 3-5 describe the target experience.

## Grove Switchboard Design

### Scion list in the TUI

Grove shows scions alongside (or integrated with) its existing repo list:

```
retry-logic       3 ahead, 0 behind   2m ago   ● active
input-validation  1 ahead, 3 behind   31m ago
error-handling    0 ahead, 0 behind   —        ● active
```

The `● active` indicator comes from runtime session detection, not process
monitoring. It means "a session exists," not "the agent is healthy." This is
an honest signal — if the session exists but the agent crashed inside it, the
indicator still shows active, and the human can `:attach` to investigate.

When tmux is not available, the indicator column is absent. Everything else
works — artifact state is always available via git.

### Attach and detach

`:attach retry-logic`:
1. Grove checks for a runtime session `graft-scion-retry-logic`
2. If found: grove suspends its TUI, invokes the runtime's attach command
3. Human interacts directly with the agent in the terminal
4. On detach (tmux: `Ctrl-b d`): grove resumes, refreshes scion state

This is the switchboard pattern used by Agent Deck and tmuxwatch. The TUI
yields the terminal, then reclaims it. No embedded terminal emulation.

### Review

`:review retry-logic`:
1. Grove calls `git diff main...feature/retry-logic`
2. Shows diff in the scroll buffer with syntax highlighting
3. Shows commit log, verify state, ahead/behind
4. Offers `:scion fuse retry-logic` or `:request-changes retry-logic`

Review is purely artifact-based. No runtime session needed. The human can
review a scion whose agent has already exited.

## Open Questions

- **Session cleanup on fuse/prune**: should graft automatically kill the
  runtime session, or warn if one is active and let the human decide?
  Auto-kill is clean but could interrupt work. Warning is safer but adds
  friction.

- **Prompt assembly**: who constructs the prompt passed to the agent? Options:
  the command's `run:` field includes prompt construction inline; a dedicated
  `graft context` command provides assembled context; or the workflow package
  provides a prompt template that graft fills. The command `run:` field is
  simplest and requires no new graft primitives.

- **Multiple runtime backends simultaneously**: can different scions use
  different backends? (e.g., one in tmux, one in docker) The `runtime:`
  config is project-level in this design. Per-command override would add
  flexibility but also complexity.

- **Grove without tmux**: grove's scion features work without tmux (artifact
  state is always available), but the attach experience requires a runtime.
  Is this clear enough to users, or does it need explicit UX for "no runtime
  available"?

## Resolved Questions (from prior design sessions)

- **Worker launch integration**: separate `graft scion start <name>` command,
  not auto-launch on create. More composable. Implemented.
- **Runtime configuration**: `scions.start` names the command (simpler than
  per-command `session: detached`). Runtime backend is tmux by default; trait
  supports future backends. Implemented.
- **Pre-fuse merge strategy**: temp ref with fast-forward. Implemented.
- **Detached HEAD handling**: explicit error, not silent fallback. Implemented.
- **Scion name validation**: strict character set, no path traversal. Implemented.
- **Temp ref cleanup**: always cleaned up, even on failure. Implemented.
- **Dirty worktree on fuse**: rejected with clear error. Implemented.

## Relationship to Prior Designs

### What scions replaced from agentic orchestration (Slices 8-13)

| Old proposal | Scion equivalent | Remaining gap |
|---|---|---|
| Slice 8: Background Sessions | Scion + runtime backend | Implemented here |
| Slice 9: Session Monitor | Grove switchboard + attach | Designed here |
| Slice 10: Context Assembly | on_create hooks + prompt | Workflow-package concern |
| Slice 11: Review Flow | Grove review UI | Designed here |
| Slice 12: Plans | Future (sequences + scions) | Not addressed |
| Slice 13: Session Dashboard | Grove scion list | Designed here |

### What changed from the context provider exploration

- **`graft context` as built-in**: deferred. Prompt assembly works through
  the command's `run:` field and existing CLI commands. A dedicated `graft
  context` may emerge later as a convenience.
- **Workflow graphs / state machines**: deferred. Linear hook chains are
  sufficient for the scion lifecycle.
- **Auto-rebase on fuse**: not addressed. Multi-scion coordination is a
  future concern.

## Sources

- [Scion Lifecycle Design](2026-03-01-shoot-lifecycle-design.md) — scion
  commands, hooks, failure semantics (implemented)
- [Graft as Context Provider](2026-02-28-graft-as-context-provider.md) —
  worker model, artifacts over actors, local PR workflow
- [Agentic Orchestration](2026-02-18-grove-agentic-orchestration.md) —
  original session/plan/context proposals (Slices 8-13)
- [Sequence Primitives](2026-02-24-sequence-primitives-exploration.md) —
  sequence design decisions
- Community tools researched: workmux, Agent Deck, Gas Town, NTM,
  CLI Agent Orchestrator (AWS Labs), agent-tmux-monitor, TmuxCC
