---
status: done
created: 2026-03-02
depends_on:
  - scion-attach
  - scion-session-cleanup
---

# Grove scion commands

## Story

Grove acts as a switchboard for scion management — users should be able to list, create,
start, stop, prune, fuse, and attach to scions without leaving the TUI. This slice adds
scion awareness to grove, mirroring what graft already provides via CLI but with the
TUI's observation and navigation advantages.

The key design principle: grove triggers graft operations, it does not reimplement them.
All scion commands call into `graft_engine` functions (already a dependency of
`grove-cli`). The runtime session indicator (active/inactive) comes from
`SessionRuntime::exists`, with graceful degradation when tmux is not available.

## Approach

Five areas of work:

1. **Command dispatch** — add `:scion <subcommand>` and `:attach <name>` to grove's
   command parser in `command_line.rs`. Follows the existing pattern: add entries to
   `PALETTE_COMMANDS`, add variants to `CliCommand` enum (e.g., `ScionList`,
   `ScionCreate(String)`, `ScionStart(String)`, `Attach(String)`), handle in
   `parse_command()`.

2. **Scion list rendering** — `:scion list` calls `graft_engine::scion_list` and
   optionally checks `TmuxRuntime::exists` for each scion (using `scion_session_id`
   from the `scion-attach` slice). Renders as a table block in the transcript scroll
   buffer with columns: name, ahead/behind, last activity, dirty indicator, session
   status (● active / blank).

3. **Query/lifecycle commands** — split into two groups:
   - **Query**: `:scion list` (read-only, needs only `repo_path` + optional runtime)
   - **Mutations**: `:scion create <name>` needs `config + dep_configs`. `:scion start
     <name>` / `:scion stop <name>` need `runtime`. `:scion prune <name>` / `:scion
     fuse <name>` need `config + dep_configs + runtime + force`.

   Each command calls the corresponding `graft_engine::scion_*` function. Output
   rendered as text blocks in the scroll buffer showing success/failure.

   **Config and dep_configs loading**: grove already loads graft.yaml for command
   resolution (see `cmd_run` which uses `self.graft_loader`). For scion lifecycle
   commands, the handler loads `GraftConfig` from the selected repo's `graft.yaml`,
   then walks its `dependencies` entries to build `dep_configs`. This follows the
   same pattern the CLI uses.

   **Repo selection guard**: all scion commands check
   `self.context.selected_repo_path` first. If no repo is selected, render
   "No repository selected" as a status error and return early (same pattern as
   `:run`).

4. **Force semantics in TUI** — TUI commands don't have CLI-style flags. For fuse and
   prune, grove passes `force: false` by default. If the engine returns the
   session-active error, grove renders the error message (which suggests `--force` or
   `graft scion stop`) and the user can either `:scion stop <name>` first or use the
   CLI with `--force`. This avoids inventing a TUI flag syntax and keeps the TUI
   interaction simple.

5. **Attach** — `:attach <name>` is a new TUI pattern. Unlike `:run` (which spawns a
   background thread and streams output into the scroll buffer), `:attach` must yield
   the entire terminal to the runtime session. This is novel in grove — no existing
   command does TUI suspend/resume. The flow uses the two-phase API from scion-attach:
   (a) call `scion_attach_check` (validates scion + session, returns session ID),
   (b) on success, suspend the TUI (leave alternate screen, disable raw mode),
   (c) call `runtime.attach(&session_id)` which blocks until detach,
   (d) re-enable raw mode, re-enter alternate screen, full redraw + state refresh.
   On validation error, render in scroll buffer without suspending.

## Acceptance Criteria

- All scion commands check for a selected repo and show "No repository selected" if none
- `:scion list` shows a table with name, ahead/behind, last commit time, dirty, session
  status
- `:scion create <name>` creates a scion and shows confirmation
- `:scion start <name>` starts a worker session and shows confirmation
- `:scion stop <name>` stops a worker session and shows confirmation
- `:scion prune <name>` removes a scion; if session active, shows error suggesting
  `:scion stop` or CLI `--force`
- `:scion fuse <name>` fuses a scion; same session-active behavior as prune
- `:attach <name>` suspends the TUI, attaches to the tmux session, resumes TUI on detach
- `:attach <name>` shows an error if the scion or runtime session doesn't exist
- When tmux is not installed, session status column is absent from list, and
  `:attach`/`:scion start`/`:scion stop` show a "no runtime available" error
