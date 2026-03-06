---
status: decided
purpose: "Design decisions for gate predicates — blocking conditions in sequences"
---

# Gate Predicates Design Note

## Context

The checkpoint analysis (2026-03-04) resolved that scions ARE the checkpoint
mechanism. With checkpoints removed, there's a gap for declarative blocking
predicates in sequences — conditions that *block* execution until true, rather
than *skip* steps (which `when:` already handles).

Gates unlock:
- **Cross-scion dependencies**: "don't fuse B until A is on main"
- **Decision nodes**: "human chose option X, proceed down path X"
- **Danger gates**: "human confirmed destructive action"

## Open Questions — Resolved

### 1. Predicate syntax

**Decision**: Reuse `WhenCondition` struct.

The existing `when:` condition uses `{state, field, equals|not_equals|starts_with|not_starts_with}`.
Gates use the same syntax with different semantics: `when:` skips on false,
`gate:` blocks on false.

A string DSL (`state.field == "value"`) reads better but requires a parser and
is inconsistent with the existing approach. Structural reuse is simpler and the
`WhenCondition` is already validated, evaluated, and tested.

### 2. Step type representation

**Decision**: Add `gate: Option<WhenCondition>` to `StepDef`, mutually exclusive
with `name`.

An enum (`StepKind::Command { name, timeout } | StepKind::Gate { condition }`)
is architecturally cleaner but is a breaking change to YAML parsing and
requires updating every `step_def.name` access site. The optional field is
incremental and backwards-compatible.

The `name` field becomes an empty string for gate steps. We validate that
exactly one of `name` (non-empty) or `gate` is present.

### 3. Exit semantics

**Decision**: Exit 0 with `phase: "waiting"` in sequence-state.json.

The sequence writes `phase: "waiting"` at the gate's step index and exits
cleanly. This is consistent with how `phase: "running"` works for crash
resumability — the step index tells the executor where to resume.

A distinct exit code (e.g., 2) was considered but adds complexity for callers
that check exit codes. Exit 0 + phase is sufficient.

### 4. Re-invocation

**Decision**: Manual re-invocation.

The user runs `graft run <seq>` again after the gate condition becomes true.
The existing resume logic reads sequence-state.json and skips completed steps.
A "waiting" gate at step N resumes from N, re-evaluates the gate, and
continues if the condition now passes.

Triggered or reactive approaches add complexity without clear benefit at this
stage.

### 5. State scope

**Decision**: Run-state only (local `.graft/run-state/*.json` files).

Gates reference run-state files using the same `WhenCondition` evaluation as
`when:`. Git state (branch existence, merge status) is deferred to a future
extension — it would require a new condition type and evaluation path.

## YAML Syntax

```yaml
sequences:
  deploy:
    steps:
      - build
      - gate:
          state: review
          field: status
          equals: approved
      - deploy-prod
```

Gate steps have no `name`, `timeout`, or `when` — they are purely declarative
blocking predicates.

## Implementation Summary

1. Add `gate: Option<WhenCondition>` to `StepDef`
2. Validate: non-empty `name` XOR `gate` present (not both, not neither)
3. Parse `gate:` in `parse_step_def()`
4. In `execute_sequence()`, before executing a step:
   - If step has `gate`, evaluate via `evaluate_when_condition()`
   - If false: write `phase: "waiting"`, print diagnostic, return 0
   - If true: continue to next step (gate passed, nothing to execute)
5. Resume logic: `phase: "waiting"` resumes from the gate step, re-evaluates
6. Update `read_resume_index` to treat `"waiting"` like `"running"` (resumable)
