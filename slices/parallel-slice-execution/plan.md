---
status: draft
created: 2026-02-26
depends_on: [sequence-resumability]
---

# Run multiple independent slices in parallel using git worktrees

## Story

Implementing slices sequentially means one Claude session runs at a time, even when
slices are completely independent — different subsystems, no shared files. Production
multi-agent systems (Cursor 2.0) demonstrated 36% latency reduction by running 4–6
agents concurrently in isolated git worktrees. This slice enables running multiple
`implement-verified` instances simultaneously: each slice gets its own worktree with
its own run-state, and results are reported when all complete.

## Approach

New `scripts/implement-parallel.sh <slice1> <slice2> [<slice3> [<slice4>]]`:

1. Validate all slice paths exist; exit 1 if any are missing
2. Derive a slug for each (strip `slices/` prefix and trailing `/`)
3. Check that all worktree branch names (`implement/<slug>`) are available; exit 1
   if any exist (all-or-nothing start — no partial state)
4. For each slice:
   - `git worktree add .worktrees/<slug> -b implement/<slug>`
   - Copy `.graft/` into the worktree: `.graft/` is part of the git tree so the
     worktree has it; verify this is correct before shipping
   - Run `graft run software-factory:implement-verified <slice>` in the worktree as
     a background process (`&`), capturing the PID
5. Wait for all PIDs to complete
6. Collect exit codes; print a summary table:
   ```
   implement-parallel results:
     slices/a  ✓ passed  (branch: implement/a)
     slices/b  ✗ failed  (branch: implement/b)
   ```
7. Print merge instructions for passing slices:
   ```
   To merge passing slices:
     git merge implement/a
   To clean up all worktrees:
     git worktree remove .worktrees/a
   ```
8. Exit 0 if all passed, exit 1 if any failed

**Key constraint**: Each worktree gets its own `.graft/run-state/` (relative to the
worktree root), so `sequence-state.json` and `verify.json` are per-worktree with no
state collision.

**Key constraint**: `implement-parallel.sh` is not invoked from within Claude Code
(it runs in grove or CLI), so `CLAUDECODE` env var is unset and Claude subprocesses
in the worktrees are not blocked.

Add `implement-parallel` command to `graft.yaml`. Since graft's `args` doesn't
support variadic positional args, use four optional args: `slice1` (required),
`slice2` (required), `slice3` (optional), `slice4` (optional). The script skips
empty args.

## Acceptance Criteria

- `graft run software-factory:implement-parallel slices/a slices/b` creates two
  worktrees and runs `implement-verified` in each in parallel
- Each worktree has its own `sequence-state.json` and `verify.json`
- If one slice fails, the other continues independently
- If any worktree branch name already exists, command exits 1 before creating any
  worktrees
- If any slice path does not exist, command exits 1 before creating any worktrees
- On completion, a summary shows per-slice pass/fail with worktree branch names
- Worktree branches are left open for human review and merge; merge instructions
  are printed
- `cargo test` passes with no regressions (this is a script-only change)

## Steps

- [ ] **Write `scripts/implement-parallel.sh` and add `implement-parallel` command
  to `graft.yaml`**
  - **Delivers** — parallel implementation of multiple slices in isolated git worktrees
  - **Done when** — `implement-parallel.sh` accepts 2–4 slice args; validates all paths
    exist; validates all worktree branches are available; creates worktrees via
    `git worktree add`; launches `graft run software-factory:implement-verified <slice>`
    in each worktree as a background process using `(cd .worktrees/<slug> && graft run
    software-factory:implement-verified <slice>) &`; collects PIDs; waits for all
    (`wait $pid1; wait $pid2; ...`); reads exit codes; prints summary table and merge
    instructions; `graft.yaml` adds `implement-parallel` command with `slice1`
    (required positional), `slice2` (required positional), `slice3` (optional),
    `slice4` (optional) string args; manual test: run on two known-independent slices,
    confirm both run simultaneously via `ps aux | grep claude`, confirm each worktree
    has its own run-state, confirm summary is printed on completion
  - **Files** — `.graft/software-factory/scripts/implement-parallel.sh`,
    `.graft/software-factory/graft.yaml`
