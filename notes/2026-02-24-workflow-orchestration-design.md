---
title: "Workflow Orchestration: End-to-End Feature Development Design"
date: 2026-02-24
status: active
participants: ["human", "agent"]
tags: [design, orchestration, sequences, workflow, grove, graft, ralph-loop]
---

# Workflow Orchestration: End-to-End Feature Development Design

## Context

Design session focused on wiring up a complete end-to-end workflow for creating features
in graft/grove, addressing gaps in the current software-factory, and asking whether the
ralph-loop pattern warrants a first-class graft primitive.

Related prior work: [Grove Agentic Orchestration (2026-02-18)](2026-02-18-grove-agentic-orchestration.md)

---

## The Problem Statement

Current state:
- `software-factory:implement` runs Claude once and stops — no post-implementation verification
- The ralph loop (iterate until done) exists as a bash script in `notes/`, not a graft primitive
- No human checkpoint mechanism — no way for a workflow to pause for review
- Grove has no workflow status view — can't see "step 2/4, iteration 3, verify failing"
- Verify output evaporates — not persisted as run-state, not observable in grove

Desired end-to-end flow:
```
plan → [human reviews] → implement-loop → [verify passes] → [human reviews step]
     → next step → repeat → all steps done → PR
```

With the constraint: everything must work from both `graft run` (CLI / Claude Code)
AND from grove's TUI. Human checkpoints must be observable AND actionable in grove.

---

## Key Design Tension: Shell Scripts vs. Native Primitive

Initial plan proposed `implement-loop.sh` — a bash script wrapping the implement+verify+retry
cycle. The human pushed back: if the ralph loop pattern repeats across multiple workflows,
a new bash script each time creates tech debt. Is there a cleaner primitive?

**The tension:**
- Shell scripts: immediate, no new infrastructure, opaque mid-run
- Native graft primitive: reusable, grove-observable, requires graft-engine work

**Resolution:** Build sequences with retry as a first-class graft primitive before writing
the shell scripts. The pattern repeats enough (implement+verify, deploy+smoke, plan+review)
that the abstraction pays for itself.

---

## Paradigm Analysis: What Are We Actually Building?

The `workflow-status.phase` state machine was identified, but that's just the status
tracking layer. The deeper question: what paradigm describes the whole system?

**What it is NOT:**
- Not a classical FSM (state is the entire collection of run-state files, not just a phase enum)
- Not a pipeline (pipelines are linear, deterministic; Claude output is non-deterministic)
- Not rayon/data-parallelism (our tasks are sequential and dependent, not homogeneous parallel)

**What it most closely resembles: a build system**

