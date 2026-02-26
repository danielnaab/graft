---
status: draft
created: 2026-02-26
depends_on: [verify-captures-state, context-snapshot]
---

# Automated acceptance criteria verification against implementation diff

## Story

`verify` confirms that tests pass and code is clean, but says nothing about whether
the implementation actually satisfies the slice's acceptance criteria semantically. A
slice might pass all tests while missing a criterion that wasn't covered by tests.
`spec-check` runs Claude against the acceptance criteria from the slice plan and the
implementation diff, producing a structured per-criterion verdict. This surfaces
unmet criteria before the human checkpoint, closing the gap between "tests pass" and
"spec is met."

## Approach

New `scripts/spec-check.sh`:
1. Reads `baseline_sha` from `context-snapshot.json` (fallback: `HEAD~1`)
2. Reads slice path from `session.json`
3. Reads the "Acceptance Criteria" section from `slices/<slug>/plan.md`
4. Gets `git diff <baseline_sha>` to see the full implementation diff
5. Pipes criteria + diff to `claude -p --dangerously-skip-permissions` with a
   verification prompt
6. Writes `spec-check.json`:
   ```json
   {
     "overall": "pass | partial | fail",
     "unmet_count": 0,
     "criteria": [
       {
         "text": "...",
         "status": "met | unmet | partial | not_verifiable",
         "evidence": "...",
         "concern": "..."
       }
     ]
   }
   ```

`not_verifiable` is used when a criterion cannot be evaluated from a static diff (e.g.
runtime behavior, performance characteristics). `overall: "pass"` when all criteria
are `met` or `not_verifiable`. `overall: "partial"` when any are `partial`.
`overall: "fail"` when any are `unmet`.

Add `spec-check` command to `graft.yaml` with `reads: [session, context-snapshot]`,
`writes: [spec-check]`.

`spec-check` is used standalone (run after `implement-verified` before `approve`) or
as a composable step in a future `implement-spec-verified` sequence.

## Acceptance Criteria

- `graft run software-factory:spec-check` produces `spec-check.json` with per-criterion
  results and an `overall` field
- `overall: "pass"` when all criteria are `met` or `not_verifiable`
- `overall: "partial"` when any criterion is `partial`
- `overall: "fail"` when any criterion is `unmet`
- Running without `session.json` exits 1 with a clear error
- Running when the slice plan cannot be found exits 1 with a clear error
- Running when the slice plan has no Acceptance Criteria section exits 1 with a
  clear error
- `cargo test` passes with no regressions

## Steps

- [ ] **Write `scripts/spec-check.sh` and add `spec-check` command to `graft.yaml`**
  - **Delivers** — standalone spec-check command that verifies acceptance criteria
    against the implementation diff
  - **Done when** — `spec-check.sh` reads `baseline_sha` from `context-snapshot.json`
    or falls back to `HEAD~1`; reads slice path from `session.json`; reads the
    Acceptance Criteria section from the slice plan (lines between `## Acceptance
    Criteria` and the next `##` heading); constructs a verification prompt with the
    diff and criteria list, requesting JSON output with `overall`, `unmet_count`,
    `criteria` fields; pipes to `claude -p --dangerously-skip-permissions`; writes
    `$GRAFT_STATE_DIR/spec-check.json`; exits 1 with clear messages on missing inputs;
    `graft.yaml` adds `spec-check` command with `reads: [session, context-snapshot]`,
    `writes: [spec-check]`; manual test: run on a recently-implemented slice, inspect
    `spec-check.json` to confirm criterion coverage is correct
  - **Files** — `.graft/software-factory/scripts/spec-check.sh`,
    `.graft/software-factory/graft.yaml`
