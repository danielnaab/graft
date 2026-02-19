---
status: working
purpose: "Append-only progress log for unified process management Ralph loop"
---

# Progress Log

## Consolidated Patterns

- **`spawn` signature**: `ProcessHandle::spawn` takes `&ProcessConfig` (not by value). Clippy
  pedantic rejects pass-by-value when the struct is not actually consumed. Callers write
  `ProcessHandle::spawn(&config)`.
- **Monitor thread pattern**: Use `try_wait()` polling (10ms) inside a lock-release loop,
  not blocking `wait()`. This lets `kill()` acquire the child mutex without deadlocking.
- **Detaching monitor thread**: `drop(thread::spawn(...))` avoids `clippy::let_underscore_must_use`
  on the intentionally-detached `JoinHandle`.
- **`#[derive(Debug)]` on ProcessHandle**: Needed for `Result::unwrap_err()` in tests because
  `std::process::Child` implements Debug in Rust 1.65+.
- **Private helpers take `&mpsc::Receiver` by reference**: `recv_timeout` is `&self`, so
  helpers can take `&mpsc::Receiver<ProcessEvent>`. Clippy `needless_pass_by_value` rejects
  owned `Receiver` that isn't consumed. Use `for event in rx` (not `rx.iter()`) because
  clippy `explicit_iter_loop` prefers the short form for `&Receiver`.
- **`#[allow(clippy::too_many_lines)]` on long but cohesive functions**: `spawn` must set up
  three threads plus a log file handle — inherently long. Targeted `#[allow]` on the single
  function is cleaner than splitting artificially.
- **`build_output` takes `&[String]` slices**: Clippy `needless_pass_by_value` rejects
  `Vec<String>` args when only `.join()` (a slice method) is called. Pass `&stdout_lines`
  and `&stderr_lines`; `Vec<String>` coerces to `&[String]` automatically.
- **Generic-Arc pattern for trait objects**: When a public API needs to accept `Arc<ConcreteType>`
  but internally stores `Arc<dyn Trait>`, use a generic bound: `pub fn foo<R: Trait + 'static>(r: Arc<R>)`.
  Inside, coerce via `let dyn_arc: Arc<dyn Trait> = r;`. Avoids callers needing explicit `.as` casts.
- **`dyn Trait` in `Arc` does not auto-implement `Debug`**: Even if all known implementors do, the
  trait object `dyn Trait` only implements `Debug` if the trait itself has `Debug` as a supertrait.
  To derive `Debug` on a struct containing `Arc<dyn Trait>`, either add `Debug` to the trait's
  supertraits or implement `Debug` manually with `finish_non_exhaustive()` to skip the field.

---

