---
status: done
created: 2026-02-26
depends_on: [step-level-timeouts, verify-captures-state]
---

# Conditional step execution based on run-state values

## Story

Steps in a sequence run unconditionally. Some workflows need steps that run only when
certain conditions are true — either as **precondition gates** (skip an expensive step
when preconditions are unmet) or as **optional enrichment** (run a step only when
enrichment is relevant). Examples:

- Skip `spec-check` when `session.json` has no `baseline_sha` (no diff to check)
- Run a notification step only when `review.json`'s `verdict` field is `"concerns"`
- Skip a type-generation step when the schema file hasn't changed

Conditional steps allow sequence definitions to skip or include steps based on values
in run-state files.

**Note**: Conditional steps do NOT solve targeted-recovery sequences (e.g., run
lint-fix when lint fails, then re-verify). That pattern requires a multi-step recovery
mechanism — a separate design problem not addressed here.

## Approach

Extend `StepDef` (from the step-level-timeouts slice) with an optional `when` field:

```rust
struct WhenCondition {
    state: String,            // run-state file name, e.g. "verify"
    field: String,            // JSON field name, e.g. "lint"
    // Exactly one of:
    equals: Option<String>,
    not_equals: Option<String>,
    starts_with: Option<String>,
    not_starts_with: Option<String>,
}
```

In `execute_sequence()`, before executing a step that has a `when` condition:
1. Read `<run_state_dir>/<state>.json`
2. Extract the specified field as a string value
3. Evaluate the condition against the string
4. If false: skip the step with message
   `↷ Skipping <step> (condition not met: <state>.<field>)`
5. If the state file does not exist or the field is absent: condition evaluates to
   false (step skipped); this is logged clearly

Conditions only reference run-state files written by previous steps in the same
sequence. No parse-time validation of ordering is performed — if a step references a
state file that a prior step hasn't written yet, it silently skips.

**Interaction with `on_step_fail`**: The `on_step_fail.step` field must name an
unconditional step (a step without `when`). Naming a conditional step that was skipped
in `on_step_fail` is undefined behavior and should be documented as unsupported.

The spec is written first (TDD — spec before code).

## Acceptance Criteria

- A step with `when: {state: review, field: verdict, equals: "concerns"}` executes
  only when `review.json`'s `verdict` field equals `"concerns"`
- A step with `when: {state: review, field: verdict, not_equals: "pass"}` executes
  only when `verdict` is not `"pass"`
- A step with `when: {state: session, field: baseline_sha, not_starts_with: ""}` can
  be used as a precondition gate (execute only when `baseline_sha` is non-empty)
- A step with `when: {state: session, field: slice, starts_with: "slices/"}` executes
  only when the slice path starts with `"slices/"`
- A step with no `when` always executes (existing behavior preserved)
- If the referenced state file does not exist, the condition is false and the step
  is skipped with a clear log message naming the missing state file
- If the referenced field does not exist in the JSON, the condition is false and the
  step is skipped
- Exactly one condition operator must be present; a step with zero or multiple
  operators is rejected at parse time with a validation error
- The spec (`graft-yaml-format.md`, `sequence-execution.md`) is written before
  implementation
- `cargo test` passes including unit tests for each operator, missing-file case,
  and missing-field case
- `cargo clippy -- -D warnings && cargo fmt --check` passes

## Steps

- [x] **Spec: add `when` condition syntax to `graft-yaml-format.md` and
  `sequence-execution.md`**
  - **Delivers** — unambiguous spec for condition syntax before any code is written
  - **Done when** — `graft-yaml-format.md` documents the `when` field on step objects:
    lists the four operators with types and semantics; documents the missing-file and
    missing-field behavior; shows worked examples with `verify.json`; validates that
    exactly one operator must be present; `sequence-execution.md` adds Gherkin scenarios
    for: condition true (step runs), condition false (step skipped), missing state file
    (step skipped with message), missing field (step skipped); spec complete before
    implementation begins
  - **Files** — `docs/specifications/graft/graft-yaml-format.md`,
    `docs/specifications/graft/sequence-execution.md`

- [x] **Implement `WhenCondition` in `config.rs` and condition evaluation in
  `sequence.rs`**
  - **Delivers** — conditional step execution based on run-state values
  - **Done when** — `graft-common/src/config.rs` adds `WhenCondition` struct with
    validation that exactly one operator is set (parse error if zero or multiple);
    `StepDef` gains `when: Option<WhenCondition>`; `graft-engine/src/sequence.rs`
    evaluates conditions before each step, reads the state JSON file, extracts the
    field, applies the operator, and skips or executes accordingly; unit tests are
    written before implementation (red phase confirmed) covering all four operators,
    missing-file, missing-field, and no-condition (always runs); all tests pass after
    implementation; `cargo test && cargo clippy -- -D warnings && cargo fmt --check`
    passes
  - **Files** — `crates/graft-common/src/config.rs`,
    `crates/graft-engine/src/sequence.rs`
