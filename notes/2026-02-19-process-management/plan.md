---
status: working
purpose: "Implementation plan for unified process management — task tracking for Ralph loop"
---

# Unified Process Management Plan

Replacing three independent subprocess execution paths (graft-common `run_command_with_timeout`,
graft-engine blocking `.output()`, grove-cli `find_graft_command` + shell out) with a shared
`ProcessHandle` in graft-common, a `ProcessRegistry` for global process tracking, and unified
execution through graft-engine as a library.

## How to use this plan

Each task is a self-contained unit of work. Read the listed specs and code, implement the
capability, verify, and mark complete. Tasks are ordered for incremental migration — each
task leaves all crates compiling and tests passing.

Key constraints:
- **Bridge pattern**: When replacing an execution path, add the new alongside the old first,
  then remove the old in a later task.
- **Test continuity**: Existing tests pass unchanged unless a task explicitly replaces them.
- **Design reference**: `notes/2026-02-19-unified-process-management.md`

## Design references

- `notes/2026-02-19-unified-process-management.md` — three-layer design (ProcessHandle, ProcessRegistry, unified execution)

## Resolved design conflicts

(Record conflicts you discover and how you resolved them here)

## Design decisions made during implementation

- **`spawn` takes `&ProcessConfig`** (not by value): clippy pedantic flags pass-by-value when
  the value is not consumed. Since ProcessConfig fields are used via borrow, taking a reference
  avoids unnecessary clones. Callers pass `&config`.
- **Monitor thread uses `try_wait()` polling** (10ms interval) instead of blocking `wait()`:
  allows `kill()` to acquire the child mutex without deadlocking against the monitor.
- **`drop(thread::spawn(...))` for monitor thread**: avoids clippy `let_underscore_must_use`
  warning on the detached `JoinHandle`.
- **Private helpers take `&mpsc::Receiver` not by value**: clippy pedantic rejects
  pass-by-value when the value is only used through shared references. `recv_timeout` takes
  `&self`, so `&Receiver` is sufficient. Use `for event in rx` (not `rx.iter()`) since
  clippy `explicit_iter_loop` prefers the shorter form for `&Receiver`.
- **`#[allow(clippy::too_many_lines)]` on `spawn`**: the function is inherently long due to
  three thread setups. The allow is targeted on the single function.
- **`spawn_registered` and `*_registered` helpers are generic over `R: ProcessRegistry + 'static`**:
  avoids requiring callers to explicitly upcast `Arc<FsProcessRegistry>` to `Arc<dyn ProcessRegistry>`.
  Internally casts to `Arc<dyn ProcessRegistry>` via unsized coercion in `spawn_inner`.
- **`ProcessHandle` uses manual `Debug` impl**: `dyn ProcessRegistry` doesn't implement `Debug`,
  so `#[derive(Debug)]` can't be used. Manual impl uses `finish_non_exhaustive()` and shows
  registry presence as `Some("<registry>")` or `None`.
- **Registry registered before background threads start**: ensures a `Running` entry is visible
  to callers immediately after `spawn_registered` returns, with no race window.

---

## Phase 1: ProcessHandle Foundation (Tasks 1–3)

### Task 1: ProcessEvent, ProcessHandle with streaming output
- [x] Implement ProcessHandle in graft-common with streaming via std threads + mpsc
- **Code**: `crates/graft-common/src/process.rs` (new), `crates/graft-common/src/lib.rs`
- **Design**: `notes/2026-02-19-unified-process-management.md` (ProcessHandle section)
- **Acceptance**:
  - New `process.rs` module defines:
    - `ProcessEvent` enum: `Started { pid: u32 }`, `OutputLine { line: String, is_stderr: bool }`, `Completed { exit_code: i32 }`, `Failed { error: String }`
    - `ProcessError` enum (thiserror): `SpawnFailed`, `KillFailed`, `Timeout`, `IoError`
    - `ProcessConfig` struct: `command: String`, `working_dir: PathBuf`, `env: Option<HashMap<String, String>>`, `log_path: Option<PathBuf>`, `timeout: Option<Duration>`
    - `ProcessHandle` struct with: `spawn(config) -> Result<(ProcessHandle, Receiver<ProcessEvent>)>`, `pid()`, `kill()`, `is_running()`
  - Spawn uses `sh -c <command>`, creates stdout/stderr reader threads, sends events on channel
  - Reader threads join before `Completed` event is sent (output ordering preserved)
  - `lib.rs` exports `pub mod process` and re-exports key types
  - Tests: spawn echo, stderr capture, non-zero exit, spawn failure, kill long-running process
  - `cargo fmt --check && cargo clippy -- -D warnings && cargo test` passes

