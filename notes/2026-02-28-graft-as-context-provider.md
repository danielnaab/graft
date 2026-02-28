---
status: working
purpose: "Exploration of evolving graft from orchestrator to context provider for autonomous Claude workers"
---

# Graft as Context Provider

Exploration of how graft, grove, and software-factory might evolve if Claude Code
becomes the orchestrator and graft becomes a queryable world model.

## The Inversion

The current architecture has a clear hierarchy:

```
Human -> Grove (TUI) -> Graft (engine) -> Claude (subprocess)
```

Grove selects commands. Graft sequences them (implement, verify, retry). Claude is the
lowest layer -- a worker that receives a prompt via `claude -p`, does work, and exits.
The workflow is rigid and predetermined in `graft.yaml` sequences.

The alternative is an inversion:

```
Human -> Grove (monitor/gateway) -> Claude (orchestrator) -> Graft (context/state API)
```

Claude becomes the decision-maker. Graft becomes the world model Claude reasons over.
Grove becomes mission control, not a command palette.

## Claude Instances as Workers

In a side-by-side model, grove doesn't launch Claude as a subprocess it monitors.
Grove is the **control plane** -- it manages work assignments, tracks progress, and
presents state. Claude instances are **workers** that pick up work, do it, and report
back through graft state.

The communication channel is the filesystem. Graft state queries, run-state, focus --
all already file-based and JSON. No SDK bridge needed, no PTY embedding, no IPC
protocol. Just shared state.

### What a worker needs

1. **An assignment**: what to work on (slice + step + context)
2. **A world view**: what state exists, what constraints apply, what's been tried
3. **A way to report back**: record progress, flag problems, request human input

Graft already provides all three:

- Assignment = focus + iterate output
- World view = state queries + command metadata + workflow position
- Report back = writing to run-state, checking off steps, updating slice status

### Worker lifecycle

```
1. Query graft: "what's my assignment?"
2. Query graft: "what's the current state?"
3. Do work (edit files, run tests)
4. Record progress in graft state
5. Hit a gate? Write checkpoint, stop.
6. More steps? Go to 1.
```

Workers don't need grove. They just need graft. Grove is for the human watching.

### How workers get started

These aren't mutually exclusive:

- **From grove**: `:launch retry-logic` starts a Claude instance in a separate
  terminal/tmux pane, pointed at that slice.
- **From the shell**: `graft work retry-logic` starts a Claude worker directly.
- **From CI**: a GitHub Action runs `graft work` against each ready slice.
- **Parallel**: multiple workers in separate git worktrees.

The launch mechanism is decoupled from the monitoring mechanism. The step from "one
Claude worker in a terminal" to "three parallel workers in worktrees" to "workers in
CI" is a deployment decision, not an architecture change.

## Artifacts Over Actors

Tracking worker state (alive? running? stuck?) is distributed systems territory --
process liveness, stale locks, crash recovery. The thing you actually care about is
**what happened to the code**. That's observable from artifacts, not processes:

- **Git commits** -- the worker made progress (via `git log`)
- **Step checkmarks** -- the worker finished a step (in `plan.md`)
- **Run-state files** -- verify results, context snapshots, session IDs
- **Slice status** -- frontmatter changed from `in-progress` to `done`

These are all things graft already knows how to query. No worker registry, no
heartbeat, no coordination protocol needed.

"Is someone working on retry-logic?" The signals are: is there an uncommitted diff
touching files in that slice's plan? Is there a recent session.json? These are state
queries against existing artifacts.

If a worker dies silently, grove shows "last activity: 47m ago" and the human knows
something's wrong. That's better than a "running" status that lies because the process
crashed after writing it.

## Local Branch/Merge as Coordination

Workers operate in git worktrees on feature branches. When work is done, it needs to
merge back to main. The checkpoint mechanism (approve/reject) maps directly to a
**local pull request** workflow.

### What the human does at a checkpoint

