---
status: draft
created: 2026-02-26
depends_on: [sequence-declarations, command-run-logging]
---

# Make long-running sessions observable in grove (elapsed time, log access, re-attach)

## Story

Long-running commands like `implement-verified` are invisible while running in grove.
The CommandOutput view shows no indication of elapsed time or whether Claude is alive.
Pressing `q` dismisses the view but leaves the process running with no way to return
to it. The run log is written to disk but inaccessible from inside grove. These gaps
make it impossible to monitor a session without leaving grove to open a separate
terminal. This slice surfaces that information directly in the TUI.

## Approach

Three targeted additions to grove:

**1. Elapsed time in CommandOutput header**

Track `command_start_time: Option<std::time::Instant>` in `App`. Set it when the
render cycle's passive event drain receives the first `CommandEvent` for a new run
(i.e. when `command_event_rx` transitions from `None` to `Some` and produces its
first event); clear it when a completion or failure event is received. There is no
`handle_command_events` function — command events are drained passively during each
render frame. The header string includes the elapsed time, updated each render frame:

```
┌ Running: » software-factory:implement-verified (2m 34s) (j/k: scroll, t: log, q: background) ─┐
```

Format: `(Xs)` for under a minute, `(Xm Ys)` thereafter.

**2. `t` key shows run log path**

In CommandOutput, `t` sets `self.status_message` to the run log file path (e.g.
`"Log: ~/.cache/graft/.../runs/20260226-142300-implement-verified.log"`). This
lets the user quickly `tail -f` the path in a separate terminal. The path is
available via `current_log_path: Option<PathBuf>` stored in `App` when the command
starts (from `RunLogging.log_path`).

**3. `q` backgrounds; re-attach from repo detail**

Change `q` in CommandOutput to "background" rather than close when `CommandState::
Running`: pop the view but preserve `output_lines`, `command_event_rx`, and
`running_command_pid`. The in-progress command continues running and accumulating
output into `output_lines`.

In the repo detail view, when `running_command_pid` is `Some`:
- The status bar shows `(1 command running)` appended to the repo name
- The hint bar shows `r: re-attach`
- Pressing `r` pushes `View::CommandOutput` to re-open the output view, showing
  all accumulated output since the command started

When the command completes while backgrounded, the indicator disappears on the next
render and `output_lines` retains the final output for review.

**Event draining while backgrounded**: `command_event_rx` remains active after
`q`-to-background and the render cycle continues draining events into `output_lines`
each frame — no special handling needed. The passive event drain loop runs regardless
of whether `View::CommandOutput` is on the view stack.

CommandOutput hint bar updates: `q: background` while running; `q: close` after
completion.

## Acceptance Criteria

- CommandOutput header shows elapsed time updated each render cycle, e.g. `(1m 23s)`;
  time stops updating when command completes or fails
- `t` key in CommandOutput sets the status bar message to the full run log path
- `q` while running: dismisses CommandOutput view but does NOT clear `output_lines`,
  `command_event_rx`, or `running_command_pid`
- `q` after completion: clears all command state (existing close behavior)
- After pressing `q` while running, the repo detail status bar shows
  `(1 command running)`
- `r` in repo detail view re-opens CommandOutput, showing all accumulated output
  (no lines lost)
- When the backgrounded command completes, the `(running)` indicator disappears on
  the next render
- CommandOutput hint bar shows `q: background` while running, `q: close` after
- `cargo test && cargo clippy -- -D warnings && cargo fmt --check` passes

## Steps

- [ ] **Add elapsed time tracking and `t` log key to CommandOutput**
  - **Delivers** — users can see how long a command has been running and access the
    run log path without leaving grove
  - **Done when** — `App` gains `command_start_time: Option<std::time::Instant>` set
    when the command starts (when `execute_command_with_args` stores the Instant before
    spawning the process — not in a `handle_command_events` function, which does not
    exist; events are drained passively in the render cycle) and cleared when a
    completion/failure event is received during event drain; `App` gains `current_log_path: Option<PathBuf>` set when a command
    starts (thread spawned in `execute_command_assembled` / `execute_command_with_args`
    — pass log path via a channel or store before spawn); the CommandOutput title
    rendered in `render.rs` includes elapsed time when `command_start_time` is Some
    and command is running, formatted as `(Xs)` or `(Xm Ys)`;
    `handle_key_command_output()` handles `t`: sets `self.status_message` to
    `format!("Log: {}", path.display())`; unit test: mock start time and assert
    formatted elapsed output; hint bar updated for `t`
  - **Files** — `crates/grove-cli/src/tui/mod.rs`,
    `crates/grove-cli/src/tui/render.rs`,
    `crates/grove-cli/src/tui/command_exec.rs`,
    `crates/grove-cli/src/tui/hint_bar.rs`

- [ ] **Add `q`-to-background and re-attach from repo detail**
  - **Delivers** — users can leave the CommandOutput view without killing the process
    and re-attach to see accumulated output
  - **Done when** — `handle_key_command_output()` handles `q`: if `CommandState::
    Running`, calls `self.pop_view()` without clearing `output_lines`,
    `command_event_rx`, or `running_command_pid`; if `CommandState::Completed` or
    `CommandState::Failed`, clears all state (existing behavior); `handle_key_repo_
    detail()` handles `r`: if `running_command_pid` is Some, pushes
    `View::CommandOutput`; `render_repo_detail()` or `render_status_bar()` appends
    `(1 command running)` when `running_command_pid` is Some; hint bar in repo detail
    shows `r: re-attach` when `running_command_pid` is Some; hint bar in CommandOutput
    shows `q: background` vs `q: close` based on `CommandState`; unit tests assert
    background and re-attach behavior
  - **Files** — `crates/grove-cli/src/tui/mod.rs`,
    `crates/grove-cli/src/tui/render.rs`,
    `crates/grove-cli/src/tui/hint_bar.rs`
