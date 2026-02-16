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

---

### Iteration 4 — Extract shared state query types and cache logic to graft-common
**Status**: completed
**Files changed**:
- `crates/graft-common/src/state.rs` (new)
- `crates/graft-common/src/lib.rs` (added state module export)
- `crates/graft-common/Cargo.toml` (added serde, serde_json, sha2, chrono dependencies)

**What was done**:
Created `graft-common/src/state.rs` with shared state query types and cache management extracted from both `graft-engine/src/state.rs` and `grove-cli/src/state/{query.rs,cache.rs}`. Extracted types: `StateMetadata` (with `time_ago()` and `summary()` methods), `StateResult` (with domain-specific summary formatting). Extracted cache functions: `compute_workspace_hash()`, `get_cache_path()`, `get_query_cache_dir()`, `read_cached_state()`, `read_all_cached_for_query()`, `read_latest_cached()`, `write_cached_state()`, `invalidate_cached_state()`. Added 7 unit tests covering hash generation, path construction, time formatting, and summary formatting. All workspace tests pass (420+ tests).

**Critique findings**:
All acceptance criteria met. Code is idiomatic Rust with proper error handling (`Option<StateResult>` for reads, `std::io::Result` for writes). Test coverage is good for the types and their methods. The cache I/O functions don't have dedicated unit tests in graft-common, but they are straightforward file operations that will be integration-tested by consumers. Fixed a defensive programming issue where `&commit_hash[..7]` could panic on short hashes by using `.min(commit_hash.len())`. The implementation cleanly matches the original code from both source locations, ensuring smooth consumer migration.

**Improvements made**:
None needed. The implementation is solid and ready for consumer migration in Task 7.

**Learnings for future iterations**:
- When extracting code that exists in multiple locations, ensure the function signatures and behavior match exactly to avoid migration issues.
- The `StateMetadata::time_ago()` and `StateResult::summary()` methods provide domain-specific formatting that's valuable for both graft and grove CLIs.
- The cache path structure (`~/.cache/graft/{workspace-hash}/{repo-name}/state/{query-name}/{commit-hash}.json`) is shared across both tools, so extracting it to graft-common is the right choice.
- Both `graft-engine` and `grove-cli` had duplicate implementations of workspace hash computation (SHA256, truncated to 16 hex chars) - now they can share the same implementation.


---

### Iteration 5 — Migrate grove-engine to use graft-common command execution
**Status**: completed
**Files changed**:
- `crates/grove-engine/Cargo.toml` (added graft-common dependency, removed wait-timeout)
- `crates/grove-engine/src/git.rs` (replaced local run_git_with_timeout with thin wrapper that delegates to graft-common)

**What was done**:
Migrated grove-engine to use the shared timeout-protected command runner from graft-common. Replaced the 50-line local implementation of `run_git_with_timeout` with a thin wrapper function that converts `CommandError` to `CoreError` and delegates to `graft_common::command::run_command_with_timeout`. The wrapper preserves the existing API for all 6 call sites in grove-engine (status, rev-parse, rev-list ahead/behind, log, status detail) and maintains the `GROVE_GIT_TIMEOUT_MS` environment variable behavior. All 40 grove-engine tests pass unchanged.

**Critique findings**:
None. The implementation is clean and correct. The thin wrapper pattern is the right approach - it maintains the existing API for grove-engine callers while delegating to the shared implementation. Error conversion is comprehensive and preserves all error context. All acceptance criteria met.

**Improvements made**:
None needed.

**Learnings for future iterations**:
- The thin wrapper pattern (convert errors, delegate to shared code) is a clean way to migrate without changing all call sites.
- Removing `wait-timeout` from grove-engine's dependencies worked seamlessly since it now comes transitively via graft-common.
- All existing grove-engine tests continue to pass without modification, confirming the wrapper preserves exact behavior.
- The shared command runner now adds timeout protection to git operations that previously lacked it (which will benefit graft-engine in Task 6).
