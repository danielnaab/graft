---
status: done
created: 2026-03-02
depends_on:
  - scion-attach
---

# Scion session cleanup on fuse and prune

## Story

When a scion has an active runtime session (e.g., a worker agent running in tmux),
`graft scion fuse` or `graft scion prune` would destroy the worktree out from under
the running process. This is destructive — the agent loses its working directory
mid-task, potentially leaving partial work uncommitted.

This slice adds a session-awareness guard: fuse and prune check for an active runtime
session and refuse to proceed unless `--force` is passed. This follows graft's
"explicit over implicit" pattern — the human decides whether to stop the session first
or force through.

Depends on `scion-attach` because that slice extracts `scion_session_id`, which this
slice uses for session existence checks.

## Approach

Two areas of work:

1. **Engine** — `scion_fuse` and `scion_prune` gain an optional `runtime` parameter
   and a `force` flag. The runtime parameter uses `Option<&dyn SessionRuntime>` (dynamic
   dispatch), matching the convention established by `scion_list`. When runtime is
   provided, they call `runtime.exists(&scion_session_id(name))` before proceeding. If
   a session is active and `force` is false, return an error explaining that a session
   is running and suggesting `--force` or `graft scion stop` first. When `force` is
   true and a session exists, call `runtime.stop()` before proceeding. When runtime is
   `None`, proceed without checking (graceful degradation when tmux is not installed).

   This is a signature change — existing callers (CLI handlers for fuse/prune, existing
   tests) are updated in the same step.

   Note: `force: true` stops the session, but fuse's existing dirty-check still runs
   afterward. If the agent left uncommitted work, fuse will stop the session
   successfully then fail with "worktree has uncommitted changes." This is correct
   (two independent safety checks), not a bug — the user can commit or discard the
   changes and retry fuse without `--force` (since the session is already stopped).

2. **CLI + tests** — add `--force` flag to `Fuse` and `Prune` variants in
   `ScionCommands`. Construct `TmuxRuntime` with graceful fallback: if
   `TmuxRuntime::new()` succeeds, pass `Some(&runtime as &dyn SessionRuntime)`; if
   tmux is unavailable, pass `None` (no error, no session check). Tests use the
   existing `MockRuntime` in `graft-engine/src/scion.rs`.

## Acceptance Criteria

- `graft scion fuse <name>` with an active session prints "scion '<name>' has an
  active runtime session; stop it first or use --force" and exits non-zero
- `graft scion fuse <name> --force` stops the session then proceeds with fuse
- `graft scion prune <name>` same behavior as fuse regarding session guard
- `graft scion prune <name> --force` stops the session then proceeds with prune
- Without a runtime backend (tmux not installed), fuse/prune proceed as before
- All existing callers compile with updated signatures
- `cargo test && cargo clippy -- -D warnings && cargo fmt --check` passes

## Steps

- [x] **Add session guard to `scion_fuse` and `scion_prune`, update all callers**
  - **Delivers** — safety check preventing destruction of active sessions
  - **Done when** — `scion_fuse` and `scion_prune` accept additional parameters
    `runtime: Option<&dyn SessionRuntime>` and `force: bool`; when runtime is `Some`
    and `runtime.exists(&scion_session_id(name))` returns true: if `force`, call
    `runtime.stop()` then proceed; if not `force`, return error with message suggesting
    `--force` or `graft scion stop`; when runtime is `None`, skip session check;
    CLI handlers for fuse/prune updated in the same step: attempt
    `TmuxRuntime::new()`, pass `Some(&runtime)` or `None`, add
    `#[arg(long)] force: bool` to both `Fuse` and `Prune` variants, pass `force`
    through; all existing engine tests updated to pass `None, false`
  - **Files** — `crates/graft-engine/src/scion.rs`, `crates/graft-cli/src/main.rs`

- [x] **Add engine tests for session guard behavior**
  - **Delivers** — coverage of all session guard branches
  - **Done when** — tests using the existing `MockRuntime` verify: (1) active session
    + `force: false` → error with message containing "--force"; (2) active session +
    `force: true` → `runtime.stop()` called, operation proceeds; (3) no active
    session → operation proceeds regardless of `force`; (4) `runtime: None` → operation
    proceeds (no session check); tests cover both fuse and prune
  - **Files** — `crates/graft-engine/src/scion.rs`