The same things they do in a PR:

- **See the diff** against main
- **See test results** (verify state)
- **Read what was done** (context snapshot, commit messages)
- **Comment** ("this error handling looks wrong")
- **Approve** (merge to main) or **request changes** (worker continues)

Grove already has most of the rendering machinery. The scroll buffer can show diffs.
The checkpoint overlay can show approve/reject. The header shows branch and dirty
state.

### What a review looks like in grove

```
-- retry-logic (feature/retry-logic -> main) --
3 commits, +127 -14, 4 files changed

Commits:
  a1b2c3 Add retry configuration to sequence engine
  d4e5f6 Implement retry loop with backoff
  g7h8i9 Add retry tests and update spec

Verify: pass (fmt ok, clippy ok, tests 423/423)

-- Diff --
 crates/graft-engine/src/sequence.rs
 +pub fn execute_with_retry(
 +    steps: &[Step],
 +    max_retries: u32,
 ...

> :approve retry-logic
> :request-changes retry-logic "handle the timeout case"
> :comment retry-logic "nice approach on the backoff"
```

`:approve` merges the branch to main and cleans up the worktree. `:request-changes`
writes feedback to a known location and the worker picks it up on its next iteration.
`:comment` is non-blocking informational feedback.

### Integration ordering

The hard part of parallel workstreams is integration order:

- `retry-logic` finishes first, approved, merged to main
- `input-validation` was branched before the merge -- now needs rebase
- `error-handling` depends on `retry-logic` -- can't merge until it lands

Graft + grove can handle this tightly:

**Approve order matters.** When you approve `retry-logic`, graft merges it. Now graft
knows `input-validation` is behind. State shows: "input-validation: 2 commits behind
main, may need rebase."

**Auto-rebase on approve.** Approving a workstream could rebase remaining branches
onto the new main. Clean rebase = worker continues unaware. Conflicts = graft surfaces
them as state.

**Dependency declarations.** Slices already exist as entities with status. They could
declare dependencies on other slices. Graft already has flat dependency resolution.

**Verify after merge.** Approval re-runs verify on the merged result. If retry-logic
passes in isolation but breaks something on main, you catch it before the next
workstream builds on it.

### Workstream state query

All derivable from git + existing graft state:

```json
{
  "workstreams": [
    {
      "slice": "retry-logic",
      "branch": "feature/retry-logic",
      "commits_ahead": 3,
      "commits_behind": 0,
      "verify": "pass",
      "status": "checkpoint"
    },
    {
      "slice": "input-validation",
      "branch": "feature/input-validation",
      "commits_ahead": 1,
      "commits_behind": 3,
      "verify": "running",
      "status": "implementing"
    }
  ]
}
```

No new storage needed. Git is the concurrency primitive.

### What this replaces

For the inner development loop -- iterating on features before they're ready for the
team -- this replaces GitHub PRs entirely. You don't push until a workstream is
approved, merged to main locally, and verified on the integrated result. Then you push
main with clean history, no PR noise for work-in-progress.

GitHub PRs still make sense for the outer loop -- review by other humans, CI on shared
infrastructure, cross-fork merging. But the inner loop of "AI does work, human
reviews, merge to local main" is faster and tighter when it stays local.

## How Each Component Evolves

### Graft

**From command runner to queryable world model.** State queries become the primary
interface. Commands shrink to pure state-mutating primitives.

New capabilities:

- **`graft context`** -- composite query assembling everything a worker needs (state
  queries + command metadata + workflow position + focus). First-class operation, not
  defined in graft.yaml.
- **Workflow graphs** -- sequences evolve from linear pipelines with hardcoded retry
  into state machines with valid transitions. Claude navigates the graph; graft
  validates transitions.
- **Branch-aware state queries** -- run against a specific branch or worktree, similar
  to temporal queries (`--commit`) but targeting active branches.
- **Merge operations** -- graft-managed merge that validates, merges, re-verifies, and
  rebases remaining branches.

