---
status: ready
created: 2026-02-24
---

# Show run-state entries in Grove repo detail view

## Story

Commands that declare `writes:` persist state to `.graft/run-state/` as JSON
files. This state is queryable via `graft state query <name>` but invisible in
Grove — you can't see what state exists, what it contains, or which commands
produce and consume it. This slice adds a "Run State" section to the repo detail
view showing all run-state entries with their contents and producer/consumer
relationships, closing the observability gap.

## Approach

Follow the exact section pattern established by State Queries and Recent Runs:
header line (not selectable), items with cursor support, expand/collapse for
JSON contents. The section sits between Recent Runs and Commands — after
execution history, before available actions.

Each run-state entry shows:
- The state name (e.g., `session`)
- A summary of the JSON content (reusing `format_state_summary`)
- Which command produces it (from `writes:` declarations in `available_commands`)
- Which commands require it (from `reads:` declarations)

Expand on Enter to show full JSON (reusing `format_state_expanded_lines`).

Loading: enumerate `.graft/run-state/*.json` files in the selected repo
directory during `ensure_state_loaded_if_needed`. No new crate dependencies —
`graft_engine::state::get_run_state_entry` already reads individual entries;
add a listing function or enumerate the directory inline.

## Acceptance Criteria

- The repo detail view shows a "Run State" section listing all `.graft/run-state/*.json` entries
- Each entry displays state name, content summary, and producer command name
- Enter toggles expand/collapse showing full JSON content
- Empty state (no run-state directory or no files) shows "No run state" message
- Producer/consumer labels are derived from `available_commands` writes/reads fields
- Cursor navigation works across the new section (j/k, up/down)
- `cargo fmt --check && cargo clippy -- -D warnings && cargo test` pass after each step

## Steps

- [ ] **Add RunState variant and data fields to App**
  - **Delivers** — structural foundation for the new section
  - **Done when** — `DetailItem::RunState(usize)` variant exists; `App` has
    `run_state_entries: Vec<(String, serde_json::Value)>` and
    `expanded_run_state: HashSet<usize>` fields; both initialized in
    `App::new()`; `rebuild_detail_items()` pushes `RunState` items between
    `Run` and `Command` items; compiles with no warnings
  - **Files** — `crates/grove-cli/src/tui/mod.rs`, `crates/grove-cli/src/tui/app.rs`,
    `crates/grove-cli/src/tui/repo_detail.rs`

- [ ] **Load run-state entries for the selected repo**
  - **Delivers** — run-state data is populated when a repo is viewed
  - **Done when** — `load_run_state_entries()` reads `.graft/run-state/` in the
    selected repo, collects `(name, value)` pairs sorted alphabetically, and is
    called from `ensure_state_loaded_if_needed()`; entries are cleared on repo
    switch (alongside `state_loaded = false`); handles missing directory
    gracefully (empty vec)
  - **Files** — `crates/grove-cli/src/tui/repo_detail.rs`

- [ ] **Render the Run State section**
  - **Delivers** — run-state entries visible in the repo detail view
  - **Done when** — `append_run_state_section_mapped()` renders a "Run State"
    header (blue, bold) and one row per entry showing
    `"  ▸ {name:<12}  {summary:<40} (← {producer})"` where producer is looked
    up from `available_commands`; entries with no data show gray placeholder;
    empty state shows "No run state" in gray; section appears between Recent
    Runs and Commands in `build_line_mapping()`
  - **Files** — `crates/grove-cli/src/tui/repo_detail.rs`

- [ ] **Handle Enter to expand/collapse run-state entries**
  - **Delivers** — full JSON inspection without leaving the view
  - **Done when** — Enter on a `RunState` item toggles `expanded_run_state`;
    expanded entries show JSON lines via `format_state_expanded_lines` (same
    helper used by State Queries); expanded lines have `None` item index (not
    cursor-selectable); consumers (commands with matching `reads:`) shown in
    expanded view as `"  reads: {cmd1}, {cmd2}"`
  - **Files** — `crates/grove-cli/src/tui/repo_detail.rs`