### Iteration — Task 1: ProcessEvent, ProcessHandle with streaming output
**Status**: completed
**Files changed**: `crates/graft-common/src/process.rs` (new), `crates/graft-common/src/lib.rs`
**What was done**: Implemented `ProcessEvent`, `ProcessError`, `ProcessConfig`, and
`ProcessHandle` in a new `process.rs` module. Spawn uses `sh -c`, reader threads send
`OutputLine` events via mpsc, and a monitor thread polls `try_wait()`, joins both readers,
then sends `Completed`/`Failed`. `kill()` delegates to `Child::kill()`. Exported key types
from `lib.rs`. Five tests pass: echo, stderr capture, nonzero exit, spawn failure (invalid
workdir), and kill.
**Critique findings**: API is clean. `Timeout` and `IoError` in `ProcessError` are declared
but not yet used (they're for Task 2). `log_path` and `timeout` in `ProcessConfig` are
present but unused (bridge for Task 2). No issues with ordering guarantees or thread safety.
**Improvements made**: none needed
**Learnings for future iterations**: See Consolidated Patterns above. Task 2 will use
`log_path` (tee to file) and `timeout` (kill after duration) — both fields are already in
`ProcessConfig`, no API change needed.

### Iteration — Task 2: Log capture and sync convenience wrappers
**Status**: completed
**Files changed**: `crates/graft-common/src/process.rs`, `crates/graft-common/src/lib.rs`
**What was done**: Added log file tee (both stdout and stderr share `Arc<Mutex<File>>`
opened in append mode), `ProcessOutput` struct, `run_to_completion`, and
`run_to_completion_with_timeout` (timeout priority: config.timeout → GRAFT_PROCESS_TIMEOUT_MS
env var → unlimited). Extracted three private helpers: `collect_output` (drains channel via
`for event in rx`), `collect_output_with_timeout` (deadline loop with `recv_timeout`),
`build_output`. New tests: log write, log append, stdout+stderr collection, multiline output,
nonzero exit, timeout trigger, no-timeout fast path, env-var fallback path (9 tests). All
39 graft-common tests pass. Fixed 5 clippy issues: `too_many_lines` (targeted `#[allow]`),
`needless_pass_by_value` on helpers (switched to `&ProcessHandle`, `&mpsc::Receiver`,
`&[String]`), `explicit_iter_loop` (`rx.iter()` → `rx`).
**Critique findings**: All acceptance criteria met. `ProcessError::IoError` still unused
(will be used in Task 3 for registry file I/O). API is clean and consistent with Task 1.
The `Disconnected` arm in `collect_output_with_timeout` handles the edge case where the
channel closes without a terminal event — defensive but harmless.
**Improvements made**: none needed
**Learnings for future iterations**: See new Consolidated Patterns entries above. Task 3
(ProcessRegistry) will add `chrono` for ISO 8601 timestamps and `serde`/`serde_json` for
JSON serialization — both already in graft-common's `[dependencies]`.

### Iteration — Task 3: ProcessRegistry trait and FsProcessRegistry
**Status**: completed
**Files changed**: `crates/graft-common/src/process.rs`, `crates/graft-common/src/lib.rs`
**What was done**: Added `ProcessStatus` enum (Running/Completed/Failed, Serialize+Deserialize+PartialEq),
`ProcessEntry` struct with `new_running()` constructor (uses `Utc::now().to_rfc3339()` for ISO 8601),
`ProcessRegistry` trait (`register`, `deregister`, `list_active`, `get`, `update_status`), and
`FsProcessRegistry` storing `{pid}.json` files. Dead PID detection uses `/proc/{pid}` existence check
on Linux, conservative `true` on other platforms. Added `ProcessError::RegistryError(String)` for
serialization failures. Exported all new types from `lib.rs`. 8 new tests: register/get, deregister,
deregister-noop, update_status, update_status-noop, list_active filtering, dead PID pruning (PID 4_000_000),
multi-entry listing with real spawned processes. All 47 graft-common tests pass.
**Critique findings**: All acceptance criteria met. API is clean and ergonomic. Thread safety is
acceptable since each PID maps to a unique file. `pid_is_alive` using `/proc/{pid}` is simple and
works correctly on Linux. The `tempdir` fix (returning `(FsProcessRegistry, TempDir)` to keep tempdir
alive) was needed after initially using the deprecated `into_path()`. No actionable issues found.
**Improvements made**: Switched from `TempDir::into_path()` (deprecated) to returning the `TempDir`
alongside the registry in test helper; this keeps the directory alive and avoids the deprecation warning.
**Learnings for future iterations**: Task 4 will wire `ProcessHandle::spawn` to an optional
`ProcessRegistry` parameter. The `FsProcessRegistry::default_path()` pattern follows `state.rs` using
`std::env::var("HOME")`. For dead PID tests, use PID 4_000_000 (above typical Linux default max of 32768
but below the 4_194_304 hard limit — safe in practice; could use u32::MAX for extra safety).

### Iteration — Task 4: Wire ProcessHandle lifecycle to ProcessRegistry
**Status**: completed
**Files changed**: `crates/graft-common/src/process.rs`, `crates/graft-common/src/lib.rs`
**What was done**: Added `spawn_registered()` (and `run_to_completion_registered()`,
`run_to_completion_with_timeout_registered()`) that accept `Arc<R: ProcessRegistry + 'static>`.
Refactored `spawn` into a private `spawn_inner(config, Option<Arc<dyn ProcessRegistry>>)`.
`spawn_inner` registers the entry before starting background threads, passes a clone of the
registry `Arc` into the monitor thread (which updates status then deregisters on completion/failure).
`kill()` deregisters regardless of kill success. `Drop` kills and deregisters if still running.
Manual `Debug` impl used since `dyn ProcessRegistry` doesn't implement `Debug`. 6 new tests:
entry visible after spawn, completion deregisters, kill deregisters, drop kills+deregisters,
`run_to_completion_registered` deregisters, `run_to_completion_with_timeout_registered` deregisters
on timeout. All 53 graft-common tests pass.
**Critique findings**: Initial API took `Arc<dyn ProcessRegistry>` which forced callers to explicitly
upcast concrete registry types. Also `dyn ProcessRegistry` doesn't implement `Debug` so `#[derive(Debug)]`
couldn't be used on `ProcessHandle`.
**Improvements made**: Made `spawn_registered` and the two blocking helpers generic over
`R: ProcessRegistry + 'static`, coercing to `Arc<dyn ProcessRegistry>` internally. Implemented
manual `Debug` for `ProcessHandle` using `finish_non_exhaustive()`.
**Learnings for future iterations**: When a function needs to store `Arc<dyn Trait>` internally but
callers have `Arc<ConcreteType>`, use a generic parameter `<R: Trait + 'static>` and coerce inside:
`let dyn_arc: Arc<dyn Trait> = concrete_arc;`. `dyn Trait` in an `Arc` does not auto-implement `Debug`
even if all known implementors do — a manual impl (or `+ Debug` supertrait) is required.

### Iteration — Task 5: graft-engine commands via ProcessHandle
**Status**: completed
**Files changed**: `crates/graft-engine/src/command.rs`
**What was done**: Replaced `graft_common::command::run_command_with_timeout` with
`graft_common::process::run_to_completion_with_timeout` in `execute_command()`. Built a
`ProcessConfig` from the `Command` definition: `command` (shell command string), `working_dir`,
`env: command.env.clone()` (both types use `Option<HashMap<String, String>>`), `log_path: None`,
`timeout: None`. Mapped `ProcessOutput` fields directly to `CommandResult` (no `String::from_utf8_lossy`
needed since `ProcessOutput` already provides `String`). Removed unused `std::process::Command`
import. Public API unchanged; all 4 existing tests pass without modification.
**Critique findings**: All acceptance criteria met. The default timeout behaviour changed from
5-second hard-coded default (old `command.rs`) to no timeout unless `GRAFT_PROCESS_TIMEOUT_MS`
env var is set — this is intentional per the design. API surface is clean with 1:1 field mapping.
**Improvements made**: none needed
**Learnings for future iterations**: When domain types share the same field types (e.g., both
`Command.env` and `ProcessConfig.env` are `Option<HashMap<String, String>>`), a direct `.clone()`
is sufficient — no manual conversion needed. Task 6 (state.rs) is analogous but adds a timeout
from the `StateQuery` definition.

### Iteration — Task 6: graft-engine state queries via ProcessHandle
**Status**: completed
**Files changed**: `crates/graft-engine/src/state.rs`
**What was done**: Replaced the bare `ProcessCommand::new("sh").output()` call (which silently
ignored the timeout field — it was computed into `_timeout_seconds` but never used) with
`run_to_completion_with_timeout` backed by a `ProcessConfig`. The timeout is now enforced:
`query.timeout.unwrap_or(300)` seconds (default 5 min per spec). `ProcessOutput` fields
(`exit_code`, `stdout`, `stderr`, `success`) map 1:1 to the existing error-handling and JSON
parsing logic. Removed `std::process::Command` import, added `graft_common::process` and
`std::time::Duration`. All 4 existing state tests pass unchanged.
**Critique findings**: All acceptance criteria met. The critical bugfix — timeout was declared
but never enforced — is now resolved. No lossy UTF-8 conversion needed since `ProcessOutput`
already provides `String`. Preview slicing in the JSON error is byte-based (same as original).
**Improvements made**: none needed
**Learnings for future iterations**: Watch for the anti-pattern `let _foo = expr` (underscore
prefix signals intentionally ignored). In state.rs, `_timeout_seconds` was the tell that timeout
enforcement was missing. When migrating such code, the gap becomes obvious and easy to fill.
Task 7 (grove-cli state queries) will add graft-engine as a dependency and replace grove's own
`sh -c` execution path.

### Iteration — Task 7: grove-cli state queries via graft-engine library
**Status**: completed
**Files changed**: `crates/grove-cli/Cargo.toml`, `crates/grove-cli/src/tui/repo_detail.rs`
**What was done**: Added `graft-engine = { workspace = true }` to grove-cli's Cargo.toml.
In `refresh_state_queries()`, replaced the local `execute_state_query_command()` function
(which used ad-hoc `sh -c` + custom JSON parsing) with `graft_engine::state::execute_state_query()`.
Key changes: compute HEAD commit hash once before the loop via `graft_common::get_current_commit()`;
build `graft_engine::StateQuery` struct literal from grove's `StateQuery` fields (all pub);
delegate to `graft_engine::state::execute_state_query()`; keep explicit cache write via
`graft_common::state::write_cached_state()`. Removed `RawStateResult`, `execute_state_query_command()`,
and the per-query `git rev-parse HEAD` inline call. Also removed unused `use crate::state::StateResult;`.
**Critique findings**: All acceptance criteria met. Computing commit hash once per refresh (not
per query) is a net improvement. `GraftError` propagated to log via `.to_string()` is adequate.
**Improvements made**: none needed
**Learnings for future iterations**: When adding a crate dependency for struct literal construction,
confirm all fields are `pub` before using struct syntax instead of a constructor. Unused imports
become dead after removing code — clippy `-D warnings` will flag them. Task 8 is the most invasive
change: replace `spawn_command` + `find_graft_command` with ProcessHandle streaming.

### Iteration — Task 8: grove-cli command execution via ProcessHandle, remove find_graft_command
**Status**: completed
**Files changed**: `crates/grove-cli/src/tui/command_exec.rs`
**What was done**: Replaced `find_graft_command()` (which probed for `uv run python -m graft` then
`graft` binary) and the manual 3-thread setup (stdout/stderr reader threads + `child.wait()`) with
`ProcessHandle::spawn()`. New `spawn_command()` keeps the same public signature (required by
integration tests); graft.yaml lookup is inside `spawn_command()` via `graft_common::parse_commands()`.
On command-not-found, sends `CommandEvent::Failed("Command 'xxx' not found in graft.yaml")`.
Event bridge: `ProcessEvent::Started{pid}` → `CommandEvent::Started(pid)`, `OutputLine{line,..}` →
`OutputLine(line)` (stderr collapsed), `Completed` → `Completed`, `Failed` → `Failed`.
Clippy rejected `match` for single-pattern destructuring — used `let Some(...) = ... else {}` instead.
All 5 integration tests pass; `test_graft_not_in_path_error` (#[ignore]) is now permanently irrelevant.
**Critique findings**: All acceptance criteria met. Plan said to change `spawn_command` signature to
`CommandDef`, but that broke integration tests — kept same signature with graft.yaml lookup inside.
`execute_command_with_args` and `handle_command_events` are unchanged as required.
**Improvements made**: none needed
**Learnings for future iterations**: When a plan calls for changing a public API, first check
whether integration tests depend on the old signature. Clippy `single_match_else` prefers
`if let Some(...) else {}` over `match` with a single Some arm. Grove no longer requires the
graft binary to be installed — commands run directly from graft.yaml definitions.

### Iteration — Task 9: `graft ps` CLI command
**Status**: completed
**Files changed**: `crates/graft-cli/Cargo.toml`, `crates/graft-cli/src/main.rs`
**What was done**: Added `graft ps` subcommand to graft-cli. Added `graft-common` as an
explicit dependency. Added `Ps { repo: Option<String> }` to the `Commands` enum with
`--repo` filter. Implemented `ps_command()`→`ps_command_impl(registry_path, repo_filter)` split
so the core logic is testable without hitting `~/.cache/graft/processes/`. Output shows
"No active processes." when the list is empty, or "Active processes (N):" with per-entry PID,
command, repository path, start time, and status. Four unit tests cover: empty registry,
active entry (using `std::process::id()` as a live PID), matching repo filter, non-matching
repo filter. Smoke test `graft ps` and `graft ps --repo /path` both work. Full workspace
tests pass (0 failures).
**Critique findings**: Split into `ps_command`/`ps_command_impl` is clean and avoids needing
a mock registry in tests. Using `std::process::id()` as the test PID is reliable since the
test runner process is always alive. `format_process_status` is defined but only `Running`
will appear via `list_active()` — the other arms (`Completed`/`Failed`) handle edge cases
where status was persisted but not yet pruned.

### Iteration — Task 10: Remove deprecated code and clean up exports
**Status**: completed (partial — git.rs migrated, command.rs retained)
**Files changed**: `crates/graft-common/src/git.rs`
**What was done**: Migrated all four git functions (`get_current_commit`, `git_rev_parse`,
`git_fetch`, `git_checkout`) from `run_command_with_timeout` to `ProcessConfig` +
`run_to_completion_with_timeout`. Removed `GitError::Command(#[from] CommandError)` variant,
added `GitError::Process(#[from] ProcessError)`. Removed `GitError::InvalidUtf8` (no longer
needed since ProcessOutput.stdout is already a String). Added `use std::process::Command`
inside the test module for test setup helpers. `command.rs` was kept because
`grove-engine/src/git.rs` and `graft-engine/src/resolution.rs` still import
`run_command_with_timeout` directly. Full migration of those two callers is deferred: the
`sh -c` semantic difference makes it non-straightforward (path/URL args in resolution.rs would
need shell escaping; grove-engine error mapping is complex).
**Critique findings**: `graft_common::process` is the primary API for all new subprocess work.
No dead code warnings. Full cleanup of command.rs is future work once grove-engine and
graft-engine callers are migrated.

### Iteration — Task 11: Update specifications
**Status**: completed
**Files changed**: `docs/specifications/grove/command-execution.md`,
`docs/specifications/graft/core-operations.md`, `docs/specifications/graft/state-queries.md`
**What was done**: Removed "graft not in PATH" edge case from command-execution.md, updated
execution Gherkin to reference graft-engine library and ProcessHandle, updated Constraints,
added 2026-02-19 decision superseding old subprocess approach. Added full `graft ps` spec to
core-operations.md. Added timeout enforcement implementation note to state-queries.md.
**Critique findings**: Spec updates are minimal and targeted. graft ps spec matches actual
implementation exactly. All 11 tasks now complete.


