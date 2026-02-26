---
status: draft
created: 2026-02-26
depends_on: [step-level-timeouts, verify-captures-state]
---

# Conditional step execution based on run-state values

## Story

The current `on_step_fail` retry mechanism is coarse: one recovery command handles
all failures of a single step, regardless of what kind of failure occurred. Real
failure modes need targeted recovery: a lint failure needs a different fix than a
test failure, and a type error needs different recovery than a missing import.
Conditional steps allow sequence definitions to skip or include steps based on values
in run-state files, enabling targeted recovery sequences:

```yaml
steps:
  - implement
  - verify
  - name: lint-fix
    when: {state: verify, field: lint, not_starts_with: "OK"}
  - name: test-fix
    when: {state: verify, field: tests, not_starts_with: "OK"}
```

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

The spec is written first (TDD — spec before code).

## Acceptance Criteria

- A step with `when: {state: verify, field: lint, not_starts_with: "OK"}` executes
  only when `verify.json`'s `lint` field does not start with "OK"
- A step with `when: {state: verify, field: lint, starts_with: "OK"}` executes only
  when it does
- A step with `when: {state: verify, field: tests, equals: "OK. 0 tests"}` executes
  only when tests equals exactly that string
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

- [ ] **Spec: add `when` condition syntax to `graft-yaml-format.md` and
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

- [ ] **Implement `WhenCondition` in `config.rs` and condition evaluation in
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