`make` solved this 50 years ago:
- Target dependencies → our `reads`/`writes`
- Skip targets with fresh outputs → our reads enforcement (don't run if state missing)
- Derive execution order from dependency graph → our producer/consumer map

`dbt` (SQL transformation tool) is the modern version of the same idea.
Graft is effectively **make for commands that happen to call Claude**.

**What we add that build systems don't have:**

1. **Retry loops with feedback** — make just fails. We need "if output is wrong, run
   a recovery step and retry." This adds a feedback edge to what's otherwise a DAG,
   turning it into a **control flow graph**.

2. **Human gates / synchronization barriers** — make is fully automated. We need
   "pause and wait for external confirmation." This is a **barrier synchronization**
   (distributed systems term) or **manual approval gate** (GitHub Actions term).

3. **Non-deterministic compute steps** — Claude's output isn't reproducible. Every
   traditional build system assumes deterministic steps. We're treating Claude like
   a compiler that might produce wrong output and needs external verification.

**Closest single existing paradigm: workflow engine with a non-deterministic agent node**

GitHub Actions is the closest analog:
- `needs:` dependencies → `reads`/`writes`
- `retry: max-attempts: 3` → our loop
- Environment protection rules with required reviewers → our checkpoint
- Job artifacts → run-state files

We're building a local, git-native CI/CD workflow engine where one of the "jobs" is
Claude Code. Novel element: **a non-deterministic reasoning agent as a workflow node**,
with external verification as the output quality gate.

**The Saga pattern** (from distributed systems) applies to the reject/compensate part:
when something fails after partial progress, run compensating transactions. Our
`reject` + re-implement is a saga rollback.

---

## Proposed graft.yaml Primitive

```yaml
# sequences: section in graft.yaml (new top-level key, analogous to commands: and state:)
sequences:
  implement-verified:
    steps: [implement, verify]
    on_step_fail:
      step: verify          # which step triggers retry
      recovery: resume      # run before retrying
      max_retries: 3
    args:
      - name: slice
        type: choice
        options_from: slices
        required: true
        positional: true
```

When a sequence runs, graft:
1. Writes `sequence-state.json` to run-state: `{name, current_step, iteration, status}`
2. Executes steps in order, tracking exit codes
3. On failure at `step`: runs `recovery`, then retries `step`, up to `max_retries`
4. On success: writes checkpoint state (for human review gate)
5. Grove shows sequence-state in Run State section with live phase/iteration

**Argument pass-through:** "pass-all" semantics — the sequence passes all its args
to every step. Steps that don't accept a given arg ignore it.

---

## Checkpoint Mechanism

The checkpoint is a state file (`checkpoint.json`) that serves as the universal
interface between running workflows and human approval. Works identically from
Claude Code (reads the file after command exits) and grove (displays prominently
in Run State section, Enter opens approval overlay).

```json
{
  "phase": "awaiting-review",
  "slice": "my-feature",
  "step": 2,
  "steps_total": 5,
  "iteration": 2,
  "message": "Step 2/5 complete. Verify passed after 2 iterations.",
  "created_at": "2026-02-24T10:30:00Z"
}
```

`approve` and `reject` commands clear/update the checkpoint. The same commands
work from `graft run` (CLI) or grove's Commands section. Grove adds an approval
overlay UI as a convenience layer on top.

---

## Run-State Schema (What Grove Shows During a Workflow)

| Entry | Producer | Key fields |
|-------|----------|------------|
| `session.json` | `implement` | `{id, slice}` |
| `verify.json` | `verify` | `{format, lint, tests, smoke}` |
| `sequence-state.json` | sequence executor | `{name, current_step, iteration, status}` |
| `checkpoint.json` | sequence executor | `{phase, slice, step, steps_total, message}` |

---

## Planned Vertical Slices

In dependency order:

| # | Slice | What | Key work |
|---|-------|------|----------|
| 1 | `verify-captures-state` | `verify` writes `verify.json` to run-state | 2 file edits |
| 2 | `sequence-declarations` | Parse `sequences:` in graft.yaml | graft-common + grove-core |
| 3 | `sequence-executor` | Execute sequences with retry in graft-engine | ~300 LOC Rust |
| 4 | `checkpoint-commands` | `approve`/`reject` commands | 3 new scripts |
| 5 | `grove-checkpoint-ui` | Prominent checkpoint display + approval overlay | ~200 LOC Rust |
| 6 | `dev-loop` | Multi-step automation (all steps, optional human review) | 1 new script |

Slices 1-4 are runnable from CLI/Claude Code without any grove changes.
Slice 5 adds grove UX on top.

---

## Prior Art: Agentic Orchestration Note (2026-02-18)

The earlier agentic orchestration exploration (see linked note) identified similar
primitives from a different angle:

- **Sessions**: named, trackable agent invocations (background, observable, reviewable)
- **Context**: workspace state assembled before session launch
- **Plans**: ordered sessions with dependencies and approval gates

The current design is consistent with that vision but starts smaller: sequences are
the intra-repo orchestration primitive (step A then step B within one repo), while
Plans (from the earlier note) are the inter-repo cross-workspace primitive.

Sequences → Plans is a natural evolution path.

---

## Open Questions

1. **Argument pass-through**: "pass-all" is the working assumption. Could be wrong for
   sequences where steps have conflicting arg names. Need to discover via use.

2. **Step marking**: `iterate.sh` currently tells Claude to check off `- [ ]` steps.
   In a sequence, does the executor mark steps, or does Claude? Current plan: let Claude
   mark steps (via iterate.sh instruction), use `--resume` for retries so Claude retains
   context. Executor doesn't touch plan.md.

3. **Grove UX issues**: Acknowledged as "will be discovered during self-hosting use."
   A `grove-ux-improvements` slice will capture these as they emerge.

4. **Async vs. sync execution**: Sequences are sequential by definition. The executor
   runs synchronously. Parallel execution within a sequence is not needed and adds
   complexity without current use cases.
