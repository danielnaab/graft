---
status: working
purpose: "Running list of grove UI issues and confusions from hands-on review"
---

# Grove UI Review (2026-03-06)

Running list of issues, confusions, and improvement opportunities from a hands-on
grove session. Each item describes the observed behavior, expected behavior, and
severity.

## Summary

**8 issues** across 4 themes:

| Theme | Issues | Severity |
|-------|--------|----------|
| **Grove can't execute state queries** | #1, #4a | High |
| **Transient errors vanish without trace** | #4b, #6b, #7 | Medium |
| **Navigation and visual clarity** | #2, #5, #8 | Medium-High |
| **Unhelpful error messages / missing arg prompts** | #3, #6a, #7 | High |

---

## 1. State queries show "(not cached)" with no way to populate

**Observed**: `:state` lists all discovered queries with `(not cached)`. Selecting
one (Enter) opens a detail view that also says "not cached". There is no way to
execute a query from within grove.

**Expected**: Selecting a query (or pressing a key like `r`) should execute it via
`graft state query <name>` and display the result. The current display-only design
means the panel is useless until the user independently runs graft outside grove.

**Severity**: High — the feature is effectively non-functional for new users.

**Notes**: The spec (`tui-behavior.md` lines 442-448) describes `r` refreshing
state queries, but the implementation only refreshes repo statuses. The code at
`transcript.rs:1264` returns `"(not cached)"` when `read_latest_cached()` returns
`None`. Grove would need to shell out to `graft state query <name>` or use the
engine directly to populate state.

---

## 2. Table columns clip to unreadable widths on narrow terminals

**Observed**: `:catalog` renders a 3-column table (Name, Description, Category).
When the terminal is narrower than the natural table width, all columns are
proportionally shrunk. Short columns like Name (6-20 chars) get squished to 2-3
characters, making the entire table unreadable.

**Expected**: Short columns (Name, Category) should keep a minimum readable width.
Only the longest column (Description) should absorb the overflow. A smarter
algorithm would: (1) set minimum widths per column, (2) shrink the widest column
first, or (3) truncate only Description and let Name/Category stay full-width.

**Severity**: Medium — the data is there but unusable at common terminal widths.

**Notes**: `compute_col_widths()` in `scroll_buffer.rs:753` uses pure proportional
scaling: `*w = (*w * available) / total_col`. This treats a 6-char Name the same
as a 60-char Description. The `pad_or_truncate()` function already handles
ellipsis truncation — the issue is purely in width allocation. This affects all
Table blocks (`:catalog`, `:state`, etc.), not just catalog.

---

## 3. Catalog selection runs commands without prompting for arguments

**Observed**: Selecting a command from the `:catalog` picker runs it immediately
with no arguments. Commands that require arguments (e.g. `diagnose` expecting
stdin or a prompt argument) fail with an error. There's no opportunity to enter
arguments before execution.

**Expected**: If the selected command has declared `args:` in its definition,
grove should prompt for them before executing — either by pre-populating the
command line with `:run software-factory:diagnose ` (cursor at end, ready for
input) or by showing an argument entry form.

**Severity**: High — selecting commands that need args always fails, making the
catalog picker a trap for those commands.

**Notes**: `transcript.rs:1196` hardcodes `CliCommand::Run(run_name, vec![])` as
the action for every catalog row. The arg definitions are available via
`graft_common::parse_commands_from_str()` (used by `graft help`), so grove could
check whether args are required and route to the command line instead. The
simplest fix would be to populate the prompt with `:run <name> ` instead of
executing immediately.

---

## 4. `:focus` fails silently for uncached queries (related to #1)

**Observed**: `:focus slices` shows `⚠ No values found for query: slices` as a
transient status bar warning, then the warning disappears after ~3 seconds. The
`slices` state query exists in graft.yaml but has never been executed, so there
are no cached values to pick from.

**Expected**: Two sub-issues:

**(a) Root cause**: Same as issue #1 — grove can't execute state queries, so any
query that hasn't been run via `graft` externally will fail here. If grove could
run queries on demand, `:focus` would trigger the query and present results.

**(b) Transient error messages vanish without a trace**: The warning is a
`StatusMessage::warning` that expires after 3 seconds (`status_bar.rs:89-90`).
Unlike command output which is appended to the scroll buffer and stays in the
transcript history, status messages are ephemeral overlays. If the user blinks
or is looking elsewhere, the error is gone with no way to see what happened.

**Severity**: Medium — (a) is a duplicate of #1; (b) is a general UX issue
affecting all status messages, not just this one.

**Notes**: Errors and warnings should arguably also be logged to the transcript
as a styled text block (e.g. dimmed, prefixed with `⚠` or `✗`) so they persist
in scroll history. The status bar can still show the flash notification, but
there should be a durable record. This affects all `StatusMessage::warning()`
and `StatusMessage::error()` calls throughout the TUI.

---

## 5. Block focus and scrolling navigation feels awkward