### Task 2: Log capture and sync convenience wrappers
- [x] Add log file tee and blocking `run_to_completion` functions
- **Code**: `crates/graft-common/src/process.rs`
- **Design**: `notes/2026-02-19-unified-process-management.md` (ProcessHandle section)
- **Acceptance**:
  - When `config.log_path` is set, output lines are tee'd to log file (append mode)
  - `run_to_completion(config) -> Result<ProcessOutput>` blocks and collects all output into `ProcessOutput { exit_code, stdout, stderr, success }`
  - `run_to_completion_with_timeout(config) -> Result<ProcessOutput>` adds timeout support — kills process and returns `ProcessError::Timeout` if exceeded; respects `config.timeout` then env var `GRAFT_PROCESS_TIMEOUT_MS` then unlimited
  - Tests: log file written, `run_to_completion` collects output, timeout triggers on slow command
  - `cargo fmt --check && cargo clippy -- -D warnings && cargo test` passes
- **Bridge note**: `run_command_with_timeout` in `command.rs` remains untouched — both paths coexist

### Task 3: ProcessRegistry trait and FsProcessRegistry
- [x] Define trait and filesystem implementation with PID liveness pruning
- **Code**: `crates/graft-common/src/process.rs` (or split into `process/` module directory)
- **Design**: `notes/2026-02-19-unified-process-management.md` (ProcessRegistry section)
- **Acceptance**:
  - `ProcessEntry` struct: `pid`, `command`, `repo_path: Option<PathBuf>`, `start_time` (ISO 8601), `log_path: Option<PathBuf>`, `status: ProcessStatus`
  - `ProcessStatus` enum: `Running`, `Completed { exit_code }`, `Failed { error }`
  - `ProcessRegistry` trait: `register()`, `deregister()`, `list_active()`, `get()`, `update_status()`
  - `FsProcessRegistry`: writes `{pid}.json` to `~/.cache/graft/processes/`, prunes dead PIDs on `list_active()` via `kill(pid, 0)` or `/proc/{pid}` check
  - Tests (using tempdir): register+list, deregister+list, dead PID pruning, multiple entries
  - `cargo fmt --check && cargo clippy -- -D warnings && cargo test` passes

## Phase 2: Registry Integration (Task 4)

### Task 4: Wire ProcessHandle lifecycle to ProcessRegistry
- [x] Auto-register on spawn, deregister on completion/kill/drop
- **Code**: `crates/graft-common/src/process.rs`
- **Design**: `notes/2026-02-19-unified-process-management.md` (ProcessRegistry section)
- **Acceptance**:
  - `ProcessHandle::spawn` accepts optional registry parameter (e.g., `spawn_registered(config, &dyn ProcessRegistry)` variant or optional field)
  - On spawn: registers `ProcessEntry` with `Running` status
  - On completion: updates status, then deregisters
  - On kill: deregisters
  - `Drop` impl: if still running and registered, kills and deregisters
  - `run_to_completion` / `run_to_completion_with_timeout` also accept optional registry
  - Tests: spawn with registry shows entry, completion clears it, kill clears it
  - Existing tests without registry pass unchanged
  - `cargo fmt --check && cargo clippy -- -D warnings && cargo test` passes

## Phase 3: Migrate graft-engine (Tasks 5–6)

### Task 5: graft-engine commands via ProcessHandle
- [x] Replace `run_command_with_timeout` with `run_to_completion_with_timeout` in command.rs
- **Code**: `crates/graft-engine/src/command.rs`
- **Existing**: Currently uses `graft_common::command::run_command_with_timeout` (see lines 64–66)
- **Acceptance**:
  - `execute_command()` internally uses `ProcessConfig` + `run_to_completion_with_timeout`
  - Sets `command`, `working_dir`, `env` from `Command` definition — same shell construction as current code
  - Public API unchanged: `execute_command()`, `execute_command_by_name()`, `CommandResult`
  - All 4 existing tests pass without modification
  - `cargo fmt --check && cargo clippy -- -D warnings && cargo test` passes
- **Bridge note**: Internal rewrite only. External API identical.

### Task 6: graft-engine state queries via ProcessHandle
- [x] Replace direct `.output()` with `run_to_completion_with_timeout` in state.rs
- **Code**: `crates/graft-engine/src/state.rs`
- **Existing**: Currently uses `ProcessCommand::new("sh").output()` directly (no timeout)
- **Acceptance**:
  - `execute_state_query()` internally uses `ProcessConfig` + `run_to_completion_with_timeout`
  - Respects timeout from `StateQuery` (timeout field, default 300s per spec)
  - Public API unchanged: `execute_state_query()`, `get_state()`, `StateResult`
  - All existing tests pass without modification
  - Timeout protection now actually works (was missing before)
  - `cargo fmt --check && cargo clippy -- -D warnings && cargo test` passes

## Phase 4: Migrate grove-cli (Tasks 7–8)

