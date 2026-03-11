---
status: draft
created: 2026-03-10
depends_on:
  - command-run-logging
---

# Run History & Log Viewer

## Story

As a developer using graft to orchestrate parallel scion workstreams,
I lose visibility into what happened after a command finishes and
scrolls off the grove transcript. When managing 3+ scions running
agent loops, I need to quickly answer: "what ran, when, and did it
succeed?" — without browsing `~/.cache/` manually.

The `command-run-logging` slice built full persistence infrastructure
(`RunMeta`, `list_runs()`, `read_run_log()`) but left the UI
unimplemented. This slice surfaces that data in both the grove TUI
(`:logs` command) and the graft CLI (`graft logs`).

## UX

### Grove TUI: `:logs`

```
┌─ Recent Runs ─────────────────────────────────────────────────────┐
│  Command                   Args            Time       Status     │
│  software-factory:agent    grove-grouped…  3m ago     ok         │
│  software-factory:verify                   12m ago    failed     │
│  software-factory:agent    grove-entity…   1h ago     ok         │
│  software-factory:plan                     2h ago     ok         │
└───────────────────────────────────────────────────────────────────┘
```

- Actionable table: selecting a row pushes the full log content as a
  read-only text block into the scroll buffer
- `:logs` shows last 20 runs for the selected repo
- `:logs <filter>` filters by command name prefix (e.g., `:logs agent`)
- Status column: green "ok", red "failed", yellow "running"
- Duration column shows elapsed time (e.g., "2m 34s") when both
  start and end times are available

### Graft CLI: `graft logs`

```bash
# List recent runs (default 20, newest first)
graft logs

# Limit results
graft logs --limit 5

# Filter by command name
graft logs --command agent

# View a specific run's log content (by index from list, 1-based)
graft logs show 1

# Machine-readable output
graft logs --json
```

Output format (text):
```
#  Command                      Time       Duration  Status
1  software-factory:agent       3m ago     2m 34s    ok
2  software-factory:verify      12m ago    42s       failed
3  software-factory:agent       1h ago     5m 3s     ok
```

`graft logs show 1` prints the full log content to stdout, suitable
for piping to `less`, `grep`, etc.

## Approach

### Shared duration formatting

Extract the existing `format_elapsed(Duration)` from
`scroll_buffer.rs` (currently private, formats as "42s" or "2m 34s")
into `graft-common` as a public `format_duration(Duration) -> String`.
Add `RunMeta::duration_display()` which parses `start_time` and
`end_time`, computes the difference as a `Duration`, and delegates
to `format_duration`. Returns `None` when `end_time` is absent
(still running). Both CLI and TUI use this method.

### Workspace resolution

**graft-cli** uses `repo_name` as both workspace and repo name
(Stage 1 simplification — see `run_current_repo_command` line 1830:
`CommandContext::local(base_dir, &repo_name, &repo_name, false)`).
`graft logs` follows the same pattern: `find_graft_yaml()` →
`base_dir.file_name()` → use as both workspace and repo name.

**grove-cli** uses a separate workspace name from its config file.
Logs written by grove live under a different cache path than logs
written by `graft run`. This is inherent to the current architecture
and acceptable: `graft logs` shows CLI-originated runs; grove `:logs`
shows grove-originated runs. Each tool sees its own history.

Error case: when no `graft.yaml` is found, `graft logs` exits with
a clear message (same pattern as `graft run`).

### `graft logs` CLI command

Add a `Logs` subcommand to graft-cli's `Commands` enum with two
subcommands via clap:

- **list** (default, no subcommand): calls `list_runs()`, renders
  as a formatted table or JSON. `--command` filter applied after
  listing (case-insensitive prefix match on `RunMeta.command`).
  `--limit` controls result count (default 20).

- **show \<N\>**: takes a 1-based index, re-fetches `list_runs()`
  with the same limit to resolve the entry, then calls
  `read_run_log()` to print content to stdout. The index is
  ephemeral (changes as new runs are added) — this is acceptable
  for interactive use. Errors clearly on out-of-range index or
  missing log file.

### `:logs` grove TUI command

Add `Logs(Option<String>)` variant to `CliCommand`. The handler:

1. Calls `list_runs(workspace, repo, 20)` to get recent runs
2. Optionally filters by command name prefix
3. Builds a `ContentBlock::Table` with actionable rows
4. Each row's action is a `CliCommand::LogView(log_file)` — the
   variant carries only the log filename string; the handler reads
   `self.workspace_name` and repo path from app state at dispatch
   time (consistent with all other command handlers)

