---
status: draft
created: 2026-02-26
depends_on: [sequence-declarations]
---

# Persist per-session context snapshot for warm Claude session resumption

## Story

When Claude resumes an implementation session — whether via `resume.sh` after verify
failure or after a crash resumption — it starts with minimal situational awareness.
It knows the verify failures (if any) and can scroll back through the session
transcript, but it must reconstruct from scratch what approach it was taking, what it
discovered, and what still needs doing. This reconstruction burns context window and
leads to inconsistent approaches across retries. A context snapshot lets Claude write
a brief structured handoff note at the end of each session, which the next session
reads to jump in with full situational awareness.

The `baseline_sha` field in the snapshot also enables other commands (`review`,
`spec-check`, `diagnose`) to compute accurate diffs of exactly what was implemented
in this session.

## Approach

Inject a prompt suffix into the piped prompts in `implement.sh` and `resume.sh`
instructing Claude to write a context snapshot as its final action. The snapshot path
is `$GRAFT_STATE_DIR/context-snapshot.json` with fields:

```json
{
  "baseline_sha": "<git sha recorded at session start>",
  "completed_work": "...",
  "current_state": "...",
  "next_steps": "...",
  "known_issues": "..."
}
```

`baseline_sha` is written by `implement.sh` **before** launching Claude (via
`git rev-parse HEAD`), not by Claude. Claude is instructed to preserve the existing
`baseline_sha` when writing its snapshot fields.

Modify `resume.sh` to read `context-snapshot.json` when present and inject a
formatted summary before any verify/feedback sections. If the file is absent or
fields are empty, resume proceeds unchanged.

Add `context-snapshot` as a state entry in `graft.yaml`: `writes: [session,
context-snapshot]` on both `implement` and `resume`; `reads: [session,
context-snapshot]` on `resume` (optional — absence must not cause a hard failure;
verify whether graft's `reads:` enforcement allows missing optional state files and
document accordingly).

## Acceptance Criteria

- After `implement` completes, `context-snapshot.json` exists with `baseline_sha`,
  `completed_work`, `current_state`, `next_steps`, and `known_issues` fields
- `baseline_sha` is the git SHA at session start, written before Claude launches
- After `resume` completes, `context-snapshot.json` is updated (overwritten)
  with new session fields; `baseline_sha` is preserved from the prior snapshot
- `resume.sh` injects a formatted context summary when `context-snapshot.json`
  exists, before verify failure or rejection feedback sections
- If `context-snapshot.json` is absent (first run, or Claude failed to write it),
  `resume.sh` proceeds unchanged without error
- `cargo test` passes with no regressions

## Steps

- [ ] **Record `baseline_sha` at session start in `implement.sh` and inject snapshot
  prompt suffix in both `implement.sh` and `resume.sh`**
  - **Delivers** — `context-snapshot.json` is written at the end of each session with
    structured handoff fields and an accurate baseline SHA
  - **Done when** — `implement.sh` runs `git rev-parse HEAD` before launching Claude
    and writes `{"baseline_sha": "<sha>", "slice": "<slug>"}` to
    `$GRAFT_STATE_DIR/context-snapshot.json` as a seed file; `iterate.sh`'s output
    (piped to Claude) gains a suffix injected by `implement.sh`: "When you have
    completed your work for this session, write a JSON context snapshot to
    `$GRAFT_STATE_DIR/context-snapshot.json` with fields: `completed_work` (string
    summary of what you did), `current_state` (what state the codebase is in now),
    `next_steps` (what remains), `known_issues` (any problems you noticed but didn't
    fix). Preserve the existing `baseline_sha` field."; `resume.sh` injects the same
    suffix; `graft.yaml` updates `implement` to `writes: [session, context-snapshot]`
    and `resume` to `reads: [session, context-snapshot], writes: [session,
    context-snapshot]` — if graft's `reads:` enforcement hard-fails on absent files,
    change `resume` to only `writes: [context-snapshot]` and let the script handle
    absence gracefully
  - **Files** — `.graft/software-factory/scripts/implement.sh`,
    `.graft/software-factory/scripts/resume.sh`,
    `.graft/software-factory/graft.yaml`

- [ ] **Inject context snapshot summary into `resume.sh` prompt**
  - **Delivers** — resumed sessions start with full situational awareness from the
    previous session
  - **Done when** — `resume.sh` reads `$GRAFT_STATE_DIR/context-snapshot.json`; if
    any of `completed_work`, `current_state`, or `next_steps` are non-empty, prepends
    a formatted section to the resume prompt: `"## Context from previous session\n\n
    Completed: <completed_work>\nCurrent state: <current_state>\nNext steps:
    <next_steps>\nKnown issues: <known_issues>\n\n"`; this section appears before any
    verify failure or rejection feedback sections; if the file is absent or fields
    are empty, the section is omitted; manual end-to-end test: run implement on a
    slice, inspect `context-snapshot.json`, run resume, confirm Claude's opening
    message acknowledges the prior session context
  - **Files** — `.graft/software-factory/scripts/resume.sh`