### Task 7: grove-cli state queries via graft-engine library
- [x] Replace grove's `sh -c` state query execution with `graft_engine::execute_state_query()`
- **Code**: `crates/grove-cli/Cargo.toml`, `crates/grove-cli/src/tui/repo_detail.rs`
- **Specs**: `docs/specifications/graft/state-queries.md`
- **Design**: `notes/2026-02-19-unified-process-management.md` (Consumer Patterns — grove-cli)
- **Acceptance**:
  - `grove-cli/Cargo.toml` adds `graft-engine` dependency
  - `execute_state_query_command()` in `repo_detail.rs` replaced with call to `graft_engine::state::execute_state_query()` or `get_state()`
  - Removes local shell execution, `git rev-parse`, and JSON parsing — graft-engine handles all of this
  - Caching still works (graft-engine + graft-common handle it)
  - All grove-cli tests pass
  - `cargo fmt --check && cargo clippy -- -D warnings && cargo test` passes
- **Bridge note**: Grove's state query types may need a thin adapter to graft-engine types

### Task 8: grove-cli command execution via ProcessHandle, remove find_graft_command
- [x] Replace `spawn_command` + `find_graft_command` with ProcessHandle streaming
- **Code**: `crates/grove-cli/src/tui/command_exec.rs`, `crates/grove-cli/src/tui/mod.rs`
- **Specs**: `docs/specifications/grove/command-execution.md`
- **Design**: `notes/2026-02-19-unified-process-management.md` (Consumer Patterns — grove-cli)
- **Acceptance**:
  - `find_graft_command()` removed entirely
  - `spawn_command()` rewritten:
    - Accepts `CommandDef` (from graft-common config parsing) instead of command name + graft binary
    - Uses `ProcessHandle::spawn()` to create subprocess with `sh -c <run_command>`
    - Bridges `ProcessEvent` to existing `CommandEvent` channel (1:1 mapping, with `OutputLine.is_stderr` collapsed)
  - `execute_command_with_args` loads graft.yaml via `graft_common::parse_commands()`, looks up command, passes `CommandDef` to spawn
  - TUI event handling (`handle_command_events`) unchanged — same `CommandEvent` flow
  - All grove-cli tests pass
  - `cargo fmt --check && cargo clippy -- -D warnings && cargo test` passes
- **Bridge note**: `CommandEvent` kept as grove-cli's internal type (may alias `ProcessEvent` later). This is the most invasive change.

## Phase 5: Observability and Cleanup (Tasks 9–11)

### Task 9: `graft ps` CLI command
- [ ] Add process listing command to graft-cli
- **Code**: `crates/graft-cli/src/main.rs` (or commands module), `crates/graft-cli/Cargo.toml`
- **Specs**: `docs/specifications/graft/core-operations.md` (to be updated)
- **Design**: `notes/2026-02-19-unified-process-management.md` (Consumer Patterns — graft-cli)
- **Acceptance**:
  - `graft ps` subcommand added via clap
  - Lists all active processes from `FsProcessRegistry::default()`
  - Shows: PID, command, repo path, start time, status
  - Supports `--repo <path>` filter (optional, filter by repo_path)
  - Prunes stale entries automatically on list
  - Tests: command parsing, output format with mock registry entries
  - `cargo fmt --check && cargo clippy -- -D warnings && cargo test` passes

### Task 10: Remove deprecated code and clean up exports
- [ ] Remove old execution paths, update graft-common exports
- **Code**: `crates/graft-common/src/command.rs`, `crates/graft-common/src/lib.rs`, `crates/graft-common/src/git.rs`
- **Design**: `notes/2026-02-19-unified-process-management.md` (What Goes Away)
- **Acceptance**:
  - If `run_command_with_timeout` has no remaining callers: remove it and `command.rs` module
  - If `git.rs` still calls it: either migrate git.rs to ProcessHandle or keep `command.rs` with `#[deprecated]` (prefer migrate if straightforward)
  - `graft_common::process` is the primary execution API, cleanly exported
  - No dead code warnings in any crate
  - All 423+ tests pass
  - `cargo fmt --check && cargo clippy -- -D warnings && cargo test` passes

### Task 11: Update specifications
- [ ] Update specs to reflect unified process management architecture
- **Files**: `docs/specifications/grove/command-execution.md`, `docs/specifications/graft/core-operations.md`, `docs/specifications/graft/state-queries.md`
- **Acceptance**:
  - `command-execution.md`: Remove "graft not in PATH" edge case; document that grove calls graft-engine as library; remove subprocess coupling language
  - `core-operations.md`: Add `graft ps` command specification
  - `state-queries.md`: Note timeout protection is now enforced (was declared but not implemented)
  - All specs internally consistent, no references to removed concepts
  - No broken cross-references
