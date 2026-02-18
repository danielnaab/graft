---
status: working
purpose: "Implementation plan for TUI view stack and command line — task tracking for Ralph loop"
---

# TUI View Stack and Command Line Plan

Evolving the Grove TUI from a fixed two-pane layout (40% repo list / 60% tabbed detail) to
full-width views navigated via a stack, plus a vim-style `:` command line for issuing commands.

## How to use this plan

Each task is a self-contained unit of work. Read the listed specs and code, implement the
capability, verify, and mark complete. Tasks are ordered for incremental migration — each
task leaves the TUI functional and tests passing.

Key constraints:
- **Bridge pattern**: When replacing a concept, add the new alongside the old first, then
  remove the old in a later task.
- **Test continuity**: Existing tests pass unchanged in early tasks. Update tests only when
  a task explicitly removes the concept they test.
- **Specs**: `docs/specifications/grove/tui-behavior.md` and `command-execution.md` describe
  current behavior. Design notes describe the target. Update specs in Task 10.

## Design references

- `notes/2026-02-18-grove-command-prompt-exploration.md` — view stack + command line design
- `notes/2026-02-18-grove-agentic-orchestration.md` — dispatch board metaphor, future session primitives

## Resolved design conflicts

(Record conflicts you discover and how you resolved them here)

## Spec gaps discovered

(Record spec gaps and reasonable choices made here)

---

## Phase 1: View Stack Architecture (Tasks 1-6)

### Task 1: Introduce View enum and ViewStack alongside ActivePane
- [x] Add `View` enum and view stack to `App`, bridging with existing `ActivePane`
- **Code**: `crates/grove-cli/src/tui/mod.rs`, `crates/grove-cli/src/tui/app.rs`
- **Specs**: `docs/specifications/grove/tui-behavior.md`
- **Design**: `notes/2026-02-18-grove-command-prompt-exploration.md` (Navigation: Hybrid Stack with Shortcuts)
- **Acceptance**:
  - `View` enum exists with variants: `Dashboard`, `RepoDetail(usize)`, `CommandOutput`, `Help`
  - `App` has `view_stack: Vec<View>` field, initialized with `[View::Dashboard]`
  - Helper methods: `current_view()`, `push_view()`, `pop_view()`, `reset_to_dashboard()`, `reset_to_view()`
  - Bridge method `active_pane_from_view()` returns matching `ActivePane` for current view
  - `active_pane` field still exists and is kept in sync via bridge
  - ALL existing tests pass unchanged — this is purely additive
  - `cargo fmt --check && cargo clippy -- -D warnings && cargo test` passes

### Task 2: Wire key dispatch through ViewStack (Dashboard + RepoDetail)
- [x] Change `handle_key()` to dispatch on `current_view()` instead of `active_pane`
- **Code**: `crates/grove-cli/src/tui/app.rs`, `crates/grove-cli/src/tui/hint_bar.rs`, `crates/grove-cli/src/tui/mod.rs`, `crates/grove-cli/src/tui/tests.rs`
- **Specs**: `docs/specifications/grove/tui-behavior.md` (Focus Management, Keybindings)
- **Acceptance**:
  - `handle_key()` dispatches on `current_view()` instead of `active_pane`
  - `handle_key_repo_list` renamed to `handle_key_dashboard`; `Enter` calls `push_view(RepoDetail(idx))`
  - New `handle_key_repo_detail` merges current detail key handling; `q` calls `pop_view()`
  - Hint bar dispatches on `current_view()` for context-sensitive hints
  - `active_pane` bridge still maintained for rendering (removed in Task 4)
  - All existing tests updated to check `current_view()` alongside `active_pane`
  - `cargo fmt --check && cargo clippy -- -D warnings && cargo test` passes

