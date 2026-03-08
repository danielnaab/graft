---
status: done
created: 2026-03-06
---

# Add block gutter markers and stronger focus highlight

## Story

Block boundaries in grove's transcript are marked only by a single blank line,
making it hard to tell where one block ends and the next begins. When a block is
focused via Tab, the highlight is `Rgb(30, 30, 45)` — nearly invisible against
a black terminal background. Users can't tell what's focused or where blocks
start and end, especially with multi-line text blocks containing similar content.

After this slice, every block gets a dimmed left gutter marker (`│`) showing
its boundary. Focused blocks get a bright gutter (`▐`) plus a stronger
background tint. Both problems — block separation and focus visibility — are
solved by one coherent visual system.

## Approach

Modify the line rendering in `render_visible()` at `scroll_buffer.rs:639-676`.
Currently, lines are rendered as-is with an optional focus background tint.
The new approach prepends a gutter character to every block line:

### Gutter characters

- **Unfocused block lines**: `│ ` (dimmed dark gray, `Color::Rgb(60, 60, 60)`)
- **Focused block lines**: `▐ ` (bright cyan, `Color::Cyan`)
- **Separator lines** (between blocks): no gutter (blank line, unchanged)

### Focus background

- **Current**: `Rgb(30, 30, 45)` — too subtle
- **New**: `Rgb(40, 40, 70)` — noticeably different on dark terminals

### Rendering change

In the `render_visible()` focus highlight loop (`scroll_buffer.rs:663-676`),
for each line associated with a block (`block_idx` is `Some`):

1. Prepend a gutter `Span` before the existing line content
2. If the block is focused: gutter is `▐ ` in Cyan, line gets `bg(Rgb(40, 40, 70))`
3. If unfocused: gutter is `│ ` in `Rgb(60, 60, 60)`, no background change

Lines not associated with a block (separators, `block_idx` is `None`) are
unchanged — they remain blank lines providing vertical spacing.

### Width accounting

The gutter consumes 2 columns (`▐ ` or `│ `). The gutter is prepended in
`render_visible()` AFTER `render_lines_at()` has already produced content
lines. Therefore, the `width` parameter passed to `render_lines_at()` at
line 647 must be reduced by 2 so content is rendered to fit within the
remaining space. This affects table column calculations (via
`compute_col_widths`) and text line wrapping automatically, since they
already use the `width` parameter.

Collapsed blocks still render a title line — that line also gets a gutter.

### Visual example

```
  │ State Queries
  │ Query     Summary        Age    Cached
  │ verify    pass           2m     yes
  │ slices    (not cached)   -      no

  ▐ Started scion 'test'
  ▐   worktree: .worktrees/test
  ▐   branch: feature/test

  │ ✗ scion start failed: dependency 'software-factory' not found
```

## Acceptance Criteria

- Every block line has a left gutter marker
- Unfocused blocks show `│ ` in dark gray
- Focused blocks show `▐ ` in cyan with `Rgb(40, 40, 70)` background
- Separator lines between blocks remain plain blank lines (no gutter)
- Table columns and text wrapping account for the 2-column gutter width
- Running blocks (with spinner) display correctly with gutter
- Collapsed blocks display correctly with gutter
- `cargo test && cargo clippy -- -D warnings && cargo fmt --check` passes

## Steps

- [x] **Add gutter markers to block line rendering**
  - **Delivers** — visible block boundaries and strong focus highlight
  - **Done when** — `render_visible()` in `scroll_buffer.rs` prepends a
    gutter `Span` to each block line: `│ ` in `Rgb(60, 60, 60)` for
    unfocused, `▐ ` in `Color::Cyan` for focused; focused lines also get
    `bg(Color::Rgb(40, 40, 70))`; separator lines (block_idx `None`)
    unchanged; old `Rgb(30, 30, 45)` background removed
  - **Files** — `crates/grove-cli/src/tui/scroll_buffer.rs`

- [x] **Adjust content width for gutter**
  - **Delivers** — tables and text fit correctly with gutter present
  - **Done when** — the `width` passed to `render_lines_at()` and used for
    table column calculation is reduced by 2 to account for the gutter;
    `total_lines()` is unaffected (it counts logical lines, not columns);
    existing table and text blocks render without clipping
  - **Files** — `crates/grove-cli/src/tui/scroll_buffer.rs`
