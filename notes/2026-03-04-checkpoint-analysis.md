---
status: working
purpose: "Analysis of checkpoint.json ownership, current limitations, and use case exploration for human-in-the-loop workflows"
---

# Checkpoint Analysis (2026-03-04)

## Current State

### Ownership split

**Graft engine** owns checkpoint as a first-class sequence concept:
- `checkpoint: Option<bool>` on `SequenceDef` in `graft-common/src/config.rs`
- `write_checkpoint_json()` in `graft-engine/src/sequence.rs`
- Written automatically when a sequence with `checkpoint: true` finishes successfully
- File: `.graft/run-state/<repo>/checkpoint.json`
- Format: `{phase, sequence, args, message, created_at}`

**Software-factory** owns the response to checkpoints:
- `approve.sh` — mutates `phase` to `"approved"`
- `reject.sh` — mutates `phase` to `"rejected"`, optionally adds `feedback`
- `resume` command — reads `checkpoint.json` for context (e.g., rejection feedback)

### The fundamental problem

The checkpoint is **terminal, not blocking**. The sequence writes checkpoint.json
and exits 0. Nothing in graft knows how to:
- Wait for the human's decision
- Resume after approval
- Branch after rejection
- Connect the checkpoint to whatever comes next

The continuation is entirely manual: the human must know to run `approve`, then
`resume` or another sequence. The engine thinks it's done. The file is an artifact,
not a control-flow mechanism.

Calling it a "checkpoint" implies pause-and-resume semantics that don't exist.
What it actually is: "write a status file on success."

### grove-checkpoint-ui plan is stale

The plan references `repo_detail.rs`, `overlays.rs`, `hint_bar.rs` — a TUI
architecture that was replaced by the transcript paradigm. The entire approach
needs redesign for the current `TranscriptApp` / `ScrollBuffer` / `ContentBlock`
architecture.

## Use Case Exploration (weak → strong)

### 1. "Review my work" (weak)
Agent implements, checkpoint fires, human reviews diff. This is just a PR review
with extra steps. No urgency, no branching, no consequence to delay. The human
always approves or stops bothering. Checkpoint adds nothing over "the agent
finished, go look at it."

### 2. "Review my plan before I spend 20 minutes implementing" (moderate)
Agent produces a plan, checkpoints. Human reviews the plan *before* implementation
begins. Catches wrong approach in 30 seconds instead of after 20 minutes of wasted
work. Stronger because the human's input *changes the trajectory* — approving plan
A leads to different code than redirecting to plan B.

### 3. "Multiple agents, each needing decisions at different times" (strong)
3 scions implementing 3 slices. Each checkpoints at different times with different
kinds of decisions. Human opens grove, sees pending checkpoints, handles in priority
order. While reviewing one, others continue working. Throughput gain from
parallelism justifies checkpoint overhead. The human is a router, not a bottleneck.

### 4. "Gate irreversible actions" (strong)
Agent prepares a database migration → CHECKPOINT before apply. Agent prepares API
schema changes → CHECKPOINT before publication. The checkpoint prevents *real harm*,
not just low-quality code.

### 5. "Decision nodes in a DAG" (strongest)
Library API change → CHECKPOINT (human approves API design) → 3 scions fork to
implement against approved API → each CHECKPOINT → human reviews, rejects one with
feedback → rejected scion resumes → all approved → fuse step merges → CHECKPOINT
before main. The human makes 5-6 high-leverage decisions. Agents do hours of
parallel work. Each decision shapes everything downstream. Without checkpoints:
either agents guess (risky) or everything is serial (slow).

## Key Insight

The strongest checkpoint is one where:
- The human's decision **materially changes** what happens next
- The cost of proceeding without review is **high** (wasted work, broken systems)
- The checkpoint carries **context** the human needs to decide
- The options are **actionable** (not just approve/reject but redirect, choose, modify)
- The agent **cannot usefully proceed** without the decision

The current mechanism supports none of these well. The file is a dead end that
requires manual knowledge to escape.

## First-Principles Redesign

### The recurring tension

The design history reveals a fork the system keeps oscillating between:

1. **Graft as orchestrator** — workflow is declared in graft.yaml, graft executes
   it, grove observes. (Sequences are this model.)
