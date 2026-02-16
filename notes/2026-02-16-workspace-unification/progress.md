---
status: working
purpose: "Append-only progress log for workspace unification Ralph loop"
---

# Progress Log

## Consolidated Patterns

(Patterns discovered across iterations that future iterations should know about)

---

### Iteration 1 — Create graft-common crate with timeout-protected command execution
**Status**: completed
**Files changed**:
- `crates/graft-common/Cargo.toml` (new)
- `crates/graft-common/src/lib.rs` (new)
- `crates/graft-common/src/command.rs` (new)
- `Cargo.toml` (added graft-common to workspace)
- `Cargo.lock` (updated)

**What was done**:
Created the new `graft-common` crate as a workspace member with timeout-protected command execution functionality. Extracted and generalized the `run_git_with_timeout` pattern from `grove-engine/src/git.rs` into `run_command_with_timeout`. The new function accepts an optional environment variable name for timeout configuration (supporting both `GROVE_GIT_TIMEOUT_MS` for backwards compatibility and future `GRAFT_COMMAND_TIMEOUT_MS`). Added 6 unit tests covering success, timeout, spawn failure, nonzero exit, and environment variable handling.

**Critique findings**:
Doc comment on `run_command_with_timeout` was misleading - it said the command "must have stdout/stderr configured" but the function itself configures them. This could confuse users about whether they need to pre-configure these.

**Improvements made**:
Updated documentation to clarify that the function configures stdout/stderr piping, so callers don't need to.

**Learnings for future iterations**:
- The workspace already has `wait-timeout`, `thiserror`, and `log` as workspace dependencies, so adding them to new crates is straightforward.
- The crate passes all checks in isolation even though the workspace has pre-existing clippy issues in `grove-cli/src/tui.rs`.
- When testing new crates in isolation, use `cd crates/<crate-name> && cargo test` to avoid workspace-wide clippy errors.

---

### Iteration 2 — Standardize on serde_yaml (remove serde_yml)
**Status**: completed
**Files changed**:
- `crates/grove-engine/src/config.rs` (replaced `serde_yml::from_str` calls)
- `crates/graft-engine/src/lock.rs` (replaced `serde_yml::from_str` and `serde_yml::to_string` calls)
- `crates/grove-engine/Cargo.toml` (changed `serde_yml` to `serde_yaml`)
- `crates/graft-engine/Cargo.toml` (removed `serde_yml`)
- `Cargo.toml` (removed `serde_yml` from workspace dependencies)
- `Cargo.lock` (removed `serde_yml` and transitive dependency `libyml`)

**What was done**:
Migrated all YAML parsing from `serde_yml` to `serde_yaml` across both `grove-engine` and `graft-engine`. Replaced 2 call sites in `grove-engine/src/config.rs` and 3 call sites in `graft-engine/src/lock.rs`. Removed `serde_yml` from all `Cargo.toml` files (workspace and individual crates). The migration is API-compatible - no behavioral changes, just swapping the underlying parser. Cargo.lock was automatically updated to remove the `serde_yml` crate and its transitive dependency `libyml`.

**Critique findings**:
None. The implementation is straightforward and correct. The changes are minimal and surgical - exactly what was needed. The `serde_yaml` and `serde_yml` APIs are compatible for basic deserialization/serialization, so this is a drop-in replacement. All 402+ tests still pass.

**Improvements made**:
None needed.

**Learnings for future iterations**:
- `serde_yaml` is API-compatible with `serde_yml` for basic `from_str` and `to_string` operations - simple find-and-replace works.
- The workspace had both `serde_yml` and `serde_yaml` dependencies before this task, so `graft-engine` was already using both.
- Removing unused dependencies from workspace `Cargo.toml` automatically cleans up `Cargo.lock` - no manual intervention needed.

---

### Iteration 3 — Add shared git primitives to graft-common
**Status**: completed
**Files changed**:
- `crates/graft-common/src/git.rs` (new)
- `crates/graft-common/src/lib.rs` (added git module export)
- `crates/graft-common/Cargo.toml` (added tempfile dev dependency)
- `Cargo.lock` (updated)

**What was done**:
Created `graft-common/src/git.rs` with shared git operations extracted from grove-engine and graft-engine. Implemented 5 functions: `is_git_repo()` (checks for .git directory), `get_current_commit()` (runs git rev-parse HEAD), `git_rev_parse()` (resolves refs to commit hashes, tries origin/ref first), `git_fetch()` (fetches from remote), and `git_checkout()` (checks out a commit). All functions use the timeout-protected command runner from Task 1. Added `GitError` enum for proper error handling. Added 9 unit tests covering success cases, failure cases, and edge cases. Test count increased from 402 to 420 (added 9 git tests, 6 command tests were from Task 1, total 15 in graft-common).

**Critique findings**:
All acceptance criteria met. Functions have proper unit tests (9 tests), existing tests pass (420 total), and verification passes (clippy clean for graft-common, pre-existing clippy issues in grove-cli TUI are expected per MEMORY.md). Code follows Rust idioms: uses `impl AsRef<Path>` for flexibility, proper error types with thiserror, comprehensive doc comments. Test coverage is good - covers both success and failure paths. The git_rev_parse function correctly tries `origin/ref` first for branches, then falls back to `ref` for tags/commits.

**Improvements made**:
None needed. The implementation is clean, well-tested, and ready for consumer migration.

**Learnings for future iterations**:
- `tempfile` crate is already available in the ecosystem and works well for git testing (creates temporary directories that auto-cleanup).
- Git test helpers (init_test_repo) can be shared across test modules - consider extracting if more git tests are added.
- The shared git primitives return `Result<String, GitError>` which consumers will need to convert to their own error types (e.g., `GraftError` or `CoreError`). This is intentional - each crate maintains its own error domain.
- Running `cargo clippy -p <crate>` checks a single crate in isolation, avoiding workspace-wide clippy issues.

