---
status: working
purpose: "Append-only progress log for TUI view stack and command line Ralph loop"
---

# Progress Log

## Consolidated Patterns

### Pre-existing clippy issues (now fixed)
The initial codebase had pre-existing clippy/fmt issues in `state/` and `main.rs` that blocked `cargo clippy -- -D warnings`. These were fixed as part of Task 1 so future tasks can verify cleanly:
- `state/query.rs`: `StateMetadata` re-export needed `#[allow(unused_imports)]` (used only in tests)
- `state/mod.rs`: `read_all_cached_for_query` and `read_cached_state` re-exports needed `#[allow(unused_imports)]`
- `state/cache.rs`: `get_cache_path_from_hash` and `read_cached_state` needed `#[allow(dead_code)]`
- `main.rs`: `eprintln!("Loading {} repositories...", repo_count)` → `eprintln!("Loading {repo_count} repositories...")`

### Bridge pattern for additive-only tasks
When adding new fields/methods that won't be used until a later task, use `#[allow(dead_code)]` on each item rather than on the whole impl block. This makes it easy to remove the annotation in the task that starts using each item.

### ArgumentInput is NOT a view
`ArgumentInput` is an overlay over the current view, not a stack view. The `sync_active_pane()` bridge correctly skips syncing when `active_pane == ArgumentInput` to preserve the overlay state.

### Clearing ArgumentInput pane before push_view (overlay Enter pattern)
When argument input confirms (Enter), the `active_pane` is still `ArgumentInput` when we call `push_view(CommandOutput)`. Since `sync_active_pane` skips syncing when `active_pane == ArgumentInput`, we must first clear it: `self.active_pane = self.active_pane_from_view()` before calling `push_view`. Same pattern when Esc cancels: `self.active_pane = self.active_pane_from_view()` instead of `pop_view()`.

### Tab-specific key handlers may need pop_view() updates
When dispatching through `handle_key_repo_detail`, the tab-specific handlers (`handle_key_changes_tab`, etc.) can also call `pop_view()`. For example, `handle_key_changes_tab` had a direct `self.active_pane = ActivePane::RepoList` for Enter — this must become `self.pop_view()`.

---

### Iteration — Task 1: Introduce View enum and ViewStack alongside ActivePane
**Status**: completed
**Files changed**: `crates/grove-cli/src/tui/mod.rs`, `crates/grove-cli/src/tui/app.rs`, `crates/grove-cli/src/state/query.rs`, `crates/grove-cli/src/state/mod.rs`, `crates/grove-cli/src/state/cache.rs`, `crates/grove-cli/src/main.rs`
**What was done**: Added `View` enum and `view_stack: Vec<View>` field to `App`. Added helper methods `current_view()`, `push_view()`, `pop_view()`, `reset_to_dashboard()`, `reset_to_view()`, `active_pane_from_view()`, and `sync_active_pane()`. Used `#[allow(dead_code)]` on all new additions since Task 2 will start using them. Also fixed pre-existing clippy/fmt issues in state/ and main.rs.
**Critique findings**: Implementation is clean and correctly handles the `ArgumentInput` overlay edge case (doesn't sync active_pane when overlay is showing). All acceptance criteria met.
**Improvements made**: none needed
**Learnings for future iterations**: Task 2 should remove `#[allow(dead_code)]` attributes as it starts using each helper. The `sync_active_pane()` pattern is the right way to keep bridge in sync without breaking overlays.

---

### Iteration — Task 2: Wire key dispatch through ViewStack (Dashboard + RepoDetail)
**Status**: completed
**Files changed**: `crates/grove-cli/src/tui/app.rs`, `crates/grove-cli/src/tui/mod.rs`, `crates/grove-cli/src/tui/hint_bar.rs`, `crates/grove-cli/src/tui/overlays.rs`, `crates/grove-cli/src/tui/tab_changes.rs`, `crates/grove-cli/src/tui/tests.rs`
**What was done**: Changed `handle_key()` to dispatch on `current_view()` with ArgumentInput intercepted before dispatch. Renamed `handle_key_repo_list` to `handle_key_dashboard` (Enter/Tab use `push_view(RepoDetail(idx))`); added `handle_key_repo_detail` (q/Esc/Tab use `pop_view()`). Updated hint bar to dispatch on `current_view()` with ArgumentInput overlay check. Updated overlays.rs: Enter in argument input clears active_pane then does `push_view(CommandOutput)`; Esc restores from view stack. Command output close uses `pop_view()`. Fixed `tab_changes.rs` Enter to use `pop_view()`. Removed all `#[allow(dead_code)]` annotations from Task 1. All tests updated to use `push_view()` for setup and assert `current_view()` alongside `active_pane`.
**Critique findings**: Implementation is clean and handles the overlay edge case correctly. The `active_pane_from_view()` call before `push_view` in the ArgumentInput Enter case is necessary due to `sync_active_pane()`'s overlay guard. `tab_changes.rs` needed updating too (discovered during test run — Enter key needed `pop_view()`).
**Improvements made**: Fixed `tab_changes.rs` Enter to use `pop_view()` after test revealed it still set `active_pane` directly.
**Learnings for future iterations**: Search all tab-specific key handlers for direct `active_pane` assignments when wiring view stack — they all need `pop_view()`. The overlay guard in `sync_active_pane()` means ArgumentInput dismissal must explicitly restore `active_pane` from the view stack rather than relying on `pop_view()`.

---

