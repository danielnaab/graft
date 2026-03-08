---
status: draft
created: 2026-03-06
---

# Smart column width allocation for table blocks

## Story

When the terminal is narrower than a table's natural width, grove proportionally
shrinks all columns equally. A 6-char "Name" column and a 60-char "Description"
column both lose the same percentage of width, so Name can shrink to 2-3
characters — making the entire table unreadable. This affects all Table blocks
(`:catalog`, `:state`, `:scion list`, etc.).

After this slice, short columns keep their full width while long columns absorb
the overflow. At extreme terminal widths, all columns gracefully degrade to a
minimum floor before falling back to proportional shrinking as a last resort.

## Approach

Replace the proportional scaling in `compute_col_widths()` with a two-phase
shrink-widest-first algorithm:

**Phase 1 — Shrink widest first, respecting minimums**:
1. Compute natural widths per column (max of header and cell content) — existing code, unchanged
2. Set a per-column minimum: `min(natural_width, MIN_COL_WIDTH)` where `MIN_COL_WIDTH = 4`
3. Calculate excess = total_natural - available
4. While excess > 0: find the widest column(s), shrink them toward the next-widest
   (or their minimum, whichever is larger), absorbing excess
5. Stop when all columns are at their minimum or excess is eliminated

**Phase 2 — Proportional fallback for extreme cases**:
If the sum of minimums still exceeds available width (very narrow terminal),
fall back to proportional scaling with a floor of 1. This is the existing
behavior, now only triggered as a last resort.

The `pad_or_truncate()` function already handles ellipsis truncation for content
wider than its allocated column — no changes needed there.

### Algorithm detail for tied widths

When multiple columns share the current maximum width, they are all shrunk
equally in the same iteration. Example: if widths are [30, 30, 10] and we
need to remove 10 of excess, both 30-width columns shrink to 25 each (5
removed per column = 10 total).

### Example: `:catalog` at various widths

Natural widths: Name=12, Description=55, Category=10 (total=77, separators=4)

| Terminal | Available | Algorithm result | Before (proportional) |
|----------|-----------|------------------|-----------------------|
| 100      | 96        | 12, 55, 10 (fits) | Same |
| 80       | 76        | 12, 54, 10 | 11, 53, 10 |
| 60       | 56        | 12, 34, 10 | 9, 40, 7 |
| 40       | 36        | 12, 14, 10 | 5, 25, 4 |

At 60-wide: Description (widest at 55) absorbs all 21 excess → shrinks to 34.
Name (12) and Category (10) are untouched because they're not the widest.

At 40-wide: Description absorbs first 19 excess (55→36, now tied with
available/cols). Then further shrinkage brings it to 14. Name and Category
stay at their natural widths since 12+14+10=36 fits.

## Acceptance Criteria

- Short columns (under `MIN_COL_WIDTH` natural width) never shrink
- Widest columns absorb overflow first, preserving short column readability
- At extreme widths, all columns degrade gracefully to minimum before proportional fallback
- Every column gets at least 1 character (existing guarantee preserved)
- All existing Table blocks (`:catalog`, `:state`, `:scion list`, `:repos`) render correctly
- Unit tests cover: fits-without-shrink, single-column-absorbs, multiple-widest,
  all-at-minimum, extreme-narrow, zero-width, single-column, empty-table
- `cargo test && cargo clippy -- -D warnings && cargo fmt --check` passes

## Steps

- [x] **Replace `compute_col_widths` proportional scaling with shrink-widest-first**
  - **Delivers** — readable tables at narrow terminal widths
  - **Done when** — `compute_col_widths()` in `scroll_buffer.rs` implements the
    two-phase algorithm: (1) shrink widest columns toward next-widest respecting
    `MIN_COL_WIDTH` floor of 4, (2) proportional fallback only when sum of
    minimums exceeds available; existing natural-width computation (header + cell
    max) is unchanged; `pad_or_truncate()` is unchanged
  - **Files** — `crates/grove-cli/src/tui/scroll_buffer.rs`

- [x] **Add unit tests for `compute_col_widths`**
  - **Delivers** — regression protection for column layout
  - **Done when** — tests in `scroll_buffer.rs` `mod tests` cover: table fits
    without shrinking, single wide column absorbs all excess, multiple columns
    tied for widest share shrinkage, all columns at minimum triggers proportional
    fallback, zero available width returns all-1s, single column table, empty
    table returns empty vec
  - **Files** — `crates/grove-cli/src/tui/scroll_buffer.rs`
