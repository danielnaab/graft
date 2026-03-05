---
status: done
created: 2026-02-26
depends_on: [workflow-checkpoints]
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
1. Reads `baseline_sha` from `session.json` (fallback: `HEAD~1` if absent)
2. Reads slice path from `session.json`
3. Reads the "Acceptance Criteria" section from the slice's `plan.md`
4. Gets `git diff <baseline_sha>` to see the full implementation diff
5. Pipes diff + acceptance criteria to `claude -p --dangerously-skip-permissions`
   with a structured **adversarial** review prompt: "You are a skeptical reviewer
   who did NOT write this code. Your goal is to find what's missing or wrong, not
   to confirm it looks correct. For each acceptance criterion, find evidence for it
   in the diff — or flag it as unaddressed. List any concerns you see."
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
7. **Always exits 0** — the review is advisory. A `verdict: "fail"` surfaces in
   grove's Run State for human review; it does not abort the sequence.
8. After writing `review.json`, reads the existing `checkpoint.json` (if present)
   and updates its `message` field to include the verdict:
   `jq --arg v "$verdict" '.message += " Review: \($v)."' checkpoint.json > tmp && mv tmp checkpoint.json`
   This runs only after `checkpoint.json` has been written by the sequence engine.
   If `checkpoint.json` is absent, skip silently.

Add `review` command to `graft.yaml` with `reads: [session]`, `writes: [review]`
(no `reads: [context-snapshot]` — baseline_sha comes from session.json).

Add `implement-reviewed` sequence: `steps: [implement, verify, review]` with
`on_step_fail: {step: verify, recovery: resume, max: 3}` and `checkpoint: true`.
Since `review` always exits 0, it cannot fail the sequence.

**Do not modify `write_checkpoint_json` in `sequence.rs`** — that would couple
graft-engine to software-factory's review.json state file name. The review script
updates checkpoint.json directly after the sequence engine writes it.

## Acceptance Criteria

- `graft run software-factory:review` produces `review.json` with `verdict`,
  `criteria`, `concerns`, and `summary` fields
- `verdict: "pass"` when all criteria appear met and no concerns
- `verdict: "concerns"` when criteria are met but minor concerns exist
- `verdict: "fail"` when one or more criteria are unmet
- `review` always exits 0 regardless of verdict — it is advisory only
- Running `review` without `session.json` exits 1 with a clear error
- Running `review` without a slice plan exits 1 with a clear error
- `implement-reviewed` sequence runs `implement → verify → review → checkpoint`
  with retry on verify failure (up to 3 times) before reaching review; `review`
  never causes the sequence to fail
- `checkpoint.json` message includes the review verdict when `review.json` is present
  (updated by `review.sh` directly, not by sequence.rs)
- Running `review` when `session.json` has no `baseline_sha` falls back to `HEAD~1`
- `cargo test && cargo clippy -- -D warnings && cargo fmt --check` passes

## Steps

- [x] **Write `scripts/review.sh` and add `review` command to `graft.yaml`**
  - **Delivers** — standalone `review` command produces a structured self-review against
    acceptance criteria
  - **Done when** — `review.sh` reads `baseline_sha` from `session.json` (fallback:
    `HEAD~1`); reads slice path from `session.json`; reads the "Acceptance Criteria"
    section from the slice's `plan.md`; constructs an adversarial prompt framing
    Claude as a skeptical reviewer who did NOT write the code, tasked with finding
    gaps and concerns rather than confirming correctness; requests JSON output with
    `verdict`, `criteria`, `concerns`, `summary` fields; pipes to
    `claude -p --dangerously-skip-permissions`; writes `$GRAFT_STATE_DIR/review.json`;
    always exits 0 (advisory); if `$GRAFT_STATE_DIR/checkpoint.json` exists, updates
    its `message` field via atomic `jq` + tmp+rename to append `" Review: <verdict>."`;
    `graft.yaml` adds `review` command with `reads: [session]`, `writes: [review]`
    (no `reads: [context-snapshot]`); manual test: run implement on a slice, run review,
    inspect `review.json` and `checkpoint.json`
  - **Files** — `.graft/software-factory/scripts/review.sh`,
    `.graft/software-factory/graft.yaml`

- [x] **Add `implement-reviewed` sequence to `graft.yaml`**
  - **Delivers** — the full implement → verify → review → human-checkpoint cycle as a
    single command; since review always exits 0, the sequence completes normally even
    when verdict is "fail"; the human sees the verdict in grove's Run State
  - **Done when** — `graft.yaml` declares `implement-reviewed` sequence with
    `steps: [implement, verify, review]`, `on_step_fail: {step: verify, recovery:
    resume, max: 3}`, `checkpoint: true`; no changes to `sequence.rs` — the checkpoint
    message enrichment is handled by `review.sh` writing directly to `checkpoint.json`
    after the sequence engine writes it; manual test: run `implement-reviewed` on a
    slice, confirm `review.json` appears in Run State, confirm `checkpoint.json`
    message includes verdict
  - **Files** — `.graft/software-factory/graft.yaml`
