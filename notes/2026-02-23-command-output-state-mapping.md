---
title: "Command Output State Mapping and Grove Workflow Operationalization"
date: 2026-02-23
status: working
participants: ["human", "agent"]
tags: [exploration, graft, grove, primitives, workflows, state, design]
---

# Command Output State Mapping and Grove Workflow Operationalization

## Context

Arose from a software factory dogfooding session. The `implement` command
runs Claude Code against a slice step. The question: should `verify` run
automatically after, or should it be a separate command? That question opened
into a broader exploration of multi-step processes, the right primitives, and
how graft and grove divide responsibility.

---

## The Problem with Embedding Sequences in Scripts

The obvious fix — have `implement.sh` call `verify.sh` after Claude finishes
— works for the happy path but fails on the recovery path. If verification
fails after a five-minute Claude session, you don't want to rerun the full
implement step. You need to fix the issue and run verify independently. So
verify being a separate command is load-bearing regardless, and embedding it
in the script hides a workflow at the wrong layer of abstraction.

The same applies to any multi-step sequence: the composition should be
declared somewhere it's visible, nameable, and executable as a unit — not
buried inside a script that pretends to be a single step.

---

## The Two-Layer Model

The key insight: Claude Code and Grove serve different but complementary
orchestration roles.

**Claude Code** orchestrates with judgment. It reads context, decides which
graft commands to run, in what order, based on the current situation. The
sequence `implement → verify → fix → verify` isn't declared anywhere — Claude
reasons about it each time. This is the exploration layer.

**Grove** operationalizes proven patterns. Once the right sequence is
established through ad-hoc Claude usage, Grove captures it as a deterministic,
repeatable workflow. No AI judgment required at runtime — Grove just executes
the known-good sequence.

This gives a clear migration path: prove the pattern with Claude, then
crystallize it in Grove. The software factory's `implement → verify` pattern
is a first candidate.

---

## What Sequencing Actually Requires

Workflows imply branching: if this fails do that; if this succeeds do the
other thing. But the real use cases don't need branching — they need
**sequences**: run these steps in order, stop on failure. The distinction
matters because sequences are trivial to implement and reason about; workflows
are a state machine problem.

For the cases at hand:
- `implement → verify` — linear, stop on failure
- `plan → implement → verify` — linear, stop on failure
- `verify → (on failure) surface results` — failure *is* the output, not a branch

All of these are sequences. "Stop on failure" is what `&&` gives you in shell.
The only missing thing is a way to declare the sequence in graft.yaml so it's
visible, nameable, and runnable as a unit — not buried in shell.

---

## State Mapping: The More Elegant Primitive

Rather than declaring execution order directly, commands can declare what
state they **read** and what state they **write**. Execution order then
emerges from data dependencies — no explicit sequencing needed.

Graft's existing model is already a one-directional data flow:

```
state queries → context → command
```

State is the read side. Commands consume it via `context:`. There is no write
side — commands produce effects in the world, but those effects aren't modeled
as state. They're side effects that happen to change what future state queries
return.

The extension: commands can declare outputs that become named state, queryable
by subsequent commands:

```yaml
commands:
  implement:
    run: bash scripts/implement.sh {slice}
    writes:
      - session           # produces session state

  resume:
    run: claude --resume {session_id} --dangerously-skip-permissions
    reads:
      - session           # requires session state from a prior implement run
```

The `.session` file that `implement.sh` already writes to and `resume.sh`
reads from *is* this pattern, implemented ad-hoc. Formalizing it would make
the dependency explicit, observable, and composable.

### Execution order from data dependency

Once commands declare reads and writes, an orchestrator (Grove, or a build
system) can ask "what commands are satisfiable right now?" without being told
the order. `resume` can't run until `session` state exists. `implement`
produces `session` state. The sequencing is implied.

This is how build systems work. Make doesn't have workflows — it has targets
with declared inputs and outputs. Execution order is derived, not prescribed.

### The verify case

`verify` doesn't need `implement`'s *data output* — it just needs to run
*after* `implement` has changed the working tree. That's a weaker dependency:
temporal, not data-driven. Two clean options:

1. **Implicit**: both commands operate on the working tree (which *is* state).
   Grove can infer "these commands touch overlapping state — sequence them."
2. **Explicit**: `implement` writes a `last_modified` marker that `verify` can
   declare as a precondition.

Option 1 is more elegant and requires no new declarations.

---

## What This Model Opens Up

Once command outputs are first-class state, several things fall out for free:

- **Observability**: `graft state query session` shows what implement last
  produced, without reading the file directly
- **Resumability**: Grove can reconstruct "where were we?" from the state
  snapshot — the full pipeline state is visible
- **Parallelism**: commands with non-overlapping state can run concurrently
  without declaring it
- **Memoization**: if inputs haven't changed, skip the command — same as build
  caching
- **Retries**: a failed command can be retried without re-running its
  prerequisites, because their outputs are already in state

---

## Relationship to Existing Design

The `{% if state.verify is defined %}` guard already in `templates/plan.md`
is embryonically this pattern — checking whether state exists before consuming
it. That same check could extend to "is this command ready to run?"

The [Unified Process Management](./2026-02-19-unified-process-management.md)
design establishes ProcessHandle as the execution substrate. Command output
state mapping would sit above that layer: ProcessHandle handles *how* a
command runs; state mapping handles *what* it produces and *what* can run
next.

---

## Open Questions

1. **Where does workflow declaration live?** In `graft.yaml` (portable,
   consumer-defined) or in Grove's config (workspace-level)? Probably graft.yaml
   for portability, with Grove understanding it natively.

2. **Granularity of state**: build systems draw the line at files. Graft draws
   it at JSON from shell scripts. Command output state would add "JSON produced
   by a command invocation, persisted and queryable." What's the storage model?

3. **The AI step gradient**: can a workflow step be "ask Claude to decide"
   rather than a deterministic command? Grove workflows proven out via Claude
   usage should support a gradient from fully AI-driven to fully deterministic,
   with steps that can be promoted from one to the other.

4. **Implicit vs. explicit dependencies**: deriving order from overlapping
   state (like a build system) is elegant but requires Grove to understand
   what state each command touches. Explicit `reads:`/`writes:` declarations
   are more verbose but unambiguous. Probably both, with explicit taking
   precedence.
