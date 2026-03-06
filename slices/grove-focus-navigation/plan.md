---
status: draft
created: 2026-03-06
---

# Improve block focus, navigation, and scroll tracking

## Story

Block navigation in grove has several friction points. After running a command,
reaching the output block requires pressing Tab repeatedly from block 0 — the
oldest block in the transcript. Tab doesn't wrap, so reaching the latest block
requires as many Tab presses as there are blocks. When focus does change via
Tab/Shift+Tab, the viewport doesn't scroll to reveal the focused block — the
user can focus something entirely off-screen with no visual feedback. And when a
block IS focused, Up/Down still line-scroll the viewport instead of moving
between blocks, forcing the user to use Tab exclusively for block navigation.

After this slice, Tab starts at the most recent block, wraps circularly, and
auto-scrolls to reveal the focused block. Up/Down navigate between blocks when
one is focused (line-scrolling when unfocused). Esc unfocuses the current block.

## Approach

Four changes to `scroll_buffer.rs` and one to `transcript.rs`:

1. **Tab from unfocused → last block**: Change `focus_next()` so that when
   `focused_block` is `None`, it sets focus to the last block (not block 0).
   This matches the existing `focus_prev()` behavior from unfocused and puts
   the user at the most recent content.

2. **Tab/Shift+Tab wrapping**: `focus_next()` wraps from last block to first.
   `focus_prev()` wraps from first block to last. Circular navigation so any
   block is reachable from any position.

3. **Auto-scroll on focus change**: After `focus_next()` and `focus_prev()`
   change `focused_block`, compute the focused block's starting line in the
   scroll buffer and adjust `scroll_offset` so the block is visible in the
   viewport. New helper `block_start_line(index)` computes the cumulative
   line offset by summing `line_count()` of preceding blocks plus separator
   lines (matching the logic in `total_lines()`).

4. **Up/Down as block navigation when focused**: In `transcript.rs` key
   handler, when `focused_block` is `Some`, Up/Down dispatch to
   `focus_prev()`/`focus_next()` instead of `scroll_up()/scroll_down()`.
   When `focused_block` is `None`, Up/Down line-scroll as today.

5. **Esc to unfocus**: Add Esc key handler in `transcript.rs` that sets
   `focused_block = None` when no picker or prompt is active. This gives
   users a clear way to exit focus mode and return to line-scrolling.

### Interaction summary

| State | Tab | Shift+Tab | Up | Down | Esc |
|-------|-----|-----------|----|----- |-----|
| Unfocused | Focus last block | Focus last block | Line scroll | Line scroll | — |
| Focused | Next block (wrap) | Prev block (wrap) | Prev block | Next block | Unfocus |

## Acceptance Criteria

- Tab from unfocused jumps to the last block (most recent)
- Shift+Tab from unfocused jumps to the last block (unchanged)
- Tab from the last block wraps to the first block
- Shift+Tab from the first block wraps to the last block
- Focus changes auto-scroll the viewport to reveal the focused block
- When a block is focused, Up/Down move focus between blocks
- When no block is focused, Up/Down line-scroll (unchanged)
- Esc unfocuses the current block (when no picker or prompt is active)
- Empty scroll buffer: Tab/Shift+Tab/Up/Down are no-ops
- Single block: Tab and Shift+Tab keep focus on that block (wraps to self)
- `cargo test && cargo clippy -- -D warnings && cargo fmt --check` passes

## Steps

- [ ] **Change focus_next/focus_prev to wrap and start at last block**
  - **Delivers** — circular tab navigation starting from most recent block
  - **Done when** — both methods early-return if `blocks.is_empty()`;
    `focus_next()` sets `focused_block` to `blocks.len() - 1` when currently
    `None`, wraps from last to 0 when at end; `focus_prev()` wraps from 0 to
    `blocks.len() - 1` when at start; single-block case: both wrap to self
    (index 0); existing behavior for mid-list focus is unchanged
  - **Files** — `crates/grove-cli/src/tui/scroll_buffer.rs`

- [ ] **Add auto-scroll on focus change**
  - **Delivers** — focused block is always visible in the viewport
  - **Done when** — new `block_start_line(index: usize) -> usize` method
    computes the line offset of a block by summing preceding blocks'
    `line_count()` plus separator lines; `focus_next()` and `focus_prev()`
    call `scroll_to_block()` after changing focus, which adjusts
    `scroll_offset` so the focused block's first line is within the viewport
    (scrolls up if above viewport, scrolls down if below, no change if
    already visible)
  - **Files** — `crates/grove-cli/src/tui/scroll_buffer.rs`

- [ ] **Add Up/Down block navigation when focused and Esc to unfocus**
  - **Delivers** — directional block navigation and clean exit from focus mode
  - **Done when** — in `handle_key_normal()`, when `self.scroll.focused_block`
    is `Some`: Up/k dispatches `focus_prev()`, Down/j dispatches
    `focus_next()`; when `None`: Up/Down line-scroll (unchanged); Esc when
    no picker or prompt is active sets `self.scroll.focused_block = None`
    (works regardless of whether a Running block is active — Esc unfocuses
    the visual selection, it doesn't cancel execution)
  - **Files** — `crates/grove-cli/src/tui/transcript.rs`