Requires updates to three places in `command_line.rs`:
- `CliCommand` enum (add `Logs` and `LogView` variants)
- `parse_command()` match arm for `"logs"` / `"l"`
- `PALETTE_COMMANDS` array (new entry with description)

### `CliCommand::LogView` handler

When a user selects a row in the logs table:

1. Reads workspace from `self.workspace_name`, repo from
   `self.context.selected_repo_path`
2. Calls `read_run_log(workspace, repo, log_file)` to get content
3. Caps output at 10,000 lines (matching the `Running` block limit
   in `scroll_buffer.rs`) with a truncation notice at the top
4. Splits into lines, wraps in `Line<'static>` spans
5. Pushes a `ContentBlock::Text` block with a title like
   "Log: software-factory:agent (3m ago)"
6. Auto-scrolls to the new block
7. Missing log file → status warning, no crash

### Argument truncation in table

Command args can be long (e.g., `slices/grove-grouped-completions`).
Truncate to ~20 chars with `…` suffix in the table display. The full
args are visible in the log content itself.

## Acceptance Criteria

- `graft logs` lists recent runs with command, time, duration, status
- `graft logs --command <name>` filters by command name prefix
- `graft logs --limit N` controls result count
- `graft logs show <N>` prints full log content to stdout
- `graft logs --json` outputs structured JSON
- `graft logs` outside a graft project prints a clear error
- Grove `:logs` pushes a "Recent Runs" table into the scroll buffer
- Grove `:logs <filter>` filters by command name prefix
- Selecting a row in the logs table opens the full log as a text block
- Log content capped at 10,000 lines with truncation notice
- Status uses colored indicators (green ok, red failed, yellow running)
- Duration format matches existing `format_elapsed` ("42s", "2m 34s")
- `cargo test && cargo clippy -- -D warnings && cargo fmt --check`
  passes

## Steps

- [x] **Extract `format_duration` to graft-common and add
  `RunMeta::duration_display()`**
  - **Delivers** — shared duration formatting for CLI and TUI
  - **Done when** — `format_elapsed` logic moved from
    `scroll_buffer.rs` to `graft-common` as public
    `format_duration(Duration) -> String`; `scroll_buffer.rs`
    delegates to it; `RunMeta::duration_display()` returns
    `Option<String>` (None when `end_time` absent); unit tests
    for `duration_display()` in `runs.rs`; existing
    `format_elapsed` tests still pass
  - **Files** — `crates/graft-common/src/lib.rs`,
    `crates/graft-common/src/runs.rs`,
    `crates/grove-cli/src/tui/scroll_buffer.rs`

- [x] **Add `graft logs` list and show subcommands**
  - **Delivers** — run history accessible from the command line
  - **Done when** — `Logs` variant added to `Commands` enum with
    clap subcommands; `graft logs` lists recent runs with command,
    time ago, duration, and status columns; `--limit`, `--command`,
    and `--json` flags work; `graft logs show <N>` prints log
    content to stdout with clear errors for out-of-range or missing;
    workspace/repo derived via `find_graft_yaml()` +
    `base_dir.file_name()` (same as `graft run`); meaningful error
    when run outside a graft project
  - **Files** — `crates/graft-cli/src/main.rs`

- [x] **Add `:logs` command and `LogView` handler to grove TUI**
  - **Delivers** — run history browsable and viewable in the TUI
  - **Done when** — `CliCommand::Logs(Option<String>)` and
    `CliCommand::LogView(String)` variants added; `parse_command`
    handles `"logs" | "l"` → `Logs`; `PALETTE_COMMANDS` entry added
    with description; `execute_cli_command` dispatches both;
    `cmd_logs` pushes a "Recent Runs" actionable table with command,
    args (truncated to 20 chars), time ago, duration, colored status
    columns; `:logs <filter>` filters by command name prefix;
    `cmd_log_view` reads log content via `read_run_log()`, caps at
    10,000 lines with truncation notice, pushes as
    `ContentBlock::Text` with descriptive title, auto-scrolls;
    missing log → status warning; help text updated
  - **Files** — `crates/grove-cli/src/tui/command_line.rs`,
    `crates/grove-cli/src/tui/transcript.rs`
