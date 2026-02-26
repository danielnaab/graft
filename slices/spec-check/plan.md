---
status: draft
created: 2026-02-26
depends_on: [verify-captures-state, context-snapshot]
---

# Automated acceptance criteria verification against implementation diff

## Story

`verify` confirms that tests pass and code is clean, but says nothing about whether
the implementation addresses the slice's acceptance criteria. A criterion that has no
corresponding code change in the diff may have been forgotten. `spec-check` runs
Claude against the acceptance criteria and the implementation diff to produce a
per-criterion **evidence report**: for each criterion, what code changes implement it?
This is a coverage check — flagging criteria with no implementation evidence — not a
correctness verification. Claude cannot reliably verify that its own implementation is
correct; it can reliably identify which criteria have no diff evidence at all.

## Approach

New `scripts/spec-check.sh`:
1. Reads `baseline_sha` from `context-snapshot.json` (fallback: `HEAD~1`)
2. Reads slice path from `session.json`
3. Reads the "Acceptance Criteria" section from `slices/<slug>/plan.md`
4. Gets `git diff <baseline_sha>` to see the full implementation diff
5. Pipes criteria + diff to `claude -p --dangerously-skip-permissions` with an
   evidence-mapping prompt: "For each criterion, identify what code changes in this
   diff implement it. If no diff changes address a criterion, flag it as uncovered."
6. Writes `spec-check.json`:
   ```json
   {
     "overall": "covered | partial | uncovered",
     "uncovered_count": 0,
     "criteria": [
       {
         "text": "...",
         "coverage": "covered | uncovered | not_diffable",
         "evidence": "...",
         "note": "..."
       }
     ]
   }
   ```

`not_diffable` is used when a criterion describes runtime behavior or performance that
cannot be inferred from a static diff. `overall: "covered"` when all criteria are
`covered` or `not_diffable`. `overall: "partial"` when any are `uncovered` but others
are `covered`. `overall: "uncovered"` when the majority are uncovered.

Add `spec-check` command to `graft.yaml` with `reads: [session, context-snapshot]`,
`writes: [spec-check]`.

`spec-check` is used standalone (run after `implement-verified` before `approve`) or
as a composable step in a future `implement-spec-verified` sequence.

## Acceptance Criteria

- `graft run software-factory:spec-check` produces `spec-check.json` with per-criterion
  evidence mapping and an `overall` field
- `overall: "covered"` when all criteria are `covered` or `not_diffable`
- `overall: "partial"` when some criteria are `covered` and some are `uncovered`
- `overall: "uncovered"` when the majority of criteria have no diff evidence
- Running without `session.json` exits 1 with a clear error
- Running when the slice plan cannot be found exits 1 with a clear error
- Running when the slice plan has no Acceptance Criteria section exits 1 with a
  clear error
- `cargo test` passes with no regressions

## Steps

- [ ] **Write `scripts/spec-check.sh` and add `spec-check` command to `graft.yaml`**
  - **Delivers** — standalone spec-check command that maps acceptance criteria to
    implementation evidence in the diff, flagging uncovered criteria
  - **Done when** — `spec-check.sh` reads `baseline_sha` from `session.json` (written
    by `implement.sh` before launching Claude) or falls back to `HEAD~1` if absent;
    reads slice path from `session.json`; reads the Acceptance Criteria section from
    the slice plan (lines between `## Acceptance Criteria` and the next `##` heading);
    constructs an evidence-mapping prompt: "For each criterion, identify what code
    changes in this diff implement it. Flag criteria with no diff evidence as
    uncovered."; requests JSON output with `overall`, `uncovered_count`, `criteria`
    fields (each: `text`, `coverage`, `evidence`, `note`); pipes to
    `claude -p --dangerously-skip-permissions`; writes `$GRAFT_STATE_DIR/spec-check.json`;
    exits 1 with clear messages on missing inputs; `graft.yaml` adds `spec-check`
    command with `reads: [session]`, `writes: [spec-check]` (no `reads: [context-snapshot]`
    — baseline_sha comes from session.json); manual test: run on a recently-implemented
    slice, inspect `spec-check.json` to confirm criterion evidence mapping is reasonable
  - **Files** — `.graft/software-factory/scripts/spec-check.sh`,
    `.graft/software-factory/graft.yaml`
