---
status: accepted
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

Two small changes: update `verify.sh` to write its result to
`$GRAFT_STATE_DIR/verify.json` before printing to stdout (the existing output is
unchanged — we just also persist it), and add `writes: [verify]` to the `verify`
command in software-factory's `graft.yaml`. Grove's existing run-state loader
picks up `verify.json` automatically — no Rust changes required. The producer
annotation `(← software-factory:verify)` will appear automatically once `writes:`
is declared.

This is a foundational slice: it makes verify output visible in grove and
referenceable in subsequent workflow slices (specifically the loop condition in
`sequence-retry` and template access for failure context).

## Acceptance Criteria

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

- [ ] **Write verify result to run-state before printing**
  - **Delivers** — verify output is persisted and observable in grove
  - **Done when** — `verify.sh` writes its JSON result to
    `$GRAFT_STATE_DIR/verify.json` (using `mkdir -p` to ensure the directory
    exists) and then prints the same JSON to stdout; running
    `graft run software-factory:verify` and opening grove shows a `verify` entry
    in the Run State section; the entry expands to show `format`, `lint`, `tests`,
    and `smoke` fields; a verify that fails still writes the file
  - **Files** — `.graft/software-factory/scripts/verify.sh`

- [ ] **Declare writes: [verify] on the verify command**
  - **Delivers** — grove shows the producer annotation and the reads/writes
    relationship is machine-readable for future enforcement
  - **Done when** — `software-factory/graft.yaml` has `writes: [verify]` on the
    `verify` command; grove's Run State section shows `(← software-factory:verify)`
    next to the `verify` entry; the `verify` state query (under `state:`) is
    unchanged — only the `commands:verify` entry gets the declaration
  - **Files** — `.graft/software-factory/graft.yaml`
