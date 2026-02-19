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

