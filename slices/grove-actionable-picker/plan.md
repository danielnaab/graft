---
status: draft
created: 2026-02-27
---

# Add actionable picker overlay to transcript TUI

## Story

The transcript TUI displays command output as read-only blocks. Commands like `:repos`,
`:catalog`, and `:state` print tables that the user can only collapse or scroll past.
To act on what they see, the user must mentally note the item, open the prompt, and type
a follow-up command (`:repo myrepo`, `:run deploy`, `:state some-query`). This is the
same friction as a CLI that prints a list and expects you to copy-paste into the next
command.

Modern transcript TUIs (OpenCode, Codex CLI, Helix) solve this with **picker overlays**:
a modal list rendered on top of the transcript with j/k navigation, type-to-filter, and
Enter to act. The transcript stays a clean read-only log; the overlay handles selection.

Grove already has an overlay — the command palette. This slice generalizes it into a
reusable picker component, then wires it into table blocks so that pressing Enter on a
focused table opens a picker seeded with that table's rows.

## Approach

### Picker overlay component

Extract a generic `Picker` from the existing palette rendering code in `prompt.rs`. A
picker has:

```rust
struct PickerState {
    items: Vec<PickerItem>,       // label + description + action payload
    filter: String,               // type-to-filter text
    selected: usize,              // highlighted row index
}

struct PickerItem {
    label: String,                // primary text (left column)
    description: String,          // secondary text (right column)
    action: CliCommand,           // command to execute on Enter
}
```

The picker renders identically to the current palette: a bordered `List` widget floating
above the prompt line, with cyan highlight on the selected row, a filter input at the
top, and Esc to dismiss. The existing palette becomes a special case — its items are
built from `PALETTE_COMMANDS` instead of from table rows.

### Actionable table blocks

`ContentBlock::Table` gains an optional `Vec<CliCommand>` parallel to its `rows` vec.
When present and the table is focused, Enter opens a picker overlay instead of toggling
collapse. Each picker item is built from the corresponding table row's display text and
the associated `CliCommand`. Collapse moves to a different key (`c` or `Space`).

### Wiring per command

| Command | Picker action per row |
|---------|----------------------|
| `:repos` | `CliCommand::Repo(name)` — switches to that repository |
| `:catalog` | `CliCommand::Run(name, vec![])` — runs the command (sequences included) |
| `:state` | `CliCommand::State(Some(name))` — expands that query's detail |

### Key rebinding

Enter currently toggles collapse on the focused block. With this change:
- **Enter on actionable table** → opens picker overlay
- **Enter on non-actionable block** → toggles collapse (unchanged)
- **`c`** → toggles collapse on any focused block (new binding, works everywhere)

This avoids a mode: Enter always means "activate" and `c` always means "collapse."

## Acceptance Criteria

- A `PickerState` component renders a filterable, navigable list overlay
- The picker supports: j/k and arrow keys to navigate, typing to filter, Enter to
  select, Esc to dismiss
- The picker renders in the same position and style as the current command palette
  (bordered List widget above the prompt line)
- The command palette uses the picker internally (no duplication of rendering/navigation)
- `ContentBlock::Table` supports an optional `actions: Vec<CliCommand>` field
- Pressing Enter on a focused actionable table opens a picker seeded with the table's
  rows and actions; the filter starts empty
- Pressing Enter on a focused non-actionable block (Text, Divider, or Table without
  actions) toggles collapse as before
- `c` toggles collapse on any focused block regardless of type
- `:repos` table is actionable: selecting a row executes `:repo <name>`
- `:catalog` table is actionable: selecting a row executes `:run <command-name>`
- `:state` table is actionable: selecting a row executes `:state <query-name>`
- Picker filter narrows rows by case-insensitive substring match on the label column
- When the picker has only one match remaining, Enter selects it
- Dismissing the picker with Esc returns focus to the transcript with no side effects
- The `:help` output documents the `c` keybinding for collapse and Enter for activate
- `cargo test && cargo clippy -- -D warnings && cargo fmt --check` passes
- Existing tests for palette, completions, and table rendering continue to pass

