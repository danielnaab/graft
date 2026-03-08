---
status: done
created: 2026-03-06
depends_on:
  - state-query-input-keyed-cache
---

# Execute state queries from grove

## Story

Grove's `:state` command discovers state queries from `graft.yaml` and displays
them in a table, but can only show cached results. Queries that haven't been
executed externally show "(not cached)" with no way to populate them. The `r`
key refreshes repository statuses but ignores state queries, contradicting the
spec (`tui-behavior.md:440-448`). This makes the state panel non-functional for
new users who haven't independently run `graft state query` from the CLI.

After this slice, pressing Enter on an uncached state query row executes it and
shows live output. Pressing `r` bulk-refreshes all state queries. In both cases,
the state table auto-refreshes to reflect updated cache status.

## Approach

Three capabilities, all reusing grove's existing async execution pattern
(`spawn_command` → channel → Running block → finalize):

1. **Individual execution** — Enter on a state table row dispatches differently
   based on cache status:
   - Uncached → `CliCommand::StateRun(name)` → spawns `graft state query <name>`
     as a Running block
   - Cached → `CliCommand::State(Some(name))` → shows detail view (unchanged)

2. **Bulk refresh** — `r` key triggers repo refresh (existing) then spawns a
   single Running block that executes all discovered state queries sequentially.
   Each query's name is emitted as a progress line ("▶ slices...", "✓ slices",
   "▶ verify...", etc.). Guarded by `CommandState::Running` — if a command is
   already in flight, bulk refresh is skipped with a status warning.

3. **Auto-refresh** — After any state query execution completes (individual or
   bulk), grove automatically appends a fresh `:state` table to the transcript.
   Implemented via a `pending_state_refresh` flag on `ExecutionState`, checked
   in `handle_command_events()` after block finalization.

### Execution mechanism

State queries are executed by shelling out to `graft state query <name>` (not
by calling engine functions directly). This matches the `:run` pattern, avoids
wiring engine internals into the TUI, and ensures cache writes happen through
the same path as CLI usage.

For bulk refresh, a single spawned thread iterates all query names, running
`graft state query <name>` for each sequentially via `ProcessHandle`. Output
from each query is bridged to the same channel, with per-query separator lines.

### Internal-only command variants

`StateRun(String)` and `StateRefresh` are internal `CliCommand` variants — they
are dispatched programmatically from table actions and the `r` key handler, not
typed by users. They do NOT need entries in `parse_command()` or
`PALETTE_COMMANDS`. No tab completion needed.

### Concurrency constraint

Grove supports one Running block at a time (single `command_event_rx`). Both
individual and bulk execution reuse this slot. If a `:run` command is already
executing, state query execution is rejected with a status message ("A command
is already running").

### Auto-refresh timing

The `pending_state_refresh` flag is checked in `handle_command_events()` AFTER
the Running block is finalized and the channel is closed (`should_close` is
true). At that point, `cmd_state(None)` is called to append a fresh table.
This avoids recursion — the state table is a static Text/Table block, not a
new Running block.

### Bulk refresh error handling

If a query fails during bulk refresh, `spawn_state_refresh_all` emits
"✗ <name>: <error>" for the failed query and continues to the next. The
Running block shows a mixed success/failure summary. The auto-refreshed
state table will reflect which queries now have cached results.

## Acceptance Criteria

- Enter on an uncached state query row executes that query and shows a Running block
- Enter on a cached state query row shows the detail view (unchanged behavior)
- Running block shows live output from `graft state query <name>`
- After individual execution completes, a fresh `:state` table is auto-appended
- `r` key refreshes repo statuses AND executes all state queries
- Bulk refresh runs queries sequentially in a single Running block
- Bulk refresh shows per-query progress ("▶ <name>...", "✓ <name>")
- After bulk refresh completes, a fresh `:state` table is auto-appended
- If a command is already running, state query execution shows "A command is already running" warning
- If no repo is selected, shows "No repository selected" warning
- If no state queries are defined, `r` refreshes repos only (no error)
- Failed query execution shows error in the Running block (same as `:run` failures)
- `cargo test && cargo clippy -- -D warnings && cargo fmt --check` passes

## Steps

- [x] **Add `StateRun` and `StateRefresh` command variants**
  - **Delivers** — command dispatch for state query execution
  - **Done when** — `CliCommand` enum in `command_line.rs` has `StateRun(String)`
    and `StateRefresh` variants; `execute_cli_command()` routes `StateRun` to
    `cmd_state_run()` and `StateRefresh` to `cmd_state_refresh_all()`;
    `ExecutionState` has `pending_state_refresh: bool` field (default false)
  - **Files** — `crates/grove-cli/src/tui/command_line.rs`,
    `crates/grove-cli/src/tui/transcript.rs`

- [x] **Add spawn helpers for state query execution**
  - **Delivers** — async subprocess execution for state queries
  - **Done when** — `spawn_state_query(name, repo_path, tx)` in `command_exec.rs`
    builds and runs `graft state query <name>` via `ProcessHandle`, bridges
    events to `CommandEvent` channel; `spawn_state_refresh_all(query_names,
    repo_path, tx)` iterates query names, runs each sequentially, emits
    "▶ <name>..." before and "✓ <name>" / "✗ <name>" after each query
  - **Files** — `crates/grove-cli/src/tui/command_exec.rs`

- [x] **Implement `cmd_state_run` for individual query execution**
  - **Delivers** — single query execution from the state table
  - **Done when** — `cmd_state_run(&name)` guards against concurrent execution,
    creates mpsc channel, spawns thread calling `spawn_state_query`, pushes
    Running block with command label `state query <name>`, sets
    `pending_state_refresh = true`
  - **Files** — `crates/grove-cli/src/tui/transcript.rs`

- [x] **Change Enter action on uncached state table rows**
  - **Delivers** — Enter on uncached rows triggers execution instead of showing empty detail
  - **Done when** — in `cmd_state()`, the action for each table row is
    `CliCommand::StateRun(name)` when `read_latest_cached()` returns `None`,
    and `CliCommand::State(Some(name))` when cached
  - **Files** — `crates/grove-cli/src/tui/transcript.rs`

- [x] **Auto-refresh state table after execution completes**
  - **Delivers** — state table stays current without manual re-run
  - **Done when** — `handle_command_events()` checks `pending_state_refresh`
    after finalizing a Running block; if true, invalidates
    `cached_state_queries`, calls `cmd_state(None)`, resets flag to false
  - **Files** — `crates/grove-cli/src/tui/transcript.rs`

- [x] **Implement bulk refresh via `r` key**
  - **Delivers** — one-key refresh of all state queries (spec alignment)
  - **Done when** — `r` key handler dispatches `StateRefresh` after setting
    `needs_refresh`; `cmd_state_refresh_all()` discovers all state queries,
    guards concurrent execution, creates channel, spawns thread calling
    `spawn_state_refresh_all`, pushes single Running block, sets
    `pending_state_refresh = true`; if no queries defined, no Running block
    is created (repo refresh only)
  - **Files** — `crates/grove-cli/src/tui/transcript.rs`
