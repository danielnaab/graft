---
status: done
created: 2026-03-02
depends_on: []
---

# Scion attach command

## Story

After starting a worker in a scion's runtime session, users need to connect to it —
observe progress, provide approvals, interact with an agent. `graft scion attach <name>`
connects the current terminal to the scion's runtime session (tmux today, future backends
later). On detach, control returns to the caller.

The `SessionRuntime::attach` method already exists and works. This slice adds engine-level
validation, a shared session ID helper, and the CLI wiring.

## Approach

Two layers of work:

1. **Engine helpers** — extract `scion_session_id(name: &str) -> String` in
   `graft-engine/src/scion.rs`. Currently `scion_start` and `scion_stop` both
   hardcode `format!("graft-scion-{name}")` inline. A shared helper prevents drift
   as more session-aware code is added (attach, session-cleanup, grove commands).
   Retrofit the existing callsites in `scion_start` and `scion_stop` to use it.

2. **Two-phase attach API + CLI** — add `scion_attach_check(repo_path, name, runtime)`
   in `graft-engine/src/scion.rs`. This function validates (scion name, worktree
   exists, session exists) and returns the session ID on success. It does NOT call
   `runtime.attach()` — that's left to the caller.

   The two-phase split exists because grove needs to suspend its TUI *between*
   validation and the blocking `runtime.attach()` call. If the engine encapsulated
   the full flow, the TUI would still be in raw mode / alternate screen when tmux
   takes over. The CLI doesn't have this problem but uses the same API for
   consistency: call `scion_attach_check`, then `runtime.attach(&session_id)`.

   Error ordering: scion-doesn't-exist takes priority over no-active-session.
   `graft scion attach typo-name` says "scion 'typo-name' does not exist" rather
   than the confusing "no active session."

No hooks, no config resolution. Attach is a direct runtime operation with validation.

This slice has no dependencies — `scion start/stop` and `SessionRuntime` are already
implemented.

## Acceptance Criteria

- `scion_session_id("retry-logic")` returns `"graft-scion-retry-logic"`
- Existing `scion_start` and `scion_stop` use `scion_session_id` (no inline format)
- `scion_attach_check` validates worktree exists before checking runtime session
- `scion_attach_check` returns the session ID string on success
- `graft scion attach <name>` calls `scion_attach_check`, then `runtime.attach()`
- If the scion doesn't exist, prints "scion '<name>' does not exist" and exits non-zero
- If the scion exists but no runtime session is active, prints "no active session for
  scion '<name>'" and exits non-zero
- On detach (tmux: `Ctrl-b d`), control returns to the caller's terminal
- `graft scion attach --help` shows usage
- `cargo test && cargo clippy -- -D warnings && cargo fmt --check` passes

## Steps

- [x] **Extract `scion_session_id` helper and retrofit callsites**
  - **Delivers** — shared session ID derivation, no inline format duplication
  - **Done when** — `pub fn scion_session_id(name: &str) -> String` returns
    `format!("graft-scion-{name}")`; `scion_start`, `scion_stop`, and `scion_list`
    (line 726) use it instead of inline `format!`; helper is exported from
    `graft-engine` for use by grove; all existing tests pass unchanged
  - **Files** — `crates/graft-engine/src/scion.rs`, `crates/graft-engine/src/lib.rs`

- [x] **Add `scion_attach_check` engine function and CLI command**
  - **Delivers** — validated attach with two-phase API for TUI compatibility
  - **Done when** — `scion_attach_check(repo_path, name, runtime: &impl SessionRuntime)`
    calls `validate_scion_name`, checks `worktree_path(repo, name).exists()` (error if
    not), calls `runtime.exists(&scion_session_id(name))` (error if no session), returns
    `Ok(session_id)` on success; `Attach { name: String }` variant added to
    `ScionCommands`; CLI handler follows the existing pattern (get repo_path, create
    `TmuxRuntime`, call `scion_attach_check`, then call `runtime.attach(&session_id)`);
    test with `MockRuntime` covers: scion doesn't exist → error, scion exists but no
    session → error, scion exists with session → returns session ID
  - **Files** — `crates/graft-engine/src/scion.rs`, `crates/graft-engine/src/lib.rs`,
    `crates/graft-cli/src/main.rs`