**Observed**: After running a command, reaching the output block requires pressing
Tab repeatedly (potentially many times if there are many blocks, since Tab starts
from block 0 and doesn't wrap). Up/Down arrows always line-scroll the viewport
regardless of context — they don't move between blocks or recall command history
when the prompt is inactive.

**Expected**: Several sub-issues:

**(a) Tab should prioritize recent blocks**: After a command completes, the user
almost always wants to interact with the newest block. Tab cycling from block 0
forward is the wrong default. Options: Shift+Tab from unfocused could jump to the
last block (it does this already), or Tab could wrap so you reach the end quickly,
or a dedicated key could jump to the most recent block.

**(b) Up/Down semantics are unclear**: When no block is focused and the prompt
is closed, Up/Down line-scroll the viewport — which feels like scrolling through
a log. But users might expect Up to recall the last command (shell-like) or move
focus to the previous block. The current behavior is consistent (always scroll)
but doesn't match either shell or vim mental models cleanly. Note: Up/Down DO
work as command history when the `:` prompt is open.

**(c) Focus can move off-screen**: Tab/Shift+Tab change `focused_block` without
auto-scrolling to reveal the focused block. You can focus a block that's entirely
off-screen with no visual feedback. Focus changes should scroll to show the
focused block.

**Severity**: Medium-High — navigation is the core TUI interaction and affects
every session.

**Notes**: Key code in `transcript.rs:471-530` (key handlers) and
`scroll_buffer.rs:563-591` (focus_next/focus_prev). Focus and scroll are fully
decoupled — `focused_block` is an index, `scroll_offset` is a line count, and
neither influences the other. Command history lives in `prompt.rs:534-578` and
only activates when the command line is open.

---

## 6. `:scion start` fails with confusing dependency error

**Observed**: `:scion create test` succeeds, then `:scion start test` fails with
`✗ scion start failed: command execution error: dependency 'software-factory' not
found`. The error is a transient status message that disappears after 3 seconds
(same class of issue as #4b).

**Expected**: The error message should explain what's actually wrong and how to
fix it. The `scions.start` field in `graft.yaml` is `software-factory:implement`,
which requires `load_dep_configs()` to find `.graft/software-factory/graft.yaml`.
If that file can't be loaded (submodule not checked out, gitignored, parse error),
the dependency list is empty and the lookup fails with a generic "not found" error.

**Severity**: High — scion workflow is completely broken from grove with no
actionable guidance. Two sub-issues:

**(a) Error message is unhelpful**: "dependency 'software-factory' not found"
doesn't tell the user *why* it wasn't found. Was the submodule not initialized?
Is `.graft/` gitignored? Did `parse_graft_yaml` fail silently? The
`load_dep_configs()` function at `config.rs:83` silently skips dependencies whose
`graft.yaml` is missing or invalid (`.ok()` on line 96-97), which masks the root
cause.

**(b) Transient error (duplicate of #4b)**: Same disappearing status message
pattern — the error flashes for 3 seconds then vanishes.

**Notes**: `transcript.rs:1696-1712` builds the config and dep_configs, then calls
`scion_start()`. The `scion.rs:727-736` lookup fails because dep_configs is empty.
The silent `.ok()` in `load_dep_configs` means grove can never tell the user *why*
the dependency wasn't found.

---

## 7. `:attach` error doesn't explain why there's no session

**Observed**: `:attach test` fails with `✗ command execution error: no active
session for scion 'test'`. The error is technically correct (the scion was created
but never successfully started due to issue #6), but the message doesn't help the
user understand what went wrong or what to do next.

**Expected**: The error should suggest next steps — e.g. "no active session for
scion 'test'. Run `:scion start test` first." Or better yet, offer to start it.
The user has to piece together that create succeeded, start failed (with its own
transient error they may have missed), and therefore attach has nothing to connect
to.

**Severity**: Low — downstream of #6, and the error is at least accurate. But
the pattern of unhelpful error messages compounds across the scion workflow.

**Notes**: `transcript.rs:1828-1833` calls `scion_attach_check()` and displays
the raw engine error. The error is again a transient `StatusMessage::error` that
disappears in 3 seconds (same as #4b, #6b).

---

## 8. Block focus highlight is too subtle and blocks bleed together

**Observed**: When a block is focused (via Tab), the visual distinction from
unfocused blocks is nearly invisible. The focus highlight is `Rgb(30, 30, 45)` —
a very faint dark blue tint that's almost indistinguishable from the default
black terminal background. Additionally, blocks are separated only by a single
blank line, making it hard to tell where one block ends and the next begins,
especially with multi-line text blocks.

**Expected**: Two sub-issues:

**(a) Focus highlight needs stronger contrast**: The background color should be
noticeably different — e.g. `Rgb(40, 40, 70)` or a border/gutter marker like
`▌` on the left edge of focused block lines. Users should immediately see which
block is selected.

**(b) Block boundaries need clearer separation**: A single blank line between
blocks is insufficient when blocks contain similar-looking content. Options:
a thin horizontal rule between blocks, alternating subtle background tints,
a left-margin gutter indicator, or more spacing.

**Severity**: Medium — affects readability and navigation confidence. Users can't
tell what's focused or where blocks start/end.

**Notes**: Focus highlight applied at `scroll_buffer.rs:667-670` via
`patch_style(Style::default().bg(Color::Rgb(30, 30, 45)))`. Block separation
is a single blank `Line::from("")` at `scroll_buffer.rs:644-645`. The Divider
content block type exists but is unused between regular blocks — it's only
pushed explicitly by commands.

---
