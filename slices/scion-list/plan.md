---
status: done
created: 2026-03-01
depends_on:
  - scion-config-create-prune
---

# Scion list with artifact-derived state

## Story

After creating scions, users need visibility into workstream state — which scions
exist, how far ahead or behind they are, whether there's active work, and when last
activity occurred. `graft scion list` derives all state from git artifacts (no worker
registry, no heartbeat), following the "artifacts over actors" principle from the
scion lifecycle design.

## Approach

Three layers:

1. **Git helpers** — add `git_last_commit_time` and `git_is_dirty` to
   `graft-common/src/git.rs`. These complement the existing `git_ahead_behind` from
   the worktree-primitives slice.

2. **Engine** — add `ScionInfo` struct and `scion_list` function to
   `graft-engine/src/scion.rs`. For each worktree returned by `git_worktree_list`,
   gather ahead/behind, last commit time, and dirty status. Filter to only
   `.worktrees/` entries (skip the main worktree).

3. **CLI** — add `graft scion list` subcommand with a text table (default) and
   `--format json` for machine consumption.

No hooks are involved — list is a pure query.

This slice depends on `scion-config-create-prune` (not just
`scion-worktree-primitives`) because that slice creates
`graft-engine/src/scion.rs` and declares the module in `lib.rs`. Building
on the existing file avoids merge conflicts.

## Acceptance Criteria

- `git_last_commit_time(repo, branch)` returns the timestamp of the branch's HEAD commit
- `git_is_dirty(worktree_path)` returns `bool` indicating uncommitted changes
- `ScionInfo` contains: name, branch, worktree path, ahead count, behind count,
  last commit time, dirty flag
- `scion_list(repo)` returns `Vec<ScionInfo>` for all `.worktrees/` entries
- `graft scion list` prints a human-readable table:
  ```
  retry-logic       3 ahead, 0 behind   last: 2m ago    dirty
  input-validation  1 ahead, 3 behind   last: 31m ago
  ```
- `graft scion list --format json` prints JSON array of scion info objects
- Empty list prints a "no scions" message (text) or empty array (JSON)
- `cargo test && cargo clippy -- -D warnings && cargo fmt --check` passes

## Steps

- [ ] **Add `git_last_commit_time` and `git_is_dirty` to `graft-common/src/git.rs`**
  - **Delivers** — git helpers for activity and dirty state
  - **Done when** — `git_last_commit_time(repo, branch)` runs
    `git log -1 --format=%ct <branch>` and returns a Unix timestamp (or `GitError`
    if branch has no commits); `git_is_dirty(worktree_path)` runs
    `git -C <path> status --porcelain` and returns `true` if output is non-empty;
    unit tests verify both
  - **Files** — `crates/graft-common/src/git.rs`

- [ ] **Add `ScionInfo` and `scion_list` to `graft-engine/src/scion.rs`**
  - **Delivers** — structured scion enumeration with artifact-derived state
  - **Done when** — `ScionInfo { name, branch, worktree_path, ahead, behind,
    last_commit_time, dirty }` struct exists with `Serialize` derive;
    `scion_list(repo_path)` calls `git_worktree_list`, filters to `.worktrees/`
    entries, extracts scion name from path, gathers per-scion metrics via
    `git_ahead_behind`, `git_last_commit_time`, `git_is_dirty`; returns
    `Vec<ScionInfo>`; integration test creates two scions, commits in one,
    verifies list output
  - **Files** — `crates/graft-engine/src/scion.rs`

- [ ] **Add `graft scion list` CLI subcommand**
  - **Delivers** — user-facing scion enumeration
  - **Done when** — `graft scion list` calls `scion_list` and prints a formatted
    table with columns: name, ahead/behind, relative time, dirty indicator;
    `--format json` flag outputs `serde_json::to_string_pretty` of the
    `Vec<ScionInfo>`; empty list prints "No scions" (text) or `[]` (JSON);
    time formatting uses relative durations ("2m ago", "1h ago", "3d ago")
  - **Files** — `crates/graft-cli/src/main.rs`
