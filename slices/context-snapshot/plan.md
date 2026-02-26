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

The `baseline_sha` is recorded in `session.json` (not the snapshot) so it is
available to other commands (`review`, `spec-check`, `diagnose`) even when the
snapshot is absent.

## Approach

`implement.sh` records `baseline_sha` in `session.json` alongside `slice`, using
`git rev-parse HEAD` before launching Claude. This gives `review`, `spec-check`, and
`diagnose` a reliable baseline SHA that does not depend on Claude writing anything.

The context snapshot (`context-snapshot.json`) is produced by injecting a prompt
suffix into the piped prompts in `implement.sh` and `resume.sh`, instructing Claude
to write the snapshot as a final action. The snapshot fields are:

```json
{
  "completed_work": "...",
  "current_state": "...",
  "next_steps": "...",
  "known_issues": "..."
}
```

This approach is **best-effort**: Claude may not write the snapshot if it runs out
of context or time. The prompt suffix must be short and placed prominently. If
`context-snapshot.json` is absent or incomplete after the session, commands that
read it proceed without it — no hard failure.

Modify `resume.sh` to read `context-snapshot.json` when present and inject a
formatted summary before any verify/feedback sections. The check is defensive:
`[ -f "$snapshot" ] || skip`.

**Do NOT declare `reads: [context-snapshot]` in `graft.yaml`** — graft's reads
enforcement hard-fails when the file is absent, which breaks first runs. Instead,
use defensive shell checks. `implement` and `resume` declare `writes: [context-snapshot]`
only.

## Acceptance Criteria

- `session.json` contains `baseline_sha` (written by `implement.sh` before Claude
  launches) alongside the `slice` field — no dependency on Claude for baseline SHA
- After `implement` or `resume` completes, `context-snapshot.json` exists (best-effort)
  with `completed_work`, `current_state`, `next_steps`, and `known_issues` fields
  (no `baseline_sha` — that lives in `session.json`)
- After `resume` completes, `context-snapshot.json` is updated (overwritten) with
  the new session's snapshot fields
- `resume.sh` injects a formatted context summary when `context-snapshot.json`
  exists, before verify failure or rejection feedback sections
- If `context-snapshot.json` is absent (first run, or Claude failed to write it),
  `resume.sh` proceeds unchanged without error
- `graft.yaml` does NOT declare `reads: [context-snapshot]` on any command
- `cargo test` passes with no regressions

## Steps

- [ ] **Record `baseline_sha` in `session.json` and inject snapshot prompt suffix in
  `implement.sh` and `resume.sh`**
  - **Delivers** — `session.json` reliably contains `baseline_sha`; `context-snapshot.json`
    is written at the end of each session with structured handoff fields (best-effort)
  - **Done when** — `implement.sh` runs `git rev-parse HEAD` before launching Claude
    and adds `"baseline_sha": "<sha>"` to the `session.json` write (alongside `"slice"`);
    `iterate.sh`'s output (piped to Claude) gains a short suffix: "When you have
    completed your work for this session, write a JSON file to
    `$GRAFT_STATE_DIR/context-snapshot.json` with these fields: `completed_work`
    (what you did), `current_state` (state of the codebase now), `next_steps`
    (what remains), `known_issues` (problems noticed but not fixed). Do not include
    `baseline_sha`."; `resume.sh` injects the same suffix; `graft.yaml` updates
    `implement` to `writes: [session, context-snapshot]` and `resume` to
    `writes: [context-snapshot]` (NO `reads: [context-snapshot]` on any command —
    absence is handled defensively in the scripts)
  - **Files** — `.graft/software-factory/scripts/implement.sh`,
    `.graft/software-factory/scripts/resume.sh`,
    `.graft/software-factory/graft.yaml`

- [ ] **Inject context snapshot summary into `resume.sh` prompt**
  - **Delivers** — resumed sessions start with full situational awareness from the
    previous session
  - **Done when** — `resume.sh` checks `[ -f "$GRAFT_STATE_DIR/context-snapshot.json" ]`
    before attempting to read it; if present and any of `completed_work`,
    `current_state`, or `next_steps` are non-empty, prepends a formatted section to
    the resume prompt: `"## Context from previous session\n\nCompleted:
    <completed_work>\nCurrent state: <current_state>\nNext steps: <next_steps>\nKnown
    issues: <known_issues>\n\n"`; this section appears before any verify failure or
    rejection feedback sections; if the file is absent or fields are empty, the section
    is silently omitted (no error); manual end-to-end test: run implement on a slice,
    inspect `context-snapshot.json`, run resume, confirm Claude's opening message
    acknowledges the prior session context
  - **Files** — `.graft/software-factory/scripts/resume.sh`