## Steps

- [ ] **Extract `PickerState` component and refactor palette to use it**
  - **Delivers** — a reusable picker overlay that the palette delegates to
  - **Done when** — `prompt.rs` contains a `PickerState` struct with `items`,
    `filter`, and `selected` fields; `PickerItem` has `label: String`,
    `description: String`, and `action: CliCommand`; `PickerState` exposes
    `render()` (renders the bordered List overlay in the same position/style as the
    current palette), `handle_key()` (j/k/arrows navigate, typing filters,
    Enter returns `Some(CliCommand)`, Esc returns dismiss signal), and
    `filtered_items()` (case-insensitive substring match on label); the existing
    command palette is converted to build a `Vec<PickerItem>` from
    `PALETTE_COMMANDS` and delegates to the picker for rendering and key handling;
    all existing palette tests pass unchanged; new unit tests: picker filter
    narrows items, picker navigation wraps around, picker Enter returns action,
    picker Esc returns None; `cargo test -p grove` passes
  - **Files** — `crates/grove-cli/src/tui/prompt.rs`,
    `crates/grove-cli/src/tui/tests.rs`

- [ ] **Add `actions` field to `ContentBlock::Table` and wire Enter/`c` keybindings**
  - **Delivers** — table blocks can carry per-row actions; Enter activates, `c`
    collapses
  - **Done when** — `ContentBlock::Table` gains `actions: Option<Vec<CliCommand>>`;
    `ScrollBuffer` exposes `focused_block_actions() -> Option<&[CliCommand]>` to
    check whether the focused block is actionable; in `transcript.rs`
    `handle_key()`, Enter on a focused block checks `focused_block_actions()`: if
    `Some`, builds a `Vec<PickerItem>` from the table's rows and actions and opens
    the picker; if `None`, toggles collapse as before; `c` key always calls
    `toggle_focused_collapse()`; `TranscriptApp` gains a `picker: Option<PickerState>`
    field; when the picker is `Some`, all key events route to `picker.handle_key()`;
    on picker Enter result, the returned `CliCommand` is executed via
    `execute_cli_command()`; on picker Esc, the picker is set to `None`; the picker
    renders on top of the scroll buffer (same layer as the palette); tests:
    Enter on actionable table opens picker, `c` toggles collapse, Enter on
    non-actionable block toggles collapse, picker selection executes command;
    `:help` output updated with `c` binding; `cargo test -p grove` passes
  - **Files** — `crates/grove-cli/src/tui/scroll_buffer.rs`,
    `crates/grove-cli/src/tui/transcript.rs`,
    `crates/grove-cli/src/tui/prompt.rs`,
    `crates/grove-cli/src/tui/tests.rs`

- [ ] **Wire `:repos`, `:catalog`, and `:state` to produce actionable tables**
  - **Delivers** — the three list commands produce tables that open pickers on Enter
  - **Done when** — `cmd_repos()` builds `actions: Some(vec![...])` with
    `CliCommand::Repo(basename)` for each row; `cmd_catalog()` builds actions with
    `CliCommand::Run(name, vec![])` for command rows and the appropriate sequence
    invocation for sequence rows; `cmd_state()` (the summary table, not the detail
    view) builds actions with `CliCommand::State(Some(query_name))` for each row;
    `push_table()` (or equivalent) accepts the optional actions vec and stores it in
    the `ContentBlock::Table`; integration tests: `:repos` Enter opens picker with
    repo names, selecting one switches repo context; `:catalog` Enter opens picker
    with command names, selecting one populates the prompt or executes; `:state`
    Enter opens picker with query names, selecting one pushes a detail block;
    `cargo test -p grove && cargo clippy -p grove -- -D warnings && cargo fmt -p grove --check` passes
  - **Files** — `crates/grove-cli/src/tui/transcript.rs`,
    `crates/grove-cli/src/tui/scroll_buffer.rs`,
    `crates/grove-cli/src/tui/tests.rs`
