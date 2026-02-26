---
status: done
created: 2026-02-24
depends_on: [sequence-declarations, verify-captures-state]
---

# Add retry semantics to sequences: native implement-verify loop

## Story

When a step in a sequence fails — particularly `verify` after `implement` — there
is currently no way to automatically run a recovery command and retry without
writing a custom bash loop. This slice adds `on_step_fail:` to sequence
declarations so graft natively handles the retry cycle, replacing the one-off
`ralph.sh` script with a composable primitive applicable to any workflow.

## Approach

Extend `SequenceDef` with an optional `on_step_fail` block that names the failing
step, a recovery command to run before retrying, and a max retry count. The
sequence executor already runs steps in order and detects failure; this slice
extends it to — on a designated step's failure — run the recovery command (which
can read and respond to the current state, e.g., resuming Claude with the verify
failure context) and then retry the failing step, up to `max` times. The iteration
count is written to `sequence-state.json` so grove shows which retry is in progress.

The software-factory gets an `implement-verified` sequence wired directly in its
`graft.yaml`: steps `[implement, verify]`, failing step `verify`, recovery command
`resume`, max `3`. This replaces the ralph loop for the standard feature development
cycle and serves as the canonical end-to-end test of the primitive.

`verify.json` (from the `verify-captures-state` slice) is available at
`$GRAFT_STATE_DIR/verify.json`. Step 4 of this slice updates `resume.sh` to
read it automatically and inject a structured failure summary into the Claude
`--resume` call — no manual catting required.

## Acceptance Criteria

- A sequence with `on_step_fail:` configured retries the named step after running
  the recovery command; the step's output (stdout/stderr) is visible each iteration
- After `max` retries without success, the sequence exits non-zero with a clear
  message: `"Step 'verify' failed after 3 retries"`
- `sequence-state.json` reflects the current phase at all times: `running`,
  `retrying` (with `iteration` count), `complete`, or `failed`
- `graft run software-factory:implement-verified slices/<slug>` implements the next
  unchecked step of the given slice and retries verify up to 3 times on failure
- If verify passes on the first attempt, the sequence completes without any retries
- If `on_step_fail.step` names a step not in the sequence's `steps` list, it is
  rejected at parse time
- `cargo test` passes with no regressions

## Steps

- [ ] **Extend SequenceDef with on_step_fail retry config**
  - **Delivers** — sequences can declare retry semantics in graft.yaml
  - **Done when** — `SequenceDef` gains an optional `on_step_fail: OnStepFail`
    field where `OnStepFail` has `step: String`, `recovery: String`, and
    `max: u32` (default 3); `max` means the number of retry attempts after the
    initial failure — `max: 3` allows 1 initial run + 3 retries = 4 total step
    executions; graft-common parses this from graft.yaml; validation rejects an
    `on_step_fail.step` value that isn't in the sequence's `steps` list; validation
    rejects an `on_step_fail.recovery` that doesn't name an existing command in
    the same graft.yaml (local commands only, not dep-qualified names); a unit test
    asserts a valid `on_step_fail` block parses correctly and an invalid step name
    produces an error
  - **Files** — `crates/graft-common/src/config.rs`

- [ ] **Implement retry logic in the sequence executor**
  - **Delivers** — a failing verify (or any named step) triggers the retry cycle
    automatically
  - **Done when** — when a step named in `on_step_fail.step` exits non-zero, the
    executor runs `on_step_fail.recovery` command with the same args as the
    sequence (not the failed step's args — `verify` has no args, but `resume`
    needs `slice`) and retries the failed step; if the recovery command itself
    exits non-zero, the sequence aborts immediately with a clear error message
    (`"Recovery command '<name>' failed (exit N); aborting"`) — a broken
    recovery indicates misconfiguration, not a transient failure, so further
    retries are skipped; `sequence-state.json` is updated to
    `{phase: "retrying", step, iteration}` before each retry attempt; after `max`
    retries the sequence sets `{phase: "failed", step, iterations_attempted}` and
    exits non-zero with `"Step '<name>' failed after <max> retry attempts (<max+1>
    total runs)"`; **test fixture**: counter file in a temp dir — step script reads
    counter, increments, exits 1 if counter ≤ N, exits 0 otherwise; a test runs a
    sequence where the check step fails twice then succeeds: assert `iteration == 2`,
    recovery ran twice, sequence exits 0; a test asserts clean failure after `max`
    retries; a test asserts recovery-command failure aborts immediately without
    further retries
  - **Files** — `crates/graft-engine/src/sequence.rs`

- [ ] **Wire implement-verified sequence in software-factory**
  - **Delivers** — `graft run software-factory:implement-verified <slice>` is the
    new native ralph loop — no bash loop script required
  - **Done when** — `software-factory/graft.yaml` declares an `implement-verified`
    sequence with `steps: [implement, verify]`, `on_step_fail: {step: verify,
    recovery: resume, max: 3}`, and the same `slice` arg as `implement`; running
    `graft run software-factory:implement-verified slices/<any-slice>` from the
    graft repo root implements the next unchecked step and verifies, retrying via
    resume if verify fails; `sequence-state.json` appears in grove's Run State
    section showing current phase; the old `ralph.sh` script in the notes folder
    is no longer the primary workflow path (it can remain as a reference)
  - **Files** — `.graft/software-factory/graft.yaml`

- [ ] **Update resume.sh to inject verify.json failure context**
  - **Delivers** — Claude receives a structured failure summary when resumed after
    a verify failure, rather than resuming with no context about what went wrong
  - **Done when** — `resume.sh` checks whether `$GRAFT_STATE_DIR/verify.json`
    exists and contains any failing field (any value that is not `"OK"` and does
    not start with `"OK"`); if failures are present, it assembles a failure prompt
    summarising the failing checks (FORMAT / LINT / TESTS / SMOKE sections);
    **assumption to verify before coding**: does `claude --resume "$session_id" -p "$failure_prompt"` work with both flags simultaneously? If yes, use that form; if
    `--resume` ignores `-p`, use stdin piping:
    `printf '%s' "$failure_prompt" | claude --resume "$session_id" --dangerously-skip-permissions`;
    if no failures are found or `verify.json` is absent, resumes without extra
    prompt (preserving current behaviour for direct
    `graft run software-factory:resume` calls); both code paths exit with Claude's
    exit code; end-to-end manual test: break a test, run `implement-verified`,
    observe Claude's resume turn references the test failure output
  - **Files** — `.graft/software-factory/scripts/resume.sh`