- Tab completion works for scion subcommands and scion names
- `cargo test && cargo clippy -- -D warnings && cargo fmt --check` passes

## Steps

- [x] **Add `:scion` and `:attach` command parsing to grove command dispatch**
  - **Delivers** — command routing for scion subcommands
  - **Done when** — `PALETTE_COMMANDS` includes entries for `:scion list`,
    `:scion create`, `:scion start`, `:scion stop`, `:scion prune`, `:scion fuse`,
    and `:attach`; `CliCommand` enum has variants for each (e.g., `ScionList`,
    `ScionCreate(String)`, `Attach(String)`); `parse_command()` handles
    `:scion <sub> <name>` and `:attach <name>` parsing; unrecognized subcommands show
    available scion subcommands
  - **Files** — `crates/grove-cli/src/tui/command_line.rs`

- [x] **Implement `:scion list` with session status**
  - **Delivers** — scion overview in grove TUI
  - **Done when** — handler checks `self.context.selected_repo_path` (early return if
    none); calls `graft_engine::scion_list(repo_path, runtime)` where runtime is
    `TmuxRuntime::new().ok()` as `Option<&dyn SessionRuntime>`; renders a table block
    in the scroll buffer with columns: name, ahead/behind (e.g., "3↑ 0↓"), last
    commit (relative time), dirty indicator, session status ("● active" or blank);
    when tmux is unavailable, session column is omitted; empty list shows "no scions"
    message
  - **Files** — `crates/grove-cli/src/tui/transcript.rs`,
    `crates/grove-cli/src/tui/formatting.rs`

- [x] **Implement `:scion create/start/stop` handlers**
  - **Delivers** — scion creation and session management from grove
  - **Done when** — all three check `self.context.selected_repo_path` first;
    `:scion create <name>` loads `GraftConfig` from repo's graft.yaml and assembles
    `dep_configs` by walking dependency entries, calls `scion_create`, shows "Created
    scion '<name>'" or error; `:scion start <name>` creates `TmuxRuntime` (error if
    unavailable), loads config, calls `scion_start`, shows confirmation with session
    name; `:scion stop <name>` creates `TmuxRuntime` (error if unavailable), calls
    `scion_stop`, shows confirmation
  - **Files** — `crates/grove-cli/src/tui/transcript.rs`

- [x] **Implement `:scion prune/fuse` handlers with session-active error relay**
  - **Delivers** — lifecycle operations that surface session guard errors in the TUI
  - **Done when** — both check `self.context.selected_repo_path` first; load
    `GraftConfig` and assemble `dep_configs`; attempt `TmuxRuntime::new()` (pass
    `Some(&runtime as &dyn SessionRuntime)` or `None`); call engine functions with
    `force: false`; on success, show confirmation; on session-active error, render the
    error message (which suggests `:scion stop` or CLI `--force`); on other errors,
    show the error message
  - **Files** — `crates/grove-cli/src/tui/transcript.rs`

- [x] **Implement `:attach <name>` with TUI suspend/resume**
  - **Delivers** — terminal handoff to runtime session (new TUI pattern)
  - **Done when** — handler checks `self.context.selected_repo_path` first; creates
    `TmuxRuntime` (error if unavailable); calls `scion_attach_check(repo_path, name,
    &runtime)` which returns session ID or error; on error, renders in scroll buffer
    without suspending; on success: executes `crossterm::execute!(stdout(),
    LeaveAlternateScreen)`, calls `crossterm::terminal::disable_raw_mode()`, calls
    `runtime.attach(&session_id)` (blocks until detach), calls
    `crossterm::terminal::enable_raw_mode()`, executes `crossterm::execute!(stdout(),
    EnterAlternateScreen)`, triggers `terminal.clear()` and full state refresh
  - **Files** — `crates/grove-cli/src/tui/transcript.rs`

- [x] **Add tab completion for scion names**
  - **Delivers** — ergonomic scion name entry
  - **Done when** — after `:scion start `, `:scion stop `, `:scion prune `,
    `:scion fuse `, `:attach `, tab completion offers known scion names (from cached
    scion list or by calling `scion_list`); `:scion create ` offers no name completion
    (the user types a new name); follows existing completion patterns in the prompt
    module
  - **Files** — `crates/grove-cli/src/tui/prompt.rs`
