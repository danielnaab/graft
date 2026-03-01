---
status: done
created: 2026-03-01
depends_on: []
---

# Git worktree primitives for scion lifecycle

## Story

Scion lifecycle commands (`create`, `list`, `fuse`, `prune`) all need git worktree
and branch operations — add, list, remove, delete branch, ahead/behind counts. These
don't exist in `graft-common` yet. Adding them as a standalone library slice keeps
git plumbing out of engine logic and lets multiple downstream slices consume the same
tested primitives.

## Approach

Add worktree and branch helpers to `crates/graft-common/src/git.rs`, following the
existing pattern: each function takes a repo path and explicit arguments, runs `git`
via `ProcessConfig` with the 30-second default timeout, returns a typed result or
`GitError`. No config parsing, no CLI commands, no engine changes — pure library
functions.

Functions take explicit paths and branch names as arguments — no hardcoded naming
conventions. The `.worktrees/<name>` path and `feature/<name>` branch convention is
policy that belongs in the engine layer (`scion_create`, `scion_prune`), not in
git primitives. This matches the existing pattern: `git_checkout(path, commit)` and
`git_rev_parse(path, git_ref)` are generic, not graft-convention-aware.

## Acceptance Criteria

- `git_worktree_add(repo, path, branch)` creates a worktree at `path` on new branch `branch`
- `git_worktree_list(repo)` returns `Vec<WorktreeInfo>` parsed from `git worktree list --porcelain`
- `git_worktree_remove(repo, path)` removes the worktree at `path`
- `git_branch_delete(repo, branch)` deletes `branch` (force-delete to handle unmerged)
- `git_ahead_behind(repo, branch, base)` returns `(usize, usize)` commit counts
- All functions take explicit paths/branch names — no hardcoded naming conventions
- All functions return `Result<_, GitError>` using the existing error type
- Unit tests cover success paths and common failure modes (already exists, not found)
- `cargo test -p graft-common && cargo clippy -p graft-common -- -D warnings` passes

## Steps

- [ ] **Add `WorktreeInfo` type and `git_worktree_list` to `graft-common/src/git.rs`**
  - **Delivers** — ability to enumerate worktrees with path, branch, and HEAD info
  - **Done when** — `WorktreeInfo { path, branch, head }` struct exists (fields
    match `git worktree list --porcelain` output: `worktree <path>`,
    `HEAD <sha>`, `branch <ref>`); `git_worktree_list` parses porcelain output
    into `Vec<WorktreeInfo>`; unit test with `git init` + `git worktree add`
    verifies round-trip
  - **Files** — `crates/graft-common/src/git.rs`

- [ ] **Add `git_worktree_add` to `graft-common/src/git.rs`**
  - **Delivers** — worktree creation primitive with explicit arguments
  - **Done when** — `git_worktree_add(repo, path, branch)` runs
    `git worktree add <path> -b <branch>`; returns the absolute worktree path
    on success; returns `GitError` if worktree or branch already exists;
    test creates a worktree and confirms it appears in `git_worktree_list`
  - **Files** — `crates/graft-common/src/git.rs`

- [ ] **Add `git_worktree_remove` and `git_branch_delete` to `graft-common/src/git.rs`**
  - **Delivers** — worktree and branch cleanup primitives
  - **Done when** — `git_worktree_remove(repo, path)` runs
    `git worktree remove <path> --force`; `git_branch_delete(repo, branch)`
    runs `git branch -D <branch>`; tests confirm worktree and branch are gone
    after removal; removing a non-existent worktree returns `GitError`
  - **Files** — `crates/graft-common/src/git.rs`

- [ ] **Add `git_ahead_behind` to `graft-common/src/git.rs`**
  - **Delivers** — commit count comparison between any two branches
  - **Done when** — `git_ahead_behind(repo, branch, base)` runs
    `git rev-list --left-right --count <branch>...<base>`, parses output into
    `(usize, usize)`; test creates diverging branches and verifies counts
  - **Files** — `crates/graft-common/src/git.rs`