### Task 3: Wire CommandOutput and ArgumentInput through ViewStack
- [x] Route command execution and argument input through the view stack
- **Code**: `crates/grove-cli/src/tui/app.rs`, `crates/grove-cli/src/tui/overlays.rs`, `crates/grove-cli/src/tui/tab_commands.rs`
- **Specs**: `docs/specifications/grove/command-execution.md`
- **Acceptance**:
  - Command execution pushes `View::CommandOutput`; `q` pops back to previous view
  - ArgumentInput stays as overlay (intercepted before view dispatch, not a stack view)
  - Stop confirmation respected: prevents `q` and `Escape` from leaving CommandOutput while command runs
  - Tests cover: push CommandOutput, pop back, argument input overlay intercept, stop confirmation gate
  - `cargo fmt --check && cargo clippy -- -D warnings && cargo test` passes

### Task 4: Full-width Dashboard view
- [ ] Rewrite rendering to dispatch on `current_view()`, remove the 40/60 split layout
- **Code**: `crates/grove-cli/src/tui/render.rs`, `crates/grove-cli/src/tui/repo_list.rs`, `crates/grove-cli/src/tui/mod.rs`
- **Specs**: `docs/specifications/grove/tui-behavior.md` (Repo List Display)
- **Design**: `notes/2026-02-18-grove-command-prompt-exploration.md` (What Full Width Gives the Dashboard)
- **Acceptance**:
  - `render()` dispatches on `current_view()` — each view gets full terminal width
  - Dashboard renders repo list full-width (no 40/60 constraint)
  - `ActivePane` enum removed entirely; bridge method removed
  - Help view renders as a full-width view, not an overlay
  - CommandOutput renders as a full-width view
  - All tests updated to remove `active_pane` references; use `current_view()` exclusively
  - `cargo fmt --check && cargo clippy -- -D warnings && cargo test` passes

### Task 5: Full-width RepoDetail view
- [ ] Replace tabbed detail pane with single scrollable full-width view
- **Code**: `crates/grove-cli/src/tui/tab_changes.rs`, `crates/grove-cli/src/tui/tab_state.rs`, `crates/grove-cli/src/tui/tab_commands.rs`, `crates/grove-cli/src/tui/tabs.rs`, `crates/grove-cli/src/tui/render.rs`
- **Specs**: `docs/specifications/grove/tui-behavior.md` (Detail Pane)
- **Design**: `notes/2026-02-18-grove-command-prompt-exploration.md` (What Full Width Gives the Repo View)
- **Acceptance**:
  - RepoDetail renders all sections vertically in a single scrollable view: header, changed files, recent commits, state queries, commands
  - `DetailTab` enum removed; tab switching keys (`1`/`2`/`3`) removed
  - Tab-specific rendering consolidated into unified repo detail rendering
  - `tabs.rs` module removed or emptied
  - All tab-related tests replaced with unified view tests
  - `cargo fmt --check && cargo clippy -- -D warnings && cargo test` passes

### Task 6: Escape-goes-home and stack discipline
- [ ] Implement full navigation semantics: `Escape` resets to dashboard, `q` pops one level
- **Code**: `crates/grove-cli/src/tui/app.rs`
- **Specs**: `docs/specifications/grove/tui-behavior.md` (Focus Management)
- **Design**: `notes/2026-02-18-grove-command-prompt-exploration.md` (Navigation: Hybrid Stack with Shortcuts)
- **Acceptance**:
  - `Escape` from any view calls `reset_to_dashboard()` (go home)
  - `q` from any view calls `pop_view()` (go back one level)
  - `q` from Dashboard triggers quit (same as current behavior)
  - Running command stop confirmation gates both `q` and `Escape` in CommandOutput view
  - Tests cover: deep stack Escape, single pop, escape from each view type, stop confirmation gate
  - `cargo fmt --check && cargo clippy -- -D warnings && cargo test` passes

## Phase 2: Command Line (Tasks 7-9)

