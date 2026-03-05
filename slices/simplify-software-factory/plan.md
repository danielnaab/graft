---
status: done
created: 2026-03-03
depends_on: []
---

# Simplify software-factory by replacing bash prompt scripts with stdin declarations

## Story

Software-factory has 14 scripts totaling ~700 lines of bash. Six of them
(implement, iterate, read-slice, new-slice, review, diagnose, spec-check,
resume) exist primarily to construct prompts in bash and pipe them to
`claude -p`. This is fragile — `new-slice` already fails because Claude
ignores the "output slug first" instruction, and `read-slice.sh` is 140 lines
of bash regex to parse markdown that Claude can just read directly.

The insight: Claude can read `plan.md` itself. Instead of bash scripts parsing
markdown, extracting sections, and assembling prompts, graft commands should use
`stdin: literal` to give Claude a short instruction and let it read the plan
file. The scripts disappear; the prompt lives in `graft.yaml` where it's
visible, versionable, and composable.

## Approach

Replace prompt-construction scripts with `stdin: literal` declarations in
`graft.yaml`. Each command's stdin tells Claude what to do and where to find
context — Claude reads the files itself.

**Session management**: `implement.sh` currently generates a session ID and
writes `session.json`. This moves to a small wrapper script that handles only
the session bookkeeping (3 lines: generate ID, write JSON, exec claude). The
prompt construction is gone.

**Commands that change**:

| Command | Before | After |
|---------|--------|-------|
| `implement` | `implement.sh` → `iterate.sh` → `read-slice.sh` (3 scripts) | `stdin: literal` + thin session wrapper |
| `new-slice` | `new-slice.sh` (prompt + slug extraction) | `stdin: literal`, slug becomes a required arg |
| `review` | `review.sh` (prompt from diff + criteria) | `stdin: literal` referencing plan + baseline |
| `diagnose` | `diagnose.sh` (prompt from failures + diff) | `stdin: literal` referencing verify.json |
| `spec-check` | `spec-check.sh` (prompt from criteria + diff) | `stdin: literal` referencing plan + baseline |
| `resume` | `resume.sh` (context assembly + session resume) | `stdin: literal` referencing state files |

**Commands that stay as-is**: `verify` (delegation), `approve`/`reject` (state
ops), `list-slices` (directory enumeration), `plan` (already uses
`stdin: template`).

**Scripts removed**: `iterate.sh`, `read-slice.sh`, `lib.sh`, `new-slice.sh`,
`review.sh`, `diagnose.sh`, `spec-check.sh`. Possibly `implement.sh` and
`resume.sh` shrink to thin session wrappers.

**New arg for `new-slice`**: takes `slug` as a required arg (user-provided,
kebab-case) instead of asking Claude to generate it. Eliminates the fragile
slug extraction. Description becomes the second arg.

## Acceptance Criteria

- `implement` reads the slice plan directly via Claude instead of bash parsing
- `new-slice` takes a slug arg and a description arg; no slug extraction from
  Claude output
- `review`, `diagnose`, and `spec-check` use `stdin: literal` to instruct
  Claude to read the plan and diff, replacing bash prompt construction
- `resume` uses `stdin: literal` to instruct Claude to read state files for
  context
- `iterate.sh`, `read-slice.sh`, `lib.sh` are removed
- `list-slices.sh`, `verify.sh`, `approve.sh`, `reject.sh` remain unchanged
- All commands produce the same functional output as before (session.json,
  review.json, diagnose.json, spec-check.json, context-snapshot.json)
- `graft run software-factory:implement <slice>` works end-to-end
- `graft run software-factory:new-slice <slug> <description>` creates a valid
  plan.md

## Steps

- [x] **Replace `implement` with `stdin: literal` and thin session wrapper**
  - **Delivers** — implement command that lets Claude read the plan directly
  - **Done when** — `implement` command uses `stdin: literal` telling Claude to
    read `slices/{slice}/plan.md`, find the next `- [ ]` step, implement it,
    run verification, and mark it `[x]`; a thin wrapper script
    (`implement-session.sh`, ~10 lines) handles only: generate session ID,
    write `session.json`, exec `claude -p --session-id` with stdin from graft;
    `iterate.sh` and `read-slice.sh` are deleted; `lib.sh` is deleted if no
    other script uses it; end-to-end test: `graft run software-factory:implement
    <slice>` launches Claude and Claude reads the plan
  - **Files** — `.graft/software-factory/graft.yaml`,
    `.graft/software-factory/scripts/implement-session.sh` (new, thin),
    `.graft/software-factory/scripts/implement.sh` (deleted),
    `.graft/software-factory/scripts/iterate.sh` (deleted),
    `.graft/software-factory/scripts/read-slice.sh` (deleted),
    `.graft/software-factory/scripts/lib.sh` (deleted if unused)

- [x] **Replace `new-slice` with `stdin: literal` and slug as arg**
  - **Delivers** — reliable slice creation without fragile slug extraction
  - **Done when** — `new-slice` command takes `slug` (string, required) and
    `description` (string, required) as args; uses `stdin: literal` to instruct
    Claude to create `slices/{slug}/plan.md` with the standard format;
    `new-slice.sh` is deleted; slug validation (kebab-case, no existing
    directory) moves to a pre-check in the run command or is left to Claude
    with clear instructions
  - **Files** — `.graft/software-factory/graft.yaml`,
    `.graft/software-factory/scripts/new-slice.sh` (deleted)

- [x] **Replace `review`, `diagnose`, and `spec-check` with `stdin: literal`**
  - **Delivers** — three Claude-calling commands simplified to yaml declarations
  - **Done when** — each command uses `stdin: literal` telling Claude to: read
    the slice plan for acceptance criteria, read the git diff since baseline
    (from `$GRAFT_STATE_DIR/session.json`), and output structured JSON to the
    appropriate state file; `review.sh`, `diagnose.sh`, `spec-check.sh` are
    deleted; the JSON output schemas remain the same (review.json,
    diagnose.json, spec-check.json)
  - **Files** — `.graft/software-factory/graft.yaml`,
    `.graft/software-factory/scripts/review.sh` (deleted),
    `.graft/software-factory/scripts/diagnose.sh` (deleted),
    `.graft/software-factory/scripts/spec-check.sh` (deleted)

- [x] **Replace `resume` with `stdin: literal` and session wrapper**
  - **Delivers** — resume command that lets Claude read state files for context
  - **Done when** — `resume` command uses `stdin: literal` telling Claude to
    read `$GRAFT_STATE_DIR/session.json`, `context-snapshot.json`,
    `verify.json`, `checkpoint.json`, and `diagnose.json` (all optional) for
    context, then continue implementing the slice; the thin session wrapper
    extracts the session ID and passes `--resume` to Claude; `resume.sh` is
    deleted
  - **Files** — `.graft/software-factory/graft.yaml`,
    `.graft/software-factory/scripts/resume-session.sh` (new, thin),
    `.graft/software-factory/scripts/resume.sh` (deleted)
