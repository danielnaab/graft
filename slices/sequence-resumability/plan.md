---
status: accepted
created: 2026-02-24
updated: 2026-02-26
depends_on: [sequence-declarations]
note: >
  Not superseded by sequence-retry. sequence-retry handles automatic retry
  within a single run (verify fails → recovery → retry). This slice handles
  crash recovery: if a sequence is killed mid-run, re-running should restart
  from the interrupted step rather than from the beginning.
---

# Resume failed sequences from the last completed step

## Story

When a sequence is killed mid-run (timeout, OOM, Ctrl+C), re-running it
currently re-executes all steps from the beginning. For expensive steps like
`implement` (which invokes Claude Code), this wastes time and money. This slice
makes sequences resumable: on re-run, steps that already completed are skipped
and execution restarts from the interrupted step.

## Resolved Design Questions

### on_step_fail interaction

When a sequence is killed during a retry (`phase: "retrying"`), treat it the
same as `phase: "running"`: restart from `step_index`. The retry loop restarts
with a fresh iteration count (losing progress within the retry is acceptable
and simpler than persisting iteration state across process boundaries).

### Skipped step messaging

Emit to stderr: `↷ Skipping <step-name> (already completed)`

### --force flag

Deferred. Users can delete `.graft/run-state/sequence-state.json` to force a
fresh start. Adding a `--force` flag requires plumbing through the sequence arg
interface; out of scope for this slice.

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

1. Read `.graft/run-state/sequence-state.json` (if absent: fresh run, all steps execute).
2. If the recorded sequence name doesn't match the current sequence: fresh run.
3. If `phase` is `"complete"` or `"failed"`: fresh run (sequence already terminated cleanly).
4. If `phase` is `"running"` or `"retrying"` at `step_index` N:
   - Steps 0..N-1 are skipped with a log message.
   - Execution restarts from step N.

## Why Not "Skip If Writes Exist"

An earlier version of this plan proposed skipping steps whose `writes:` state
files already exist. This approach is broken:

- **Fails for verify**: `verify` declares `writes: [verify]`, so `verify.json`
  exists after both successful and failed runs. The writes-check would
  incorrectly skip a failed verify on re-run.
- **Ambiguous on partial writes**: a step may write multiple state files and be
  killed after writing some but not all of them.
- **Silent data hazard**: stale state from a previous run looks identical to
  fresh state.

`sequence-state.json` tracks completion explicitly at the step level.

## Acceptance Criteria

- A re-run of a sequence that was interrupted at step N skips steps 0..N-1
- Each skipped step prints `↷ Skipping <step-name> (already completed)` to stderr
- A sequence with no existing `sequence-state.json` runs all steps normally
- A sequence whose `sequence-state.json` records a different sequence name runs all steps normally
- A sequence whose `sequence-state.json` has `phase: complete` or `phase: failed` runs all steps normally
- `phase: retrying` is treated the same as `phase: running` for resumption purposes
- `cargo test` passes with no regressions

## Steps

- [ ] Spec: add `docs/specifications/graft/sequence-execution.md` with Gherkin
      scenarios covering normal execution, retry, resumability, and the
      sequence-state.json schema (TDD — spec before code)
- [ ] Tests: add unit tests in `crates/graft-engine/src/sequence.rs` for
      resume-from-interrupted-step behavior; tests must fail before
      implementation (red phase)
- [ ] Implement: in `execute_sequence()`, read `sequence-state.json` on entry;
      compute `resume_from` index; skip steps before it with the prescribed
      message; handle `phase: retrying` same as `phase: running`
- [ ] Verify: run full test suite; confirm all new tests pass and no regressions;
      check spec matches implementation
