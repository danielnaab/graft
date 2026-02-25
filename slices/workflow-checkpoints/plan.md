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
args, message, created_at}`. The `args` field embeds the full sequence arg map
(e.g. `{"slice": "slices/my-feature"}`), so `approve` and `reject` can read the
slice from the checkpoint file itself — they require no positional arguments.

`approve` transitions the checkpoint to `{phase: "approved"}` (atomic `.tmp`+`mv`).
`reject` transitions it to `{phase: "rejected"}`. Neither deletes the file — phase
history is preserved, and grove's visual treatment (the `!` overlay) only fires for
`awaiting-review`. The next `implement-verified` run overwrites `checkpoint.json`
with a fresh `awaiting-review` entry.

**`message` field**: the executor writes a generic message ("Sequence `<name>`
complete.") since it does not have retry iteration count available at checkpoint-write
time — iteration state lives in the retry executor layer, not the base sequence layer.
The grove overlay displays whatever `message` is in the file. A richer message
(including iteration count) is deferred to a future slice that composes the retry
and checkpoint layers more tightly.

**`created_at` field**: requires an ISO timestamp. If `chrono` is not already a
dependency of `graft-engine`, add it to `Cargo.toml`.

Grove's existing run-state loader picks up `checkpoint.json` automatically — no Rust
changes are required for basic visibility. The `implement-verified` sequence (from
`sequence-retry`) gains `checkpoint: true` to wire the complete
implement-verify-review cycle.

## Acceptance Criteria

- After `graft run software-factory:implement-verified <slice>` succeeds,
  `checkpoint.json` appears in grove's Run State section with `phase: awaiting-review`
  and an `args` field containing the sequence args
- `graft run software-factory:approve` (no args) reads checkpoint.json, transitions
  phase to `approved`; grove Run State shows the entry as normal JSON (not `!`)
- `graft run software-factory:reject` (no args) transitions phase to `rejected`
- Both `approve` and `reject` exit non-zero with `"No pending checkpoint"` when
  `checkpoint.json` is absent or `phase` is not `awaiting-review`
- Sequences without `checkpoint: true` do NOT write `checkpoint.json`
- `cargo test` passes with no regressions

## Steps

- [ ] **Add `checkpoint` flag to SequenceDef; sequence executor writes checkpoint.json on success**
  - **Delivers** — sequences can declare a human review gate; checkpoint.json is
    observable in grove after successful completion
  - **Done when** — `SequenceDef` gains `checkpoint: Option<bool>` (treated as false
    when absent); when a sequence with `checkpoint: true` exits all steps with
    status 0, the executor writes `$GRAFT_STATE_DIR/checkpoint.json` with fields
    `{phase: "awaiting-review", sequence: <name>, args: {<name>: <value>, ...},
    message: "Sequence <name> complete.", created_at: <ISO8601 timestamp>}`; the
    `args` field is the full map of resolved sequence arg values; if `chrono` is
    not already a `graft-engine` dependency, add `chrono = {version = "...", features
    = ["serde"]}` to `Cargo.toml`; a unit test asserts `checkpoint.json` is written
    with the correct `args` map on sequence success when the flag is set; a test
    asserts it is NOT written when `checkpoint` is absent or false; a test asserts
    it is NOT written when the sequence exits non-zero
  - **Files** — `crates/graft-common/src/config.rs`,
    `crates/graft-engine/src/sequence.rs`, `Cargo.toml` (if chrono absent)

- [ ] **Write approve.sh and reject.sh; register commands; wire checkpoint: true on implement-verified**
  - **Delivers** — the complete implement-verify-review cycle is live: humans can
    approve or reject a pending checkpoint from CLI, Claude Code, or grove's Commands
    section; `implement-verified` is the one-command development cycle
  - **Done when** — `approve.sh` reads `$GRAFT_STATE_DIR/checkpoint.json`, verifies
    `phase == "awaiting-review"`, and atomically writes the existing content merged
    with `{phase: "approved"}` via `jq '. + {phase:"approved"}' ... > .tmp && mv`
    pattern; it exits 0 and prints `"Checkpoint approved."`; `reject.sh` does the
    same with `{phase: "rejected"}` and prints `"Checkpoint rejected. Resume the
    session with feedback (graft run resume <slice>) or re-implement
    (graft run implement-verified <slice>)."`; both exit 1 with `"No pending
    checkpoint"` when `checkpoint.json` is absent or `phase != "awaiting-review"`;
    `graft.yaml` declares `approve` and `reject` with `reads: [checkpoint]`,
    `writes: [checkpoint]`, a description, and NO positional args (`writes:
    [checkpoint]` on `approve` is intentional — it transitions state, even though
    it does not produce new content); the `implement-verified` sequence in
    `software-factory/graft.yaml` has `checkpoint: true`; end-to-end manual test:
    run `graft run software-factory:implement-verified slices/<slug>` → verify
    passes → `checkpoint.json` appears in grove Run State with
    `phase: awaiting-review`; run `graft run software-factory:approve` → checkpoint
    transitions to `phase: approved`, grove Run State shows normal JSON entry;
    `cargo test` passes
  - **Files** — `.graft/software-factory/scripts/approve.sh` (new),
    `.graft/software-factory/scripts/reject.sh` (new),
    `.graft/software-factory/graft.yaml`
