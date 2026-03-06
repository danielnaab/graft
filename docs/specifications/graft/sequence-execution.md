---
title: "Sequence Execution Specification"
date: 2026-02-26
status: draft
---

# Sequence Execution Specification

## Overview

Sequences are ordered lists of commands that share arguments and are executed
as a unit. This specification covers the full execution lifecycle: normal
execution, per-step retry (`on_step_fail`), run-state tracking
(`sequence-state.json`) and crash resumability.

## Definitions

- **Step**: a named command within the sequence's `steps:` list.
- **Pass-all args**: all positional arguments given to `graft run <seq> <args>`
  are forwarded to every step unchanged.
- **run-state dir**: `.graft/run-state/` relative to the consumer repository root.

---

## Normal Execution

```gherkin
Given a sequence with steps [A, B, C]
When graft run <seq> is invoked
Then steps A, B, C execute in order
And all receive the same positional args
And a progress line is printed to stderr before each step:
    "[1/3] <seq>: A"
```

```gherkin
Given step A exits 0 and step B exits 0 and step C exits 0
When the sequence completes
Then sequence-state.json is written with phase: "complete"
And "✓ Sequence '<seq>' completed successfully" is printed to stderr
And graft exits 0
```

```gherkin
Given step B exits non-zero and no on_step_fail is configured
When step B fails
Then sequence-state.json is written with phase: "failed" at step B's index
And "✗ Sequence '<seq>' failed at step 'B' (exit N)" is printed to stderr
And steps after B are not executed
And graft exits with B's exit code
```

---

## sequence-state.json Schema

Written atomically (`.tmp` + rename) before each step begins and on every
phase transition. Lives at `.graft/run-state/sequence-state.json`.

```json
{
  "sequence":   "<sequence-name>",
  "step":       "<current-step-name>",
  "step_index": 0,
  "step_count": 3,
  "phase":      "running | retrying | complete | failed",
  "iteration":  1
}
```

- `iteration` is only present during `phase: retrying`.
- `step` and `step_index` are set to the step currently executing or the step
  that caused the final `failed` phase.
- On `phase: complete`, `step` is set to `""` and `step_index` to the last
  step's index.

---

## Retry (on_step_fail)

```gherkin
Given a sequence with on_step_fail: {step: B, recovery: R, max: 3}
And step B fails on its first attempt
When the retry loop runs
Then recovery command R is executed with the same args
And then step B is re-executed
And sequence-state.json has phase: "retrying" with iteration: 1
And this repeats up to max times
```

```gherkin
Given step B fails and recovery R also fails
When recovery exits non-zero
Then the sequence aborts immediately with phase: "failed"
And no further retries are attempted
```

```gherkin
Given step B fails and on_step_fail.max: 0
When the step fails
Then the sequence fails immediately with phase: "failed"
And no recovery or retry is attempted
```

```gherkin
Given a step other than B (the on_step_fail.step) fails
When that other step exits non-zero
Then the sequence fails immediately with phase: "failed"
And no recovery or retry is attempted for it
```

---

## Crash Resumability

When a sequence is killed mid-run (SIGKILL, timeout, OOM), `sequence-state.json`
retains `phase: "running"` or `phase: "retrying"` at the interrupted step's
index. On re-run, the executor detects this and skips already-completed steps.

### Resumption Rules

```gherkin
Given sequence-state.json does not exist
When graft run <seq> is invoked
Then all steps execute normally (fresh run)
```

```gherkin
Given sequence-state.json records a different sequence name
When graft run <seq> is invoked
Then all steps execute normally (fresh run, state is stale)
```

```gherkin
Given sequence-state.json has phase: "complete" or phase: "failed"
When graft run <seq> is invoked
Then all steps execute normally (sequence already terminated cleanly)
```

```gherkin
Given sequence-state.json has phase: "running" at step_index: N for sequence S
When graft run S is invoked
Then steps 0..N-1 are skipped
And for each skipped step, "↷ Skipping <step-name> (already completed)" is printed to stderr
And execution begins at step N
```

```gherkin
Given sequence-state.json has phase: "retrying" at step_index: N for sequence S
When graft run S is invoked
Then steps 0..N-1 are skipped
And execution begins at step N (retry loop restarts with a fresh iteration count)
```

### Resumption Does Not Persist Retry State

When resuming from `phase: retrying`, the retry iteration count resets to zero.
This means the sequence gets a fresh `max` retries from the resumed step.

### Forcing a Fresh Run

Delete `.graft/run-state/sequence-state.json` to force all steps to re-execute
regardless of prior state. A `--force` flag is not provided in this version.

---

## Keybindings / CLI

No new keybindings. `graft run <dep>:<sequence-name> [args]` is the invocation.

---

## Decisions

- **2026-02-26**: Resumability uses sequence-state.json, not writes-state existence
  - Writes-state existence is ambiguous: a failed step may have written partial
    output; stale state from a previous run looks identical to fresh state;
    `verify` declares `writes: [verify]` so its state exists even on failure.
  - `sequence-state.json` tracks completion explicitly at the step level.
  - `phase: retrying` treated same as `phase: running` for resumption: retry
    restarts with a fresh count, which is simpler than persisting iteration
    state across process boundaries.

- **2026-02-26**: No --force flag in v1
  - Deleting `sequence-state.json` is the escape hatch.
  - Adding `--force` requires plumbing through the sequence arg interface;
    deferred until a concrete use case arises.

## Sources

- [Slice plan: sequence-resumability](../../../slices/sequence-resumability/plan.md)
- [graft.yaml Format Specification](./graft-yaml-format.md) — sequences section

---

## Conditional Step Execution (`when:`)

A step may declare a `when:` condition that gates its execution on a value in
a run-state file. Conditions are evaluated lazily — immediately before each
step runs — so they reflect the most recent run-state written by prior steps.

```gherkin
Given a step with when: {state: verify, field: lint, equals: "OK"}
And verify.json exists with field "lint" = "OK"
When the step is evaluated
Then the step executes normally
```

```gherkin
Given a step with when: {state: verify, field: lint, equals: "OK"}
And verify.json exists with field "lint" = "FAILED: unused import"
When the step is evaluated
Then the step is skipped
And "↷ Skipping <step> (condition not met: verify.lint)" is printed to stderr
```

```gherkin
Given a step with when: {state: verify, field: lint, equals: "OK"}
And verify.json does not exist
When the step is evaluated
Then the step is skipped
And "↷ Skipping <step> (condition not met: verify.lint — state file missing)" is printed
```

```gherkin
Given a step with when: {state: session, field: baseline_sha, not_starts_with: ""}
And session.json exists but has no "baseline_sha" field
When the step is evaluated
Then the step is skipped
And "↷ Skipping <step> (condition not met: session.baseline_sha — field missing)" is printed
```

### Operators

| Operator          | Evaluates to true when…               |
|-------------------|---------------------------------------|
| `equals`          | field value is identical to string    |
| `not_equals`      | field value is not identical          |
| `starts_with`     | field value begins with prefix        |
| `not_starts_with` | field value does not begin with prefix|

All operators compare the field value as a plain string. Exactly one operator
must be present per `when:` block; zero or multiple operators is a parse error.

### Interaction with `on_step_fail`

The `on_step_fail.step` field must name an unconditional step (a step without
`when:`). Naming a conditionally-skipped step in `on_step_fail` is undefined
behavior and is not validated at parse time.

