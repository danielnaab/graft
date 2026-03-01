---
status: working
purpose: "Append-only progress log for scion lifecycle Ralph loop"
---

# Progress Log

## Consolidated Patterns

- **Always re-export from `lib.rs`**: New public types/functions added to `graft-common/src/git.rs`
  must also be added to the `pub use git::{ ... }` block in `lib.rs`. Downstream crates import
  from `graft_common::` not `graft_common::git::`.
- **`ProcessConfig` pattern**: Every git helper uses `ProcessConfig { command, working_dir, env: None,
  env_remove: vec![], log_path: None, timeout: Some(Duration::from_secs(30)), stdin: None }` then
  calls `run_to_completion_with_timeout`. Check `output.success`, return `GitError::CommandFailed`
  with `output.stderr` on failure.
- **Porcelain parsing**: `git worktree list --porcelain` stanzas are separated by `\n\n`. Each line
  uses `strip_prefix`. Unknown lines (locked, prunable, detached) are silently skipped.
- **Test helper**: `init_test_repo(path)` sets git config + initial commit; `make_commit(path, file, msg)`
  for additional commits. Use `TempDir::new()` from tempfile crate.

---

### Iteration — Slice 1 (Tasks 1.1–1.C): Git Worktree Primitives
**Status**: completed
**Files changed**: `crates/graft-common/src/git.rs`, `crates/graft-common/src/lib.rs`
**What was done**: Added `WorktreeInfo` struct, `parse_worktree_list` (private), `git_worktree_list`,
`git_worktree_add`, `git_worktree_remove`, `git_branch_delete`, `git_ahead_behind`. 12 new tests.
Re-exported all new symbols from `lib.rs`.
**Critique findings**: New functions were not re-exported from the crate root — downstream crates
would have needed to use `graft_common::git::*` instead of `graft_common::*`. Also `std::path::PathBuf`
was being used as a qualified path in the struct definition instead of importing it.
**Improvements made**: Added all new types/functions to `pub use git::{ ... }` in `lib.rs`. Added
`PathBuf` to the `use std::path::...` import and removed qualified references.
**Learnings for future iterations**: Always check `lib.rs` re-exports when adding to `graft-common`.
The `graft-engine` crate imports via `use graft_common::{WorktreeInfo, git_worktree_list, ...}`.

