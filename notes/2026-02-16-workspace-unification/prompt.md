You are an autonomous coding agent unifying the graft Rust workspace. The Rust rewrite is complete (14 tasks, 91 tests, full parity with Python). Your job is to extract shared infrastructure into a `graft-common` crate, clean up repo organization, and bring documentation to meta-KB compliance.

You are running inside a loop that will invoke you repeatedly until all tasks are complete. Each invocation starts with fresh context — you rely on files for memory.

## Step 1: Read state

Read these files to understand where things stand:
- `AGENTS.md` — project conventions
- `Cargo.toml` — workspace dependencies
- `notes/2026-02-16-workspace-unification/plan.md` — task list with checkboxes
- `notes/2026-02-16-workspace-unification/progress.md` — learnings from prior iterations

Pay attention to the "Consolidated Patterns" section in progress.md — it contains hard-won knowledge from previous iterations.

## Step 2: Pick the next task

Find the first task in plan.md marked `- [ ]`. If a task is marked `- [~]` (in progress), resume it. If all tasks are `- [x]`, you're done.

A task may be an implementation task OR a documentation/cleanup task. Follow the same workflow for both.

## Step 3: Read the relevant code

Read ALL source files and specs listed in the task. For extraction tasks, read BOTH the source code being extracted AND the target location. Understand the existing patterns before making changes.

Key code locations:
- `crates/grove-engine/src/` — grove engine (git ops, config parsing)
- `crates/grove-cli/src/state/` — grove state queries
- `crates/graft-engine/src/` — graft engine (resolution, commands, state)
- `crates/graft-cli/src/` — graft CLI
- `docs/` — documentation tree
- `grove/docs/` — grove docs (to be merged)

## Step 4: Implement

Implement the task, satisfying all acceptance criteria. For extraction tasks:
- Create shared code in `graft-common` first
- Update consumers to use the shared code
- Remove the old duplicated code
- Ensure all tests still pass

Use workspace dependencies from `Cargo.toml`. If you need a new dependency, add it to the workspace first.

If the task is too large for one iteration, mark it `- [~]` (in progress) and record what you completed in progress.md. The next iteration will pick it up.

If you discover a needed sub-task, insert it as `- [ ]` after the current task in plan.md.

## Step 5: Verify

Run: `cargo fmt --check && cargo clippy -- -D warnings && cargo test`

For documentation tasks, also verify:
- No broken links (check that referenced files exist)
- Frontmatter is valid YAML

If verification fails, fix the issues before proceeding. Do not mark a task complete if verification fails.

## Step 6: Commit implementation

Commit your implementation changes: `git add <specific files> && git commit -m "feat(graft): <description>"`

For documentation tasks use: `git commit -m "docs: <description>"`

Do NOT include plan.md or progress.md yet — those will be updated and committed in Step 9.

## Step 7: Self-critique

After committing, critically review what you just built. Read back through ALL the code you wrote or modified in this iteration. Evaluate against:

1. **Acceptance criteria** — Re-read the task's acceptance criteria. Are they all genuinely met, or did you interpret them loosely?
2. **Code quality** — Is the code idiomatic Rust? Does it follow existing patterns? Are error messages helpful? Is the public API clean?
3. **Test coverage** — Are the tests thorough? Do they cover error paths and edge cases?
4. **Integration** — Does this work well with what was built in prior iterations? Any inconsistencies?
5. **No regressions** — Did you accidentally break anything that was working before?

Write your critique findings directly into the progress log entry for this iteration (see Step 9).

## Step 8: Implement improvements

If the critique identified concrete issues (not just style nitpicks), fix them now:

- Fix acceptance criteria gaps, missing edge cases, inadequate tests, unclear error messages, or unidiomatic code.
- Do NOT add speculative features.
- Run verification again after fixes.
- Commit fixes separately: `git commit -m "fix(graft): address critique — <what was fixed>"`

If the critique found no actionable issues, skip this step.

## Step 9: Update plan and log progress

Mark the task `- [x]` in plan.md. Record any issues or patterns discovered in the appropriate plan.md sections.

Append an entry to `notes/2026-02-16-workspace-unification/progress.md` with this format:

```
### Iteration — <task title>
**Status**: completed | in-progress
**Files changed**: list of files
**What was done**: brief summary
**Critique findings**: what the self-review identified
**Improvements made**: what was fixed (or "none needed")
**Learnings for future iterations**: anything the next iteration should know
```

If you discovered a pattern that applies across tasks (not just this one), also add it to the "Consolidated Patterns" section at the top of progress.md.

**Commit the updated plan.md and progress.md**: `git add notes/2026-02-16-workspace-unification/plan.md notes/2026-02-16-workspace-unification/progress.md && git commit -m "docs: Update plan and progress for <task title> completion"`

This ensures each task ends with a commit marking its completion, making it clear where each task boundary is in the git history.
