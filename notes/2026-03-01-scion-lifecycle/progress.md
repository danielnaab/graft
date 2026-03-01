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

---

### Iteration — Slice 2 (Tasks 2.1–2.C): Scion Config, Create, Prune
**Status**: completed
**Files changed**: `docs/specifications/graft/graft-yaml-format.md`, `crates/graft-engine/src/domain.rs`,
`crates/graft-engine/src/config.rs`, `crates/graft-engine/src/error.rs`,
`crates/graft-engine/src/scion.rs`, `crates/graft-engine/src/lib.rs`,
`crates/graft-cli/src/main.rs`, `crates/graft-engine/src/query.rs`,
`crates/graft-engine/src/validation.rs`
**What was done**: Added `scions:` spec section with all hook points, environment vars, composition,
and failure semantics. Added `ScionHooks` struct to domain.rs. Wired `scion_hooks: Option<ScionHooks>`
into `GraftConfig`. Added scions YAML parsing with string→vec normalization and unknown-key rejection.
Added hook command validation in `GraftConfig::validate()`. Added `From<GitError> for GraftError`.
Created `graft-engine/src/scion.rs` with `scion_create`/`scion_prune` applying `.worktrees/<name>`
+ `feature/<name>` convention. Added `graft scion create/prune` CLI subcommands.
**Critique findings**: No actionable issues. All acceptance criteria met. `map_err(GraftError::from)?`
pattern is functionally equivalent to `?` and clippy-clean. Empty scions mapping correctly produces
`scion_hooks: None` (no hooks defined).
**Improvements made**: None needed.
**Learnings for future iterations**: When adding a new field to `GraftConfig`, search for struct
literal initializers across the crate (`grep "GraftConfig {"`) — they need `field: None` added.
In this iteration, `query.rs` and `validation.rs` both had literals that needed updating.