2. **Graft as world model** — graft provides state, the orchestrator is external
   (Claude, the human, grove). (The context-provider note is this model.)

Checkpoints try to be both: sequence machinery (model 1) that requires human
intervention (model 2). None of the obvious solutions resolve this cleanly.

### Four candidate models compared

**Model A: Actionable State** — Run-state files carry an `actions` field declaring
next steps. Grove renders them. Sequences stay simple, run and exit.

- Pro: No engine changes. Fits artifacts-over-actors.
- Con: Scatters workflow across JSON files. No single declaration of the workflow.
  No validation. (Feb 23 note: "composition should be declared somewhere visible,
  nameable, and executable as a unit.")

**Model B: Resumable Sequences** — Sequences persist progress and can be re-invoked
to resume from the last step. Checkpoint is a step that exits "paused."

- Pro: Workflow in one place. Validates step order.
- Con: Circular justification (Feb 24). Adds process lifecycle complexity. Moves
  toward graft-as-orchestrator, away from graft-as-world-model.

**Model C: Grove as Workflow Operator** — Grove renders artifact-derived state and
provides review actions (`:approve`, `:request-changes`). Human chains short
sequences manually.

- Pro: Cleanest separation. No engine changes.
- Con: Workflow logic lives in the UI, not portable. Doesn't compose when
  software-factory adds new steps.

**Model D: Workflow Graph as Queryable State** — Graft declares the workflow graph
in graft.yaml. Graft computes next-actions from the graph + current state. But graft
doesn't execute it — the caller (grove, CLI, Claude) drives the loop.

- Pro: Resolves the tension. Workflow is declared and validatable (model 1). Execution
  is external and flexible (model 2). Graph is inspectable.
- Con: Requires a new concept (gates/predicates). More complex than models A/C.

### Model D: gates as declarative predicates

A gate is not a command. It's a predicate over current state. The sequence executor
evaluates it: true → advance, false → "waiting." No special exit codes, no running
processes.

```yaml
steps:
  - step: implement
  - step: verify
    on_fail: {recovery: resume, max: 3}
  - step: review
  - gate: checkpoint.phase == "approved"    # predicate, not a command
  - step: fuse
```

The things that *produce* the state the gate checks are regular commands and human
actions. `review` writes `checkpoint.json` with `phase: awaiting-review`. `approve`
(a command the human runs) changes it to `"approved"`. The gate just checks.

**Why predicates, not commands:**
- Gates compose: multiple conditions can be AND'd
- Cross-scion dependencies use the same mechanism: `gate: main.contains(dep_slice)`
- The sequence graph is inspectable: `graft sequence status` evaluates all predicates
  and reports which gates block and why
- No new execution semantics — steps run or wait, no "paused" exit code

**Three categories of gates (same primitive):**
- Approval: `checkpoint.phase == "approved"` (human reviews work)
- Decision: `decision.choice != null` (human picks a direction)
- Danger: `authorization.confirmed == true` (human authorizes irreversible action)
- Cross-scion: `main.contains(dependency_slice)` (wait for dependency to land)

All are predicates over state. Software-factory defines the specific predicates and
the commands that satisfy them.

### Re-invocation trigger

When a gate blocks, the sequence exits with "waiting" status. Something must later
re-invoke it. Options:

1. **Manual**: human types `:run <sequence>` again (simplest)
2. **Automatic**: the approve command re-invokes the sequence after mutating state
3. **Reactive**: file watcher detects state changes and re-evaluates (most complex)

Simplest first version: the approve command re-invokes the sequence. No file
watchers, no polling. The human acts, the action triggers continuation.

### Interaction with scions

Each scion runs its own sequence independently with its own run-state directory,
so gates are naturally per-scion. Scion A can be approved while B is still
implementing. Cross-scion gates ("don't fuse B until A is fused") are predicates
over shared state (main branch contents), using the same mechanism.

### Open questions (for next session)

- **Predicate syntax**: What expression language? Simple field equality? JSONPath?
  A custom DSL? How expressive does it need to be?
- **State scope**: What state can predicates reference? Only the current scion's
  run-state? Any run-state? Git state?
- **`checkpoint: true` migration**: How does this replace the current boolean flag?
  Can the transition be incremental?
- **Sequence re-invocation protocol**: How does the approve command know which
  sequence to resume and with what args? Is this stored in sequence-state.json?

## Resolution: Scions as Checkpoints

### The core insight

Checkpoint.json is redundant with scion branch state. A scion that has completed
its sequence (commits ahead of main, verify passed, no active worker) IS a
checkpoint — structurally, by virtue of being on an unmerged branch.

**Fuse IS approve. The branch gap IS the checkpoint. No file needed.**

### Why checkpoint.json feels wrong

It's an artificial coordination mechanism overlaid on something git provides
natively. It creates a second source of truth weaker than the first:

- `phase: awaiting-review` is derivable from: scion has commits ahead + verify
  passed + no active worker
- `phase: approved` is derivable from: scion was fused to main
- The file and structural state can disagree (fused but file says "awaiting",
  or file says "approved" but nobody fused)

The `message` and `args` fields are also derivable from sequence name, verify
results, and scion config.

### The simplified model

**Within a scion**: sequences run to completion. No pausing, no gates, no
checkpoint files. Implement → verify with retry → done. Artifacts: commits,
verify.json, plan.md with checked steps.

**Between scion and main**: the gap is structural. Grove shows scion state
(commits ahead, verify results, diff). Human reviews and fuses or gives feedback.

**The workflow:**
1. `:scion create my-feature` → worktree + branch
2. `:scion start` → launches worker (runs sequence)
3. Sequence completes → scion has commits, verify passes
4. Grove shows scion: "3 commits ahead, verify: pass"
5. Human reviews diff
6. `:scion fuse` → merge to main, cleanup
7. Or: feedback + `:scion start` → worker resumes

**Eliminated:** `checkpoint: true` on SequenceDef, `write_checkpoint_json()`,
`checkpoint.json`, `approve.sh`, `reject.sh`. Three scripts and ~100 lines of
engine code replaced by structural state that already exists.

### Gate categories mapped to scions

| Gate type | Mechanism |
|---|---|
| Approval ("is this good?") | Scion waiting to be fused |
| Decision ("which way?") | Agent writes question, sequence exits, human responds, re-launches |
| Danger ("should I deploy?") | Post-fuse action against main; fuse itself is the gate |
| Cross-scion dependency | Fuse order; `pre_fuse` hook checks dependencies on main |

### What's actually missing

Not checkpoint files or gate primitives. Grove showing scion state richly enough
for the human to act:

- Diff summary (files changed, lines added/removed)
- Verify status (from scion's run-state/verify.json)
- Worker status (active or finished)
- Fuse readiness (verify passed? conflicts with main?)
- Obvious `:scion fuse` action
- Feedback path (write feedback + re-launch worker)

This is a grove UX problem, not an engine primitive problem.

### Implications

1. **`checkpoint: true` can be removed** from SequenceDef — sequences just run to
   completion
2. **`checkpoint.json` writing can be removed** from sequence executor
3. **`approve.sh` / `reject.sh` can be removed** from software-factory
4. **`grove-checkpoint-ui` slice is obsoleted** — replaced by scion review UX
5. **New slice needed**: "scion review" — grove shows rich scion state with
   fuse/feedback actions. This replaces the stale checkpoint UI plan.

## Sources

- `crates/graft-common/src/config.rs` — SequenceDef, checkpoint field
- `crates/graft-engine/src/sequence.rs` — write_checkpoint_json, execute_sequence
- `.graft/software-factory/scripts/approve.sh` — approve logic
- `.graft/software-factory/scripts/reject.sh` — reject logic
- `.graft/software-factory/graft.yaml` — sequence definitions with checkpoint: true
- `slices/grove-checkpoint-ui/plan.md` — stale UI plan
- `notes/2026-02-23-command-output-state-mapping.md` — two-layer model, state mapping primitive
- `notes/2026-02-24-sequence-primitives-exploration.md` — sequence critique, YAGNI, circular justification
- `notes/2026-02-28-graft-as-context-provider.md` — worker model, artifacts over actors, local PR
- `notes/2026-03-01-shoot-lifecycle-design.md` — scion lifecycle, composable hooks
- `notes/2026-03-02-dogfood-pain-points.md` — original pain point log
- `notes/2026-03-04-pain-point-strategy.md` — strategic resolution decisions