### Task 7: Command line input infrastructure (`:` key)
- [ ] Add command line state and rendering triggered by `:` key
- **Code**: `crates/grove-cli/src/tui/mod.rs`, `crates/grove-cli/src/tui/app.rs`, `crates/grove-cli/src/tui/render.rs`, `crates/grove-cli/src/tui/hint_bar.rs`
- **Design**: `notes/2026-02-18-grove-command-prompt-exploration.md` (The `:` Command Line)
- **Acceptance**:
  - `CommandLineState { buffer: String, cursor_pos: usize }` added to mod.rs
  - `App` has `command_line: Option<CommandLineState>` field
  - `:` from any view activates command line (sets `command_line = Some(...)`)
  - Command line renders at bottom of screen, replacing hint bar when active
  - Char input appends to buffer; backspace deletes; left/right moves cursor
  - `Escape` cancels command line (sets `command_line = None`)
  - `Enter` with non-empty buffer submits (parsing in Task 8)
  - Content area stays visible above command line
  - Tests cover: activation, input, cursor movement, cancel, submit
  - `cargo fmt --check && cargo clippy -- -D warnings && cargo test` passes

### Task 8: Command execution from command line
- [ ] Parse and execute commands entered via `:` command line
- **Code**: `crates/grove-cli/src/tui/app.rs`, `crates/grove-cli/src/tui/mod.rs`, new `crates/grove-cli/src/tui/command_line.rs`
- **Design**: `notes/2026-02-18-grove-command-prompt-exploration.md` (Mapping Existing and New Features to Commands)
- **Acceptance**:
  - New `command_line.rs` module with `CliCommand` enum: `Help`, `Quit`, `Refresh`, `Repo(String)`, `Run(String, Vec<String>)`, `State`, `Unknown(String)`
  - `parse_command(input: &str) -> CliCommand` function with unit tests
  - `:help` pushes Help view; `:quit` sets `should_quit`; `:refresh` triggers refresh
  - `:repo <name>` calls `reset_to_view(RepoDetail(idx))` matching by name or index
  - `:run <cmd> [args]` executes the command (reusing existing command execution logic)
  - `:state` pushes state queries view (if applicable, or shows status message)
  - Unknown commands show error in status bar
  - Tests cover: each command variant parsing and execution
  - `cargo fmt --check && cargo clippy -- -D warnings && cargo test` passes

### Task 9: Command palette (`:` with empty input)
- [ ] Show filterable command palette when command line is empty
- **Code**: `crates/grove-cli/src/tui/command_line.rs`, `crates/grove-cli/src/tui/render.rs`, `crates/grove-cli/src/tui/app.rs`
- **Design**: `notes/2026-02-18-grove-command-prompt-exploration.md` (Discoverability)
- **Acceptance**:
  - Empty command line buffer shows a command palette popup above the command line
  - Palette lists all available commands with short descriptions
  - Typing in command line filters palette entries (case-insensitive substring match)
  - `j`/`k` (or up/down arrows) navigate palette entries
  - `Enter` on selected palette entry fills command line with that command
  - `Escape` dismisses palette and command line
  - Tests cover: palette display, filtering, navigation, selection
  - `cargo fmt --check && cargo clippy -- -D warnings && cargo test` passes

## Phase 3: Spec Updates (Task 10)

### Task 10: Update TUI behavior and command execution specs
- [ ] Update specs to reflect view stack and command line architecture
- **Files**: `docs/specifications/grove/tui-behavior.md`, `docs/specifications/grove/command-execution.md`
- **Acceptance**:
  - `tui-behavior.md`: Split-Pane Layout sections replaced with View Stack sections; Dashboard/RepoDetail/Help described as full-width views; Navigation section describes stack semantics (push/pop/reset); Command Line section added; Keybindings table updated with `:`, view-specific keys
  - `command-execution.md`: `:run` documented as alternative to `x` key picker; Output Pane renamed to Command Output View; updated keybindings
  - Specs are internally consistent (no references to old pane/tab concepts)
  - No broken spec cross-references
