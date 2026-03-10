# Grouped Completions — Post-Review Fixes

## Fix 1: Replace `__sentinel__` with proper initial state

**File:** `crates/grove-cli/src/tui/prompt.rs` — `build_grouped_items`

Change `current_group` from `Option<&str>` initialized to `Some("__sentinel__")` to a two-variable approach: `current_group: Option<&str> = None` plus `is_first = true`. On each iteration, emit a header when `is_first || group_label != current_group`. Set `is_first = false` after first iteration. This eliminates the magic string entirely.

## Fix 2: Use explicit loop in `build_flat_items`

**File:** `crates/grove-cli/src/tui/prompt.rs` — `build_flat_items`

Replace the `.map()` closure that mutates `max_w` with an explicit `for` loop matching the style of `build_grouped_items`. Cleaner and consistent.

## Fix 3: Apply `max(20)` minimum width in `build_grouped_items`

**File:** `crates/grove-cli/src/tui/prompt.rs` — `build_grouped_items`

Change the return to `(items, display_row, max_w.max(20))` to match `build_flat_items`.

## Fix 4: Sort filtered completions by group

**File:** `crates/grove-cli/src/tui/prompt.rs` — `filter_scion_completions`

After filtering by prefix, sort the result by `group` (None last) so that headers remain contiguous in the rendered popup. Use a stable sort to preserve within-group order.

## Fix 5: Label orphaned scions distinctly

**File:** `crates/grove-cli/src/tui/transcript.rs` — `scion_completions`

When appending existing scions not in the source list, set `group: Some("existing".to_string())` instead of `None`, so they get a meaningful section header rather than "other".

## Fix 6: Add config parsing test for `group_by`

**File:** `crates/graft-common/src/config.rs` (test section)

Add `parse_state_query_entity_group_by` test that parses YAML with `group_by: status` and asserts `entity.group_by == Some("status")`.

## Verification

```bash
cargo fmt --check
cargo clippy -- -D warnings
cargo test
```
