---
status: draft
created: 2026-02-24
resolve_before_implementing:
  - "Depends on sequence-declarations — implement that first"
  - "Skip-if-writes-exist vs explicit progress tracking?"
  - "Should skipped steps log a message or be silent?"
---

# Resume failed sequences from the last completed step

## Story

When a sequence fails mid-way (e.g., implement succeeds but verify fails), re-
running the sequence currently re-executes all steps from the beginning. For
expensive steps like `implement` (which invokes Claude Code), this wastes time
and money. This slice makes sequences resumable: on re-run, steps whose
`writes:` state already exists in the run-state store are skipped.

## Coupling to Sequences

This slice only makes sense if sequences are a first-class primitive (the
`sequence-declarations` slice). Without sequences, the user is the orchestrator
and skips completed steps manually by running individual commands.

The core mechanism — "check if this step's writes already exist before running
it" — is simple. The design questions are about UX:

- Should skipped steps print a message? ("Skipping implement: session state
  already exists")
- Should there be a `--force` flag to re-run all steps regardless?
- What about steps with no `writes:` declaration (like `verify`)? They can't be
  skipped because there's no state to check. This means `verify` always re-runs,
  which is actually correct behavior.

## Approach (tentative)

Before executing each step in a sequence, check whether all of the step's
`writes:` entries already exist in `.graft/run-state/`. If they do, skip the
step with a log message. Steps with no `writes:` always execute.

The `--force` flag (or `--from-scratch`) clears relevant run-state before
starting.

## Acceptance Criteria

- A sequence re-run skips steps whose writes state exists
- Steps without writes declarations always execute
- Skipped steps produce a visible log message
- `--force` re-runs all steps regardless of existing state
- `cargo test` passes with no regressions

## Steps

TBD — implement sequence-declarations first.