The `reads`/`writes` command metadata (recently added) becomes the data-flow graph
Claude uses to reason about what to do. `graft help --json` already exposes this.

### Software-factory

**From script bundle to workflow schema.** Many scripts exist to wrap "pipe prompt to
claude." In the inverted model:

- `implement.sh`, `resume.sh`, `diagnose.sh`, `review.sh`, `spec-check.sh` --
  disappear as commands. Become prompt templates that Claude reads and applies within
  its own session.
- `iterate.sh` -- survives as a pure query (or absorbed into `graft context`).
- `verify.sh` -- survives as a pure action.
- `list-slices.sh`, `read-slice.sh` -- survive as state queries.
- `approve.sh`, `reject.sh` -- survive as pure actions.

Software-factory's value becomes the **workflow pattern** -- the reusable state machine
declaration that any graft consumer can adopt and Claude can navigate. The work
protocol is standardized; you could swap workers (different agent, a human developer,
a hybrid).

### Grove

**From command palette to mission control.**

- **Session monitoring**: show what workers are doing based on artifact-derived state
  (last commit, current step, verify status, elapsed time)
- **Context curation**: human prepares context before launching workers -- focus a
  slice, set constraints, pin relevant state
- **Review interface**: show diffs, verify results, commit history for each workstream.
  Approve/reject/comment as local PR equivalents.
- **Multi-session dashboard**: show all active workstreams, their progress, their
  relationship to main. Intervention at the portfolio level.

The entity-focus slice (planned) is step zero -- it decouples grove from
software-factory's JSON shape and creates the mechanism for human-to-worker intent
signaling.

## The Architecture

```
+-----------+     state queries      +--------+
| Claude    | <--------------------> | Graft   |
| Workers   |  artifacts (git,       | (world  |
| (branches)|  run-state, plan.md)   |  model) |
+-----------+                        +--------+
      |                                  |
      | git commits,                     | state queries,
      | run-state writes                 | merge ops
      |                                  |
      v                                  v
  [worktree]                         +--------+
  [worktree]  <--- approve/reject -- | Grove   |
  [worktree]       rebase            | (mission|
                                     | control)|
                                     +--------+
                                         ^
                                         |
                                       Human
```

- **Workers** operate in worktrees on feature branches. Read graft state, do work,
  write artifacts. Fully decoupled. No registration, no heartbeat.
- **Graft** provides state queries over artifacts and merge/rebase operations.
- **Grove** renders artifact-derived state. Shows progress, diffs, verify results.
  Provides review interface. Approval triggers merge.
- **Coordination** is git branches and worktrees. Conflicts resolved at merge.
  Standard git workflow.

## Open Questions

- How does a worker receive human feedback from `:request-changes`? A file in
  run-state that the worker's prompt instructs it to check? A convention in
  software-factory's work protocol?
- Should `graft context` be a built-in graft operation or a composable state query
  defined in software-factory?
- How much workflow intelligence belongs in graft (validating transitions) vs. in the
  worker (deciding which transition to take)?
- What's the right granularity for approval -- per-step, per-slice, or per-batch?
- Should grove auto-rebase remaining branches on approve, or surface the need and let
  the human trigger it?

## Sources

- [Entity Focus Slice](../slices/grove-entity-focus/plan.md) -- decouples grove from
  hardcoded JSON extraction
- [Software-Factory graft.yaml](../.graft/software-factory/graft.yaml) -- current
  workflow definition
- [State Queries Spec](../docs/specifications/graft/state-queries.md) -- state query
  primitives
- [Agentic Orchestration](2026-02-18-grove-agentic-orchestration.md) -- earlier
  exploration of grove as dispatch board
- [Command Output State Mapping](2026-02-23-command-output-state-mapping.md) --
  command outputs as first-class state
- [Sequence Primitives](2026-02-24-sequence-primitives-exploration.md) -- sequence
  design decisions
