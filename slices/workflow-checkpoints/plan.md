---
status: accepted
created: 2026-02-24
depends_on: [sequence-retry]
---

# Human checkpoint gate: approve/reject after sequence success

## Story

When a sequence step completes with all checks passing, there is currently no way
for a human to review the result before the workflow continues to the next step.
This slice adds a checkpoint mechanism: sequences that declare `checkpoint: true`
write a `checkpoint.json` run-state file on success, and two new commands —
`approve` and `reject` — give humans a clear control plane from both CLI and grove.

## Approach

Extend `SequenceDef` with an optional `checkpoint: bool` (default false). When a
sequence with `checkpoint: true` exits all steps successfully, the executor writes
`checkpoint.json` to `$GRAFT_STATE_DIR` with `{phase: "awaiting-review", sequence,
slice, message, created_at}`. Grove's existing run-state loader picks this up
automatically — no Rust changes are required for basic visibility.

Two new shell scripts — `approve.sh` and `reject.sh` — read and clear checkpoint.json.
Both are registered as commands in `software-factory/graft.yaml` with `reads: [checkpoint]`
and `writes: [checkpoint]`. The `implement-verified` sequence (from `sequence-retry`)
gains `checkpoint: true` to wire the complete implement-verify-review cycle.

Both scripts use a `.tmp` + `mv` pattern for atomic writes and fail gracefully when
no pending checkpoint exists.

## Acceptance Criteria

- After `graft run software-factory:implement-verified <slice>` succeeds,
  `checkpoint.json` appears in grove's Run State section with `phase: awaiting-review`
- `graft run software-factory:approve <slice>` removes `checkpoint.json`; the entry
  disappears from grove Run State on the next refresh
- `graft run software-factory:reject <slice>` writes `{phase: "rejected"}` back to
  `checkpoint.json`; the entry updates in grove Run State
- Both `approve` and `reject` exit non-zero with a clear message when no pending
  checkpoint exists (`"No pending checkpoint for <slice>"`)
- Sequences without `checkpoint: true` do NOT write `checkpoint.json`
- `cargo test` passes with no regressions

## Steps

- [ ] **Add `checkpoint` flag to SequenceDef; sequence executor writes checkpoint.json on success**
  - **Delivers** — sequences can declare a human review gate; checkpoint.json is
    observable in grove after successful completion
  - **Done when** — `SequenceDef` gains `checkpoint: Option<bool>` (treated as false
    when absent); when a sequence with `checkpoint: true` exits all steps with status 0,
    the executor writes `$GRAFT_STATE_DIR/checkpoint.json` with fields
    `{phase: "awaiting-review", sequence: <name>, slice: <arg>, message: "Step N
    complete. Verify passed.", created_at: <ISO timestamp>}`; the `slice` field is
    populated from the first positional arg if present; a unit test asserts
    `checkpoint.json` is written on sequence success when the flag is set; a test
    asserts it is NOT written when `checkpoint` is absent or false
  - **Files** — `crates/graft-common/src/config.rs`,
    `crates/graft-engine/src/sequence.rs`

- [ ] **Write approve.sh and reject.sh; register approve/reject commands in software-factory**
  - **Delivers** — humans can approve or reject a pending checkpoint from CLI, Claude
    Code, or grove's Commands section
  - **Done when** — `approve.sh` reads `$GRAFT_STATE_DIR/checkpoint.json`, verifies
    `phase == "awaiting-review"`, removes the file, and exits 0; prints
    `"Checkpoint approved. Run implement-verified again for the next step."`;
    `reject.sh` does the same but writes `{phase: "rejected"}` atomically via
    `.tmp` + `mv` and prints `"Checkpoint rejected. Re-run implement-verified to
    retry this step."`; both exit 1 with `"No pending checkpoint"` when
    `checkpoint.json` is absent or phase is not `awaiting-review`; `graft.yaml`
    declares `approve` and `reject` with `reads: [checkpoint]`, `writes: [checkpoint]`,
    `description`, and a positional `slice` arg using `options_from: slices`
  - **Files** — `.graft/software-factory/scripts/approve.sh` (new),
    `.graft/software-factory/scripts/reject.sh` (new),
    `.graft/software-factory/graft.yaml`

- [ ] **Set checkpoint: true on the implement-verified sequence**
  - **Delivers** — `graft run software-factory:implement-verified <slice>` is the
    complete one-command feature development cycle: implement → verify-with-retry →
    checkpoint awaiting review
  - **Done when** — the `implement-verified` sequence in `software-factory/graft.yaml`
    has `checkpoint: true`; end-to-end test: run
    `graft run software-factory:implement-verified slices/<slug>` → verify passes →
    `checkpoint.json` appears in grove Run State; run
    `graft run software-factory:approve slices/<slug>` → checkpoint disappears;
    `cargo test` passes
  - **Files** — `.graft/software-factory/graft.yaml`
