---
status: draft
created: 2026-02-26
depends_on: [workflow-checkpoints, context-snapshot]
---

# Structured self-review step before human checkpoint (Reflexion pattern)

## Story

After implementation passes mechanical verification (format, lint, tests), the
implementation goes straight to a human checkpoint without any semantic review. The
human must read the diff cold, without a pre-digested summary of what was done, whether
the acceptance criteria were met, and what concerns exist. A `review` step runs Claude
against the diff and the slice's acceptance criteria before the checkpoint, producing a
structured verdict. This surfaces issues so Claude catches obvious problems before the
human has to, and gives the human a concise brief instead of a raw diff.

This is the "Reflexion" pattern: the agent self-critiques its output before handing off
to a human, giving 11–21% improvement in task success rates.

## Approach

New `scripts/review.sh`:
1. Reads `baseline_sha` from `context-snapshot.json` (fallback: `HEAD~1` if absent)
2. Reads slice path from `session.json`
3. Reads the "Acceptance Criteria" section from the slice's `plan.md`
4. Gets `git diff <baseline_sha>` to see the full implementation diff
5. Pipes diff + acceptance criteria to `claude -p --dangerously-skip-permissions`
   with a structured review prompt requesting JSON output
6. Writes `review.json`:
   ```json
   {
     "verdict": "pass | concerns | fail",
     "summary": "...",
     "criteria": [
       {"criterion": "...", "status": "met | unmet | partial", "evidence": "..."}
     ],
     "concerns": ["..."]
   }
   ```

Add `review` command to `graft.yaml` with `reads: [session, context-snapshot]`,
`writes: [review]`.

Add `implement-reviewed` sequence: `steps: [implement, verify, review]` with
`on_step_fail: {step: verify, recovery: resume, max: 3}` and `checkpoint: true`.

Modify `write_checkpoint_json` in `crates/graft-engine/src/sequence.rs` to read
`review.json` from the run-state dir when present and incorporate `verdict` and
`concerns` into the `message` field.

## Acceptance Criteria

- `graft run software-factory:review` produces `review.json` with `verdict`,
  `criteria`, `concerns`, and `summary` fields
- `verdict: "pass"` when all criteria appear met and no concerns
- `verdict: "concerns"` when criteria are met but minor concerns exist
- `verdict: "fail"` when one or more criteria are unmet
- Running `review` without `session.json` exits 1 with a clear error
- Running `review` without a slice plan exits 1 with a clear error
- `implement-reviewed` sequence runs `implement → verify → review → checkpoint`
  with retry on verify failure (up to 3 times) before reaching review
- `checkpoint.json` message incorporates the review verdict:
  `"Sequence complete. Review: <verdict>."` with concerns appended if any
- Running `review` when `context-snapshot.json` is absent falls back to `git diff HEAD~1`
- `cargo test && cargo clippy -- -D warnings && cargo fmt --check` passes

## Steps

- [ ] **Write `scripts/review.sh` and add `review` command to `graft.yaml`**
  - **Delivers** — standalone `review` command produces a structured self-review against
    acceptance criteria
  - **Done when** — `review.sh` reads `baseline_sha` from `context-snapshot.json`
    or falls back to `HEAD~1`; reads slice path from `session.json`; reads the
    "Acceptance Criteria" section from the slice's `plan.md` (lines between
    `## Acceptance Criteria` and the next `##` heading); constructs a prompt with the
    diff and criteria, requesting JSON output with `verdict`, `criteria`, `concerns`,
    `summary` fields; pipes to `claude -p --dangerously-skip-permissions`; writes
    `$GRAFT_STATE_DIR/review.json`; `graft.yaml` adds `review` command with
    `reads: [session, context-snapshot]`, `writes: [review]`; manual test: run
    implement on a slice, run review, inspect `review.json`
  - **Files** — `.graft/software-factory/scripts/review.sh`,
    `.graft/software-factory/graft.yaml`

- [ ] **Add `implement-reviewed` sequence and incorporate review verdict into
  checkpoint message**
  - **Delivers** — the full implement → verify → review → human-checkpoint cycle as a
    single command; checkpoint message summarizes the review verdict for the human
  - **Done when** — `graft.yaml` declares `implement-reviewed` sequence with
    `steps: [implement, verify, review]`, `on_step_fail: {step: verify, recovery:
    resume, max: 3}`, `checkpoint: true`; `write_checkpoint_json` in
    `crates/graft-engine/src/sequence.rs` reads `run_state_dir/review.json` when
    present; if `verdict` field exists, appends `" Review: <verdict>."` to the message;
    if `concerns` is a non-empty array, appends `" Concerns: <concerns joined by '; '>."`;
    unit tests assert checkpoint message includes review content when `review.json` is
    present and falls back to the default message when absent
  - **Files** — `.graft/software-factory/graft.yaml`,
    `crates/graft-engine/src/sequence.rs`
