---
status: done
created: 2026-03-08
depends_on: [simplify-software-factory]
---

# Add single-agent command to software-factory with retry loop

## Story

As a developer using software-factory to implement slices, I want a single
agent command that reads repo state, does one unit of work, verifies, and is
automatically re-invoked until done — so that I can start a scion and walk
away instead of orchestrating implement → verify → resume → verify chains
manually.

## Approach

Add an `agent` command to software-factory that collapses the
implement/resume/verify/review/diagnose cycle into a single autonomous loop.

The command runs `agent-loop.sh`, a wrapper script (~30 lines) that:

1. Generates a fresh session ID per invocation
2. Launches `claude -p --dangerously-skip-permissions` with a generic prompt
3. After Claude exits, reads an `agent-status.json` file that Claude wrote
   to determine the outcome (done / continue / blocked)
4. Loops on "continue" (up to a max), exits on "done" or "blocked"

Each loop iteration is a **fresh Claude session**. No `--session-id` reuse —
a stuck agent shouldn't carry forward a flawed reasoning chain. The files
are the memory: plan.md checkmarks, verify output, and context-snapshot.json
tell the next invocation exactly where things stand.

The prompt is generic and stable:

> Read CLAUDE.md for project conventions. Read slices/$GRAFT_SLICE/plan.md
> for your task. Find the next unchecked step. Implement it. Run
> scripts/verify.sh. Fix failures. Mark the step done when verification
> passes. When all steps are done and verification passes, write
> `{"status":"done"}` to $GRAFT_STATE_DIR/agent-status.json. If more work
> remains, write `{"status":"continue"}`. If you are blocked and need human
> help, write `{"status":"blocked","reason":"..."}`.

The exit code protocol lives entirely in the wrapper script — graft
doesn't need to change. The retry loop, max limit, and status file
interpretation are user-managed concerns in bash.

Existing commands (`implement`, `resume`, `verify`, `review`, `diagnose`)
and sequences (`implement-verified`, `implement-reviewed`) remain for
ad-hoc human-driven use. The `agent` command is the new primary path for
autonomous scion execution.

## Acceptance Criteria

- Software-factory has an `agent` command that drives the full
  implement→verify→fix cycle autonomously
- `agent-loop.sh` launches fresh Claude sessions in a loop, reading
  `agent-status.json` after each invocation to decide whether to continue
- The loop exits on: "done" (exit 0), "blocked" (exit 2), max iterations
  reached (exit 1), or Claude failure (exit code preserved)
- Each iteration writes `agent-status.json` before Claude runs with
  `{"status":"running","iteration":N,"max":M}` for observability
- `graft scion create <name>` + `graft scion start <name>` with
  `scions.start: software-factory:agent` launches the autonomous agent
  loop in a tmux session
- The generic prompt instructs Claude to self-review before marking a
  step done
- Existing commands and sequences are unchanged (no regressions)
- `cargo test` passes with no regressions

## Steps

- [x] **Create agent-loop.sh wrapper script**
  - **Delivers** — the retry loop and exit code protocol
  - **Done when** — `scripts/agent-loop.sh` takes a slice arg; validates
    the slice plan exists; runs a loop up to `MAX_ITERATIONS` (default 30,
    overridable via `$GRAFT_AGENT_MAX_ITERATIONS`); each iteration:
    writes `agent-status.json` with `{"status":"running","iteration":N,
    "max":M}` to `$GRAFT_STATE_DIR`, generates a fresh session ID,
    launches `claude -p --dangerously-skip-permissions` with stdin piped
    from the parent process (graft's stdin: literal), writes
    `session.json` with the session ID and baseline SHA; after Claude
    exits: if Claude exited non-zero, the loop exits with that code; reads
    `$GRAFT_STATE_DIR/agent-status.json` — if `status` is `"done"`,
    writes final status and exits 0; if `"blocked"`, writes final status
    and exits 2; if `"continue"`, continues the loop; if the file is
    missing or unparseable, treats it as `"continue"` (agent forgot to
    write it — retry and hope it does next time); after max iterations
    without "done", writes `{"status":"exhausted",...}` and exits 1;
    the script reads stdin into a variable on first entry so it can
    re-pipe the same prompt to each fresh Claude session
  - **Files** — `.graft/software-factory/scripts/agent-loop.sh`

- [x] **Add agent command to software-factory graft.yaml**
  - **Delivers** — one command to run the autonomous agent loop
  - **Done when** — software-factory's `graft.yaml` has an `agent` command
    with: `run: "bash scripts/agent-loop.sh {slice}"`,
    `writes: [session, context-snapshot]`, a `stdin:` literal prompt that
    instructs Claude to: read CLAUDE.md and AGENTS.md for project
    conventions, read `slices/$GRAFT_SLICE/plan.md` for the task, find
    the next unchecked step (`- [ ]`), implement it following the step's
    "Done when" criteria, run `scripts/verify.sh` and fix any failures,
    self-review the changes against acceptance criteria before marking
    done, mark the step `[x]` when verification passes, write
    `context-snapshot.json` to `$GRAFT_STATE_DIR`, write
    `agent-status.json` to `$GRAFT_STATE_DIR` with `{"status":"done"}`
    when all steps are complete and verification passes,
    `{"status":"continue"}` when the current step is done but more remain,
    or `{"status":"blocked","reason":"..."}` when stuck; the prompt must
    emphasize: work on ONE step at a time, do not attempt multiple steps
    in a single invocation; `args:` takes a `slice` choice arg with
    `options_from: slices` (same as `implement`); the command has
    `category: core`
  - **Files** — `.graft/software-factory/graft.yaml`

- [x] **Wire agent as scion start hook**
  - **Delivers** — `graft scion start` runs the autonomous agent loop
  - **Done when** — software-factory's `graft.yaml` `scions.start` is set
    to `software-factory:agent`; existing commands and sequences remain
    unchanged; end-to-end validation: `graft run software-factory:agent
    slices/<test-slice>` launches the agent loop, Claude reads the plan,
    does work, writes agent-status.json, and the wrapper script responds
    correctly to each status value
  - **Files** — `.graft/software-factory/graft.yaml`
