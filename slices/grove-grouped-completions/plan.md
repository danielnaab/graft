---
status: done
created: 2026-03-09
depends_on:
  - scion-run-source-completion
---

# Grouped scion completions

## Story

As a developer typing `:scion run <tab>`, I want to see slice names
grouped by status (in-progress, draft, accepted) so I can quickly
find the one I want to work on. Done items should be excluded, and
the grouping field should be declared in the entity config.

## UX

Completion popup with section headers:

```
┌─ Completions ─────────────────────────────┐
│  in-progress ───────────────────────────  │
│  grove-actionable-picker       +3         │
│  draft ─────────────────────────────────  │
│  new-feature-idea                         │
│  another-slice                            │
│  accepted ──────────────────────────────  │
│  ready-to-go                              │
└───────────────────────────────────────────┘
```

- Section headers: DarkGray + bold, non-selectable (arrow keys skip)
- Items: Cyan value + DarkGray description (unchanged)
- Group order: first-seen in state query output (not alphabetical)
- Existing scions show status annotation (`+3 [session]`, `exists`)
- Empty groups omitted
- Without `group_by` configured, flat list (unchanged behavior)

## Approach

### EntityDef gains `group_by`

```yaml
entity:
  key: slug
  group_by: status
```

One new field. `group_by` names the JSON field whose value becomes
each item's category. No `exclude` — the existing hardcoded
`status == "done"` filter stays (YAGNI; if declarative filtering is
needed later, it's a separate slice).

### Extraction returns grouped options

`extract_options_from_state` returns `Vec<GroupedOption>`:

```rust
pub struct GroupedOption {
    pub value: String,
    pub group: Option<String>,
}
```

When `group_by` is set, the group is populated from each item's
field. When absent, group is `None`. A named struct over a tuple —
this flows through three functions and self-documents at every site.

### `resolve_options_from` threads groups through

`resolve_options_from` is the 3-tier cache layer (disk → in-memory →
subprocess) between extraction and all consumers. It currently
returns `Vec<String>`. It changes to return `Vec<GroupedOption>` so
that group metadata survives the cache round-trip.

All three callers update:
- **`resolve_source_completions`** — preserves groups, passes to
  `scion_completions`
- **`commands_with_resolved_options`** — maps `.value` only (groups
  discarded; command completions stay flat)
- **`focus_entity_opts_for_buffer`** — maps `.value` only

### Scoped to scion completions

Groups only flow through the scion completion path:

```
extract_options_from_state → Vec<GroupedOption>
  → resolve_options_from → Vec<GroupedOption>       (3-tier cache)
    → resolve_source_completions → Vec<GroupedOption>
      → scion_completions → ArgCompletion { value, description, group }
        → render_palette + key handling (grouped)
```

The command completion pipeline (`commands_with_resolved_options` →
`ArgDef.options` → `compute_run_completions`) and focus pipeline
(`focus_entity_opts_for_buffer`) discard groups via
`.map(|o| o.value)`. Command/focus grouping is a future slice if
wanted.

### ArgCompletion gains `group`

```rust
pub struct ArgCompletion {
    pub value: String,
    pub description: String,
    pub group: Option<String>,  // NEW
}
```

### Completion popup groups when groups are present

When any `ArgCompletion` has `group` set:

**Ordering** — groups appear in first-seen order (the order they
appear in the state query output), not alphabetically. Within each
group, item order is preserved. This is done by assigning each group
a sequence index on first encounter, then stable-sorting by that
index. Data-driven ordering means the query controls the UX (e.g.,
in-progress before draft before accepted).

**Rendering** (`render_palette`) —
1. Inserts DarkGray bold section header rows at group boundaries
2. Accounts for header rows in popup height calculation

**Navigation** (key handling in `handle_completion_key`) —
1. Up/Down arrow skips header rows (they are non-selectable)
2. Selection index tracks only selectable items

## Acceptance Criteria

- `EntityDef` supports `group_by: Option<String>`
- `:scion run <tab>` shows options grouped by status with section
  headers when `group_by` is configured on the source query entity
- Section headers are visually distinct and non-selectable
- Without `group_by`, behavior is unchanged (flat list, no groups)
- Existing scion completions (no source configured) are unaffected
- `cargo test && cargo clippy -- -D warnings && cargo fmt --check`
  passes

## Steps

- [x] **Add `group_by` to EntityDef and return grouped options from
  extraction**
  - **Delivers** — group metadata available from state queries
  - **Done when** — `EntityDef` has `group_by: Option<String>`;
    `GroupedOption` struct added; `extract_options_from_state`
    returns `Vec<GroupedOption>`; group populated from `group_by`
    field when set; `resolve_options_from` returns
    `Vec<GroupedOption>`; callers of `resolve_options_from` updated
    (`commands_with_resolved_options` and
    `focus_entity_opts_for_buffer` map `.value` only;
    `resolve_source_completions` preserves groups);
    software-factory slices query declares `group_by: status`;
    existing tests updated; new test for grouped extraction
  - **Files** — `crates/graft-common/src/config.rs`,
    `crates/grove-cli/src/tui/transcript.rs`,
    `crates/grove-cli/src/tui/tests.rs`,
    `.graft/software-factory/graft.yaml`

- [x] **Thread groups into scion completions**
  - **Delivers** — `ArgCompletion` carries group for rendering
  - **Done when** — `ArgCompletion` has `group: Option<String>`;
    `scion_completions` populates `ArgCompletion.group` from
    `GroupedOption.group`; orphaned scions (not in source) get
    `group: None`
  - **Files** — `crates/grove-cli/src/tui/prompt.rs`,
    `crates/grove-cli/src/tui/transcript.rs`

- [x] **Render grouped completion popup**
  - **Delivers** — visual grouping with section headers
  - **Done when** — `render_palette` detects grouped completions;
    orders groups by first-seen index (data order, not alphabetical);
    inserts DarkGray bold header rows at group boundaries;
    key handling (`handle_completion_key` or equivalent) skips header
    rows on Up/Down; popup height accounts for headers; ungrouped
    items appear at end without header; empty groups omitted
  - **Files** — `crates/grove-cli/src/tui/prompt.rs`
