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

