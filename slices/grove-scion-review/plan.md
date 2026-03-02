---
status: pending
created: 2026-03-02
depends_on:
  - grove-scion-commands
---

# Grove scion review

## Story

After a worker finishes (or at any point during its work), the human needs to review
what changed — diffs against main, commit history, verification status. This is the
core human-in-the-loop moment: assess the scion's work and decide whether to fuse,
request changes, or continue.

Review is purely artifact-based. No runtime session is needed — the human can review a
scion whose agent has already exited. All state comes from git (diff, log, ahead/behind,
dirty).

## Approach

Four areas of work:

1. **Export `resolve_base_branch`** — this private function in
   `graft-engine/src/scion.rs` determines the base branch from the main worktree's
   branch ref. `scion_fuse` already uses it. Make it `pub` and add it to
   `graft-engine/src/lib.rs` exports so grove (and future callers) can resolve the
   base branch consistently. This is one line of visibility change + one export line.

2. **Git output helpers** — the existing git helpers in `graft-common/src/git.rs`
   return structured data (`git_ahead_behind` → `(u32, u32)`, `git_is_dirty` → `bool`).
   Review needs raw text output: diff content and commit log. Add two helpers:
   - `git_diff_stat(repo, base, head)` → `String` (runs `git diff --stat <base>...<head>`)
   - `git_diff_output(repo, base, head)` → `String` (runs `git diff <base>...<head>`)
   - `git_log_output(repo, base, head)` → `String` (runs `git log <base>..<head> --oneline`)

   These use three-dot (`...`) and two-dot (`..`) syntax respectively. The base branch
   is resolved by the caller via `resolve_base_branch()`, not hardcoded.

3. **Review command** — `:review <name>` gathers scion artifacts and renders them in
   the scroll buffer. Default view shows a **summary**: header (name, branch,
   ahead/behind, dirty, session status), commit log, and `--stat` diff summary.
   `:review <name> full` shows the complete diff instead of the stat summary.

   This two-tier approach handles large diffs gracefully — a scion with 50 changed files
   could produce thousands of lines of diff. The stat summary gives a quick overview;
   the full diff is available on demand.

   The output is a multi-section display:
   - **Header**: scion name, branch (`feature/<name>`), ahead/behind, dirty status,
     session indicator (if runtime available)
   - **Commit log**: commits on `feature/<name>` not on base branch
   - **Changes**: `--stat` summary (default) or full diff (`:review <name> full`)
   - **Verify**: deferred to a follow-up slice. The state cache is input-keyed (not
     scion-keyed), making cached result lookup non-trivial.

   Base branch resolution uses `resolve_base_branch()` from `graft-engine`, the same
   function `scion_fuse` uses. This ensures review and fuse always agree on the base.

4. **Follow-up actions** — after review, the output suggests next steps:
   `:scion fuse <name>` to merge, `:attach <name>` to interact with the agent,
   `:scion stop <name>` to halt the worker. These are informational suggestions
   rendered as text, not interactive buttons.

## Acceptance Criteria

- `resolve_base_branch` is exported from `graft-engine`
- `:review <name>` shows stat summary + commit log against the resolved base branch
- `:review <name> full` shows full diff instead of stat summary
- Diff uses three-dot syntax (`<base>...feature/<name>`) for merge-base comparison
- Commit log shows commits on the scion's branch not on the base branch
- Header shows scion name, branch, ahead/behind count, dirty status
- If the scion doesn't exist, shows a clear error
- If the scion has no commits ahead of the base branch, shows "no changes to review"
- Follow-up action suggestions shown after the review content
- `git_diff_stat`, `git_diff_output`, and `git_log_output` helpers exist in `graft-common`
- `cargo test && cargo clippy -- -D warnings && cargo fmt --check` passes

## Steps

- [ ] **Export `resolve_base_branch` from `graft-engine`**
  - **Delivers** — shared base branch resolution for review and future callers
  - **Done when** — `resolve_base_branch` in `graft-engine/src/scion.rs` changed from
    `fn` to `pub fn`; added to `pub use scion::{ ... }` in
    `graft-engine/src/lib.rs`; all existing tests pass unchanged
  - **Files** — `crates/graft-engine/src/scion.rs`, `crates/graft-engine/src/lib.rs`

- [ ] **Add `git_diff_stat`, `git_diff_output`, and `git_log_output` to `graft-common`**
  - **Delivers** — raw git text output for review display
  - **Done when** — `git_diff_stat(repo, base, head)` runs
    `git diff --stat <base>...<head>` (three-dot, merge-base) and returns the output
    as a `String`; `git_diff_output(repo, base, head)` runs
    `git diff <base>...<head>` (three-dot, merge-base, full diff) and returns the
    output as a `String`; `git_log_output(repo, base, head)` runs
    `git log <base>..<head> --oneline` (two-dot, commits on head not on base) and
    returns the output as a `String`; all return `GitError` on failure; tests verify
    output with a simple repo (create branch, add commit, check that stat/diff/log
    output contain expected content)
  - **Files** — `crates/graft-common/src/git.rs`

- [ ] **Add `:review` command parsing with optional `full` modifier**
  - **Delivers** — command routing for review with two display modes
  - **Done when** — `PALETTE_COMMANDS` includes `:review`; `CliCommand` enum has
    `Review(String, bool)` variant (name, full_diff flag); `parse_command()` handles
    `:review <name>` (full=false) and `:review <name> full` (full=true); tab
    completion offers scion names after `:review `; dispatches to review handler
  - **Files** — `crates/grove-cli/src/tui/command_line.rs`

- [ ] **Implement review data gathering and rendering**
  - **Delivers** — formatted review display in grove TUI
  - **Done when** — handler checks `self.context.selected_repo_path` (early return if
    none); calls `graft_common::git_worktree_list(repo_path)` once; uses result for
    both `graft_engine::resolve_base_branch(&worktrees)` (base branch) and scion
    existence validation (check if any worktree path ends in `.worktrees/<name>`);
    if scion not found, renders error and returns; calls `scion_list(repo_path, runtime)`
    to get `ScionInfo` for the named scion (ahead/behind, dirty, session status);
    if ahead count is 0, renders "no changes to review" and returns; otherwise gathers
    `git_log_output(repo, &base, &scion_branch)` and either `git_diff_stat` (default)
    or `git_diff_output` (full mode); renders as text blocks in scroll buffer with
    section headers ("Commits", "Changes"); header line shows scion name, branch,
    ahead/behind, dirty indicator, session status (if runtime available); follow-up
    suggestions rendered at the end (e.g., "Next: :scion fuse retry-logic |
    :attach retry-logic | :review retry-logic full")
  - **Files** — `crates/grove-cli/src/tui/transcript.rs`,
    `crates/grove-cli/src/tui/formatting.rs`
