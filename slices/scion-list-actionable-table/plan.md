---
status: draft
created: 2026-03-02
depends_on: []
---

# Make :scion list an actionable table with per-row review action

## Story

`:scion list` is the only list command that renders as plain text instead of an
actionable table. `:repos`, `:catalog`, and `:state` all produce
`ContentBlock::Table` with per-row picker actions, but `:scion list` uses
`ContentBlock::Text` with manually formatted lines. To act on a scion after
listing, users must remember the name, open the prompt, and retype it — friction
that compounds during monitoring loops.

## Approach

Convert `cmd_scion_list` from rendering `ContentBlock::Text` to
`ContentBlock::Table` with proper headers and per-row actions. Each row's action
opens `:review <name>`, the most common follow-up. This follows the established
pattern: `:repos` → `CliCommand::Repo`, `:catalog` → `CliCommand::Run`,
`:state` → `CliCommand::State`, and now `:scion list` →
`CliCommand::Review`.

## Acceptance Criteria

- `:scion list` renders as a `ContentBlock::Table` with columns: Name,
  Ahead/Behind, Dirty, Session
- Pressing Enter on the table opens a picker with scion names
- Selecting a scion from the picker executes `:review <name>`
- Empty scion list still shows "No scions" message
- When tmux is unavailable, Session column shows "–" instead of being omitted
- `:help` output is unchanged (already documents Enter on tables)
- `cargo test && cargo clippy -- -D warnings && cargo fmt --check` passes

## Steps

- [ ] **Convert `:scion list` to render a `ContentBlock::Table` with review actions**
  - **Delivers** — scion list as an actionable table, consistent with other list commands
  - **Done when** — `cmd_scion_list` builds `headers`, `rows`, and
    `actions: Some(vec![CliCommand::Review(name, false)])` per scion; renders as
    `ContentBlock::Table` instead of `ContentBlock::Text`; Enter on the table
    opens a picker; selecting a row executes `:review <name>`; empty list still
    shows "No scions" text block; tests verify table structure and action wiring;
    `cargo test -p grove && cargo clippy -p grove -- -D warnings` passes
  - **Files** — `crates/grove-cli/src/tui/transcript.rs`,
    `crates/grove-cli/src/tui/tests.rs`
