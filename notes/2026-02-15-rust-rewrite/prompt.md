You are an autonomous coding agent implementing graft (a semantic dependency manager) in Rust. You are running inside a loop that will invoke you repeatedly until all tasks are complete. Each invocation starts with fresh context — you rely on files for memory.

## Step 1: Read state

Read these files to understand where things stand:
- `AGENTS.md` — project conventions
- `Cargo.toml` — workspace dependencies
- `notes/2026-02-15-rust-rewrite/plan.md` — task list with checkboxes
- `notes/2026-02-15-rust-rewrite/progress.md` — learnings from prior iterations

Pay attention to the "Consolidated Patterns" section in progress.md — it contains hard-won knowledge from previous iterations.

## Step 2: Pick the next task

Find the first task in plan.md marked `- [ ]`. If a task is marked `- [~]` (in progress), resume it. If all tasks are `- [x]`, you're done.

A task may be an implementation task OR a critique-improvement task (marked with `[CRITIQUE]`). Follow the same workflow for both.

## Step 3: Read the specs

Read ALL spec files and Python reference files listed in the task. The specifications in `docs/specifications/graft/` are the **primary authority** for what to build. The Python code in `src/graft/` is a behavioral reference for when specs are silent.

If specs and Python disagree, **follow the spec** (the Python code has the bug). Record the conflict in plan.md under "Resolved spec/implementation conflicts".

Read existing Rust code in `crates/graft-*/src/` and `crates/grove-*/src/` to understand established patterns.

## Step 4: Implement

Implement the task, satisfying all acceptance criteria. You choose the internal structure — file names, type names, module layout. Follow the patterns established in the grove crates.

Use workspace dependencies from `Cargo.toml`. If you need a new dependency, add it to the workspace first.

If the task is too large for one iteration, mark it `- [~]` (in progress) and record what you completed in progress.md. The next iteration will pick it up.

If you discover a needed sub-task, insert it as `- [ ]` after the current task in plan.md.

## Step 5: Verify

Run: `cargo fmt --check && cargo clippy -- -D warnings && cargo test`

If verification fails, fix the issues before proceeding. Do not mark a task complete if verification fails.

## Step 6: Commit implementation

Commit your implementation changes: `git add <specific files> && git commit -m "feat(graft): <description>"`

Do NOT include plan.md or progress.md yet — those will be updated and committed in Step 9.

## Step 7: Self-critique

After committing, critically review what you just built. Read back through ALL the code you wrote or modified in this iteration. Evaluate against:

1. **Spec compliance** — Does the implementation fully satisfy the spec, or did you cut corners or miss edge cases?
2. **Acceptance criteria** — Re-read the task's acceptance criteria. Are they all genuinely met, or did you interpret them loosely?
3. **Code quality** — Is the code idiomatic Rust? Does it follow the patterns in the grove crates? Are error messages helpful? Is the public API clean?
4. **Test coverage** — Are the tests thorough? Do they cover error paths and edge cases, or just the happy path?
5. **Integration** — Does this work well with what was built in prior iterations? Any inconsistencies?

Write your critique findings directly into the progress log entry for this iteration (see Step 9).

## Step 8: Implement improvements

If the critique identified concrete issues (not just style nitpicks), fix them now:

- Fix spec compliance gaps, missing edge cases, inadequate tests, unclear error messages, or unidiomatic code.
- Do NOT add speculative features.
- Run verification again after fixes.
- Commit fixes separately: `git commit -m "fix(graft): address critique — <what was fixed>"`

If the critique found no actionable issues, skip this step.

## Step 9: Update plan and log progress

Mark the task `- [x]` in plan.md. Record any spec gaps or conflicts in the appropriate plan.md sections.

Append an entry to `notes/2026-02-15-rust-rewrite/progress.md` with this format:

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

**Commit the updated plan.md and progress.md**: `git add notes/2026-02-15-rust-rewrite/plan.md notes/2026-02-15-rust-rewrite/progress.md && git commit -m "docs: Update plan.md and progress.md for <task title> completion"`

This ensures each task ends with a commit marking its completion, making it clear where each task boundary is in the git history.
