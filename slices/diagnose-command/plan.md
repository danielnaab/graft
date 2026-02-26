---
status: draft
created: 2026-02-26
depends_on: [verify-captures-state, context-snapshot]
---

# Targeted diagnosis step to produce structured root-cause analysis before resume

## Story

When verify fails with complex errors — type system breakdowns, missing imports,
architectural mismatches — `resume.sh` injects raw error text from `verify.json` into
the Claude prompt. Claude's first action is typically to re-read many files to
reconstruct context, burning most of the session on diagnosis rather than fixing.

A `diagnose` command runs a focused diagnostic session: it reads the verify failures
plus the recently changed files, runs Claude with a targeted diagnosis prompt, and
produces structured `diagnose.json` with root cause, affected files, and a concrete
remediation approach. `resume.sh` injects this brief instead of raw errors, letting
the implementation session start with targeted fixing rather than re-diagnosis.

This separates "understanding what's wrong" from "fixing it" — the Plan-and-Execute
pattern that the field has found more reliable than monolithic ReAct loops.

**Cost note**: `diagnose` launches a full Claude session (est. $0.10–0.50 per run).
It is an optional standalone command — not integrated into any automated sequence.
Users run it manually when verify failures are complex and resume is struggling.

## Approach

New `scripts/diagnose.sh`:
1. Reads `verify.json` — exits 1 with `"Nothing to diagnose: verify passed"` if all
   fields start with "OK"
2. Reads recently changed files from `git diff --name-only <baseline_sha>` (from
   `session.json`) or falls back to `git status --short` if baseline_sha is absent
3. Reads slice acceptance criteria from `session.json`'s `slice` field
4. Constructs a targeted diagnosis prompt: here are the failures, here are the changed
   files, here is what this step was supposed to achieve — diagnose root cause
5. Pipes to `claude -p --dangerously-skip-permissions`
6. Writes `diagnose.json`:
   ```json
   {
     "root_cause": "...",
     "affected_files": ["...", "..."],
     "suggested_approach": "...",
     "specific_fixes": [
       {"file": "...", "issue": "...", "fix": "..."}
     ]
   }
   ```

Modify `resume.sh` to check for `diagnose.json` when present: if it exists and verify
still shows failures (same fields non-OK), inject diagnosis content instead of the raw
verify failure dump. If verify now passes (diagnose is stale), ignore `diagnose.json`.

Format of injected diagnosis:
```
## Diagnosis from automated analysis

Root cause: <root_cause>

Affected files: <affected_files joined by ', '>

Approach: <suggested_approach>

Specific issues:
- <file>: <issue> → <fix>
```

Add `diagnose` command to `graft.yaml` with `reads: [verify, session]`,
`writes: [diagnose]` (no `reads: [context-snapshot]` — baseline_sha comes from
`session.json`; absence handled defensively in the script).

## Acceptance Criteria

- `graft run software-factory:diagnose` when `verify.json` has failures produces
  `diagnose.json` with `root_cause`, `affected_files`, `suggested_approach`,
  `specific_fixes`
- `graft run software-factory:diagnose` when verify shows all OK exits 1 with
  `"Nothing to diagnose: verify passed"`
- `resume.sh` injects `diagnose.json` content when present and verify still shows
  failures, replacing the raw verify failure dump
- A stale `diagnose.json` (from a prior session where verify now passes) is ignored
  by `resume.sh`
- Running `diagnose` without `verify.json` exits 1 with a clear error
- `cargo test` passes with no regressions

## Steps

- [ ] **Write `scripts/diagnose.sh` and add `diagnose` command to `graft.yaml`**
  - **Delivers** — standalone diagnosis command that produces `diagnose.json` from
    verify failures
  - **Done when** — `diagnose.sh` checks `verify.json` for failures; if all fields
    start with "OK", exits 1; reads changed files from git; reads slice path from
    `session.json`; reads acceptance criteria from the slice plan; constructs diagnosis
    prompt; pipes to `claude -p --dangerously-skip-permissions`; writes `diagnose.json`;
    `graft.yaml` adds `diagnose` command with `reads: [verify, session]`,
    `writes: [diagnose]` (baseline_sha read from `session.json`, not context-snapshot);
    manual test: let verify fail on a slice, run diagnose, inspect `diagnose.json`
  - **Files** — `.graft/software-factory/scripts/diagnose.sh`,
    `.graft/software-factory/graft.yaml`

- [ ] **Inject `diagnose.json` into `resume.sh` prompt**
  - **Delivers** — resumed sessions start with a targeted diagnosis brief rather than
    a raw error dump
  - **Done when** — `resume.sh` checks for `diagnose.json`; if present, re-checks
    `verify.json` for current failures; if failures still exist, injects the formatted
    diagnosis section above any remaining verify context; if verify now passes (diagnose
    is stale), skips the injection; manual end-to-end test: run implement, let verify
    fail, run diagnose, run resume — confirm Claude opens with the structured diagnosis
    brief rather than raw error text
  - **Files** — `.graft/software-factory/scripts/resume.sh`
