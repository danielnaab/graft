---
status: draft
created: 2026-02-24
updated: 2026-02-26
depends_on: [sequence-declarations]
note: >
  Not superseded by sequence-retry. sequence-retry handles automatic retry
  within a single run (verify fails → recovery → retry). This slice handles
  crash recovery: if a sequence is killed mid-run, re-running should restart
  from the interrupted step rather than from the beginning.
resolve_before_implementing:
  - "How does resumability interact with on_step_fail retry? If killed during
    retry iteration 2 of 3, should resume restart the recovery step, restart
    the failed step, or restart the whole sequence?"
  - "Should skipped steps log a message or be silent?"
  - "Should there be a --force flag to re-run all steps regardless?"
---

# Resume failed sequences from the last completed step

## Story

When a sequence is killed mid-run (timeout, OOM, Ctrl+C), re-running it
currently re-executes all steps from the beginning. For expensive steps like
`implement` (which invokes Claude Code), this wastes time and money. This slice
makes sequences resumable: on re-run, steps that already completed are skipped
and execution restarts from the interrupted step.

## Coupling to Sequences

This slice only makes sense if sequences are a first-class primitive (the
`sequence-declarations` slice, now done). Without sequences, the user is the
orchestrator and skips completed steps manually by running individual commands.

## Approach

Use `sequence-state.json` as the authoritative resumption checkpoint. The
sequence executor already writes this file atomically before each step with:

```json
{
  "sequence": "<name>",
  "step":     "<step-name>",
  "step_index": 1,
  "step_count": 2,
  "phase": "running | retrying | complete | failed"
}
```

On re-run of a sequence:

1. Read `.graft/run-state/sequence-state.json`.
2. If absent or `phase == "complete"`: run all steps normally (fresh run).
3. If `phase == "failed"`: the sequence already terminated cleanly — run
   normally (sequence executor already handles this case via exit code).
4. If `phase == "running"` for step N: step N was interrupted. Skip steps
   0..N-1 and restart from step N.

Steps that are skipped emit a visible message:
```
↷ Skipping implement (already completed)
```

## Why Not "Skip If Writes Exist"

An earlier version of this plan proposed skipping steps whose `writes:` state
files already exist. This approach is broken:

- **Fails for verify**: `verify` declares `writes: [verify]` (added in
  slice 1), so `verify.json` exists after both successful and failed runs.
  The writes-check would incorrectly skip a failed verify on re-run, leaving
  the failure result in place.
- **Ambiguous on partial writes**: a step may write multiple state files and be
  killed after writing some but not all of them.
- **Silent data hazard**: stale state from a previous run looks identical to
  fresh state; no way to detect staleness without a timestamp comparison.

The `sequence-state.json` approach avoids all of these: it tracks completion
explicitly at the step level, written atomically before execution begins.

## on_step_fail Interaction

When `on_step_fail` is configured and the sequence is killed during a retry
iteration, the resumption point is ambiguous. Two options:

- **Restart the whole sequence**: simplest; may re-run the expensive initial
  step unnecessarily.
- **Restart from the failing step**: resume the retry from where it was
  interrupted; requires storing the retry iteration count in
  `sequence-state.json` (the `iteration` field already exists).

Resolve before implementing. Initial recommendation: restart from the failing
step (not the recovery step), using the stored `step_index`. The recovery step
will re-run naturally as part of the retry loop.

## Acceptance Criteria

- A sequence re-run reads `sequence-state.json` and skips steps before the
  interrupted step
- Skipped steps print `↷ Skipping <step> (already completed)` to stderr
- Steps not yet reached (step_index > current) run normally
- A sequence with no existing `sequence-state.json` runs all steps normally
- `--force` flag (or equivalent) bypasses resumability and runs all steps
- `cargo test` passes with no regressions

## Steps

TBD — resolve the `on_step_fail` interaction question before implementing.
