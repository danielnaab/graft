---
status: done
created: 2026-02-26
depends_on: [sequence-declarations]
---

# Per-step timeout declarations in sequence definitions

## Story

Sequence steps currently have no individual timeouts. A hung `implement` step — a
Claude subprocess blocked waiting for something, or an infinite loop — will run
forever with no recourse from within the sequence. There is no way to declare that
`implement` should fail after 600 seconds or that `verify` should fail after 180
seconds. This makes `implement-verified` unreliable for unattended use: a single
stuck step consumes the entire session indefinitely.

## Approach

Change `SequenceDef`'s steps from `Vec<String>` to `Vec<StepDef>` where `StepDef`
is normalized from a `#[serde(untagged)]` enum supporting both forms:

```yaml
# Bare string form (existing, unchanged)
steps:
  - implement
  - verify

# Object form (new)
steps:
  - name: implement
    timeout: 600
  - name: verify
    timeout: 180

# Mixed (also supported)
steps:
  - implement
  - name: verify
    timeout: 180
```

In Rust:
```rust
#[serde(untagged)]
enum StepEntry {
    Simple(String),
    Full(StepDef),
}

struct StepDef {
    name: String,
    timeout: Option<u64>,  // seconds
}
```

`SequenceDef.steps` is stored as `Vec<StepDef>` after parsing (bare strings normalize
to `StepDef { name, timeout: None }`).

In `execute_sequence()`, pass `step_def.timeout` as the timeout to the command
execution path. Steps with `timeout: None` use the existing no-timeout behavior.

Update the spec files before implementation (TDD).

## Acceptance Criteria

- `steps: [implement, verify]` (bare strings) works exactly as before
- `steps: [{name: implement, timeout: 600}, {name: verify, timeout: 180}]` works
- Mixed forms in the same sequence work: `steps: [implement, {name: verify, timeout: 180}]`
- A step exceeding its timeout fails with a clear message; the sequence records
  `phase: "failed"` in `sequence-state.json` at that step
- Steps with no timeout use the existing no-timeout behavior
- `docs/specifications/graft/graft-yaml-format.md` documents the step object form
  before implementation
- New unit tests cover: bare string parse, object form parse, mixed form, timeout
  enforcement (step exceeds timeout → sequence fails)
- `cargo test` passes with no regressions
- `cargo clippy -- -D warnings && cargo fmt --check` passes

## Steps

- [x] **Spec: add step object syntax to `graft-yaml-format.md` and
  `sequence-execution.md`**
  - **Delivers** — unambiguous spec for step object syntax before any code is written
  - **Done when** — `graft-yaml-format.md` documents the step object form in the
    `sequences:` section, showing bare string, full object, and mixed examples; the
    `timeout` field is documented with type (`integer`, seconds), default (none),
    and behavior (step killed and sequence fails on timeout); `sequence-execution.md`
    adds Gherkin scenarios for step-with-timeout succeeding within budget and failing
    after timeout; spec is complete before implementation begins
  - **Files** — `docs/specifications/graft/graft-yaml-format.md`,
    `docs/specifications/graft/sequence-execution.md`

- [x] **Implement `StepEntry`/`StepDef` in `config.rs` and pass timeout in
  `sequence.rs`**
  - **Delivers** — sequences can declare per-step timeouts; the executor enforces them
  - **Done when** — `graft-common/src/config.rs` defines `StepEntry` as an untagged
    enum and `StepDef { name, timeout }`; `SequenceDef.steps` is `Vec<StepDef>`
    (normalized after parse); `parse_sequences_from_str` correctly handles all three
    forms; `graft-engine/src/sequence.rs`'s `execute_sequence` reads `step_def.timeout`
    and threads it through the full call chain: `execute_sequence` →
    `execute_command_by_name` → `ProcessConfig { timeout_seconds: step_def.timeout }`
    (trace the exact field name and call sites before coding to confirm no intermediate
    step silently drops the value);
    unit tests are written before implementation (red phase confirmed) covering all
    three parse forms and timeout enforcement; all tests pass after implementation;
    `cargo test && cargo clippy -- -D warnings && cargo fmt --check` passes
  - **Files** — `crates/graft-common/src/config.rs`,
    `crates/graft-engine/src/sequence.rs`
