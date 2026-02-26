---
status: done
created: 2026-02-24
---

# Persist verify output as named run-state

## Story

Every `graft run software-factory:verify` call produces a structured JSON result
but it evaporates — it doesn't appear in grove's Run State section, can't be
referenced in templates, and isn't usable as a loop condition. This slice makes
verify's output first-class run-state so it's observable in grove and available
for downstream commands.

## Approach

Three small changes: fix the consumer's `scripts/verify.sh` to exit non-zero
when any check fails (currently the final `jq` command always exits 0 regardless
of test failures, making it useless as a gate); update the software-factory's
`verify.sh` wrapper to capture the consumer's output and write it to
`$GRAFT_STATE_DIR/verify.json`; and add `writes: [verify]` to the `verify`
command in software-factory's `graft.yaml`.

Grove's existing run-state loader picks up `verify.json` automatically — no Rust
changes required. The producer annotation `(← software-factory:verify)` appears
automatically once `writes:` is declared.

This is a foundational slice: it makes verify output visible in grove and
referenceable in subsequent workflow slices (specifically the exit-code gate in
`sequence-retry` and template access for failure context in `resume.sh`).

## Acceptance Criteria

- `verify` exits non-zero when any check fails (format, lint, tests, or smoke)
- After `graft run software-factory:verify`, a `verify` entry appears in grove's
  Run State section showing `{format, lint, tests, smoke}` with producer annotation
- The existing stdout output of `verify.sh` is unchanged — nothing that currently
  consumes it breaks
- `graft state query verify` (from software-factory's context) returns the last
  verify result
- A verify run that finds failures still writes the result (so the loop can read why
  it failed) — the persistence is not gated on success
- `.graft/run-state/verify.json` is not committed to git (`.gitignore` already covers
  `.graft/run-state/`)
- `cargo test` passes with no regressions

## Steps

- [ ] **Fix consumer verify.sh to exit non-zero on check failures**
  - **Delivers** — verify is a proper gating mechanism; the sequence executor can
    detect failure via exit code and trigger retries
  - **Done when** — `scripts/verify.sh` computes an `overall_exit` from the
    individual check exit codes (`fmt_exit`, `lint_exit`, `test_exit`, `smoke_exit`)
    and calls `exit $overall_exit` after the `jq -n` output; the JSON result is
    always produced regardless of exit code (persistence is unconditional); a
    broken test or lint error causes the script to exit 1 while still writing valid
    JSON; `cargo test` continues to pass
  - **Files** — `scripts/verify.sh`

- [ ] **Write verify result to run-state before printing**
  - **Delivers** — verify output is persisted and observable in grove
  - **Done when** — `.graft/software-factory/scripts/verify.sh` is restructured
    so that ALL exit paths produce output and, when `$GRAFT_STATE_DIR` is set,
    write it: (1) the unconfigured-consumer path (`if [ ! -f "$VERIFY_SCRIPT" ]`)
    is changed to set `result=$(jq -n '{status:"unconfigured",...}')` and fall
    through to the write+print block rather than exiting early — this ensures
    `verify.json` is written even when no consumer verify script exists, so grove
    always shows an entry after `graft run software-factory:verify`; (2) the main
    path captures `result=$(bash "$VERIFY_SCRIPT"); rc=$?`; (3) both paths then
    write to `$GRAFT_STATE_DIR/verify.json` **only when `$GRAFT_STATE_DIR` is set
    and nonempty** (`if [ -n "${GRAFT_STATE_DIR:-}" ]; then mkdir -p
    "$GRAFT_STATE_DIR" && printf '%s\n' "$result" > "$GRAFT_STATE_DIR/verify.json";
    fi`) — this guard prevents failure when called via `state: verify` (state
    queries do not inject `$GRAFT_STATE_DIR` and must remain read-only); (4) the
    script prints to stdout and exits with `$rc`; running
    `graft run software-factory:verify` and opening grove shows a `verify` entry
    in the Run State section regardless of whether a consumer verify script exists;
    a verify that fails still writes the file; running `graft state query verify`
    does NOT write or update `verify.json`
  - **Files** — `.graft/software-factory/scripts/verify.sh`

- [ ] **Declare writes: [verify] on the verify command**
  - **Delivers** — grove shows the producer annotation and the reads/writes
    relationship is machine-readable for future enforcement
  - **Done when** — `software-factory/graft.yaml` has `writes: [verify]` on the
    `verify` command; grove's Run State section shows `(← software-factory:verify)`
    next to the `verify` entry; the `verify` state query (under `state:`) is
    unchanged — only the `commands:verify` entry gets the declaration
  - **Files** — `.graft/software-factory/graft.yaml`
