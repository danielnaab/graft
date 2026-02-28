---
status: done
created: 2026-02-28
---

# Declare entities on state queries; add focus to grove

## Story

Grove's `extract_options_from_state` hardcodes three conventions that couple it to
the software-factory graft's JSON shape:

1. **Collection location** — looks for `data[query_name]` as the array
2. **Value extraction** — tries `path` (strips to parent dir), then `name`, then bare string
3. **Filtering** — skips items where `status == "done"`

A different graft returning `{"results": [{"id": "abc"}]}` gets nothing — wrong
collection key, wrong value field. The coupling also blocks a useful UX pattern:
when a user is working on a single entity (a slice, an environment, a migration),
every command invocation requires re-typing the same argument. There is no way to
say "I'm working on this one" and have grove fill it in.

This slice solves both problems:

- **`entity` declaration** on state queries in `graft.yaml` — replaces hardcoded
  extraction with a per-query declaration of collection key and identity field.
- **Focus mechanism** in grove — `:focus`/`:unfocus` commands let the user select
  a default value per state query. Commands whose args use `options_from` auto-fill
  from focus when the user omits the argument.

## Approach

### 1. `entity` block on state queries

Add an optional `entity` block to state query definitions:

```yaml
state:
  <query-name>:
    run: string
    cache: ...
    timeout: integer
    entity:              # Optional: declares this query returns a collection
      key: string        # Required: field name for identity value
      collection: string # Optional: JSON key for the array (default: query name)
```

**`key`** names the JSON field whose value is used for `options_from` resolution
and focus. For each object in the collection array, `entity.key` is extracted.

**`collection`** defaults to the query name. Override when the JSON key differs:

```yaml
  active-tasks:
    run: "..."
    entity:
      collection: tasks    # JSON key differs from query name
      key: id
```

**No filtering.** The hardcoded `status != "done"` filter is removed. The state
query script controls what it returns — if a graft wants to exclude completed
items, its script should filter them. The format does not encode business logic.

**Backward compatible.** Queries without `entity` use the existing extraction
conventions. Nothing breaks. Grafts opt in by adding `entity`.

#### Examples

```yaml
# Software-factory graft
state:
  slices:
    run: "bash scripts/list-slices.sh"
    entity:
      key: slug

# Migrations graft
state:
  migrations:
    run: "bash scripts/list-migrations.sh"
    entity:
      collection: migrations
      key: name

# Deployments graft — command + state together
commands:
  deploy:
    run: "bash scripts/deploy.sh {env}"
    args:
      - name: env
        type: choice
        options_from: environments

state:
  environments:
    run: "bash scripts/list-envs.sh"
    entity:
      key: name
```

### 2. Focus mechanism

**Focus is a selected value from a state query, used as the default argument for
commands that reference that query via `options_from`.** It is a default-argument
mechanism tied to state queries, not a display filter.

#### Focus state

```rust
/// Per-query focus: maps query name → selected entity value.
pub(super) focus: HashMap<String, String>,
```

Focus is per-query. A user can focus on a slice AND an environment simultaneously.
A command with `options_from: slices` uses the slice focus; a command with
`options_from: environments` uses the environment focus.

#### `:focus` command

Three modes:

```
:focus                          → list focusable queries and current focus values
:focus environments             → picker over environment entities
:focus environments staging     → set focus directly, no picker
```

Alias: `:f`

#### `:unfocus` command

```
:unfocus environments           → clear focus for that query
:unfocus                        → clear all focuses
```

Alias: `:uf`

#### Auto-fill on `:run`

When executing a command via `:run`:

1. For each required positional arg the user did not provide, check whether the
   arg has `options_from` set and whether `self.focus` contains a value for that
   query.
2. If both true, inject the focused value into the args vec.
3. Show a status message: `"Using focused slices: retry-logic"`.
4. **Explicit args always override focus.** `:run deploy production` uses
   `production` even if `staging` is focused.

#### Header display

`HeaderData` gains `focus: &'a HashMap<String, String>`. After the
branch/dirty/ahead-behind indicators, if focus is non-empty, append focus entries:

```
/path/to/repo (main) ● | slices: retry-logic
```

Separator in dark gray, query name in dim, value in cyan. Multiple focuses
separated by commas. Stale focus (value no longer in query results, checked
opportunistically when fresh data is available):

```
/path/to/repo (main) ● | slices: retry-logic (stale)
```

#### Completions

`:focus` completes query names (from `state_query_names()`). After a space,
completes entity values from the query's results (reuses `resolve_options_from`).

### Layered separation

| Layer | Concern |
|-------|---------|
| **graft.yaml format** | State queries declare entity structure (`entity.key`) |
| **`options_from`** | Commands declare which query provides their arg values |
| **grove focus** | User selects a default value per query |
| **auto-fill** | When running a command, check if its `options_from` query has a focus |

No layer knows about software-factory, slices, or transactions.

## Acceptance Criteria

- `graft-yaml-format.md` documents the `entity` block on state queries
- `StateQueryDef` gains `entity: Option<EntityDef>` with `key: String` and
  `collection: Option<String>`
- `extract_options_from_state` uses `entity` when present; preserves existing
  behavior when absent
- `StateQuery` in grove carries entity fields from `StateQueryDef`
- `:focus` and `:unfocus` commands work (all three modes each)
- `:focus` completions show query names, then entity values
- `:run` auto-fills from focus when args are omitted; explicit args override
- Header shows current focus state; stale detection is opportunistic
- `cargo test && cargo clippy -- -D warnings && cargo fmt --check` passes

## Steps

- [x] **Spec and parse `entity` on state queries**
  - **Delivers** — format spec + parsing for the new declaration
  - **Done when** — `graft-yaml-format.md` documents `entity` block (key required,
    collection optional defaulting to query name); `EntityDef` struct added to
    `graft-common/src/config.rs` with `key: String` and
    `collection: Option<String>`; `StateQueryDef` gains
    `entity: Option<EntityDef>`; `parse_state_query` deserializes entity; unit
    tests cover: entity with both fields, entity with only key (collection
    defaults), no entity (backward compat)
  - **Files** — `docs/specifications/graft/graft-yaml-format.md`,
    `crates/graft-common/src/config.rs`

- [x] **Use `entity` declaration in `extract_options_from_state`**
  - **Delivers** — generic entity extraction replacing hardcoded conventions
  - **Done when** — `extract_options_from_state` signature gains
    `entity: Option<&EntityDef>`; when `Some`, uses `entity.collection` (falling
    back to query name) to find the array and `entity.key` to extract values from
    each object; when `None`, preserves existing hardcoded behavior;
    `resolve_options_from` threads entity through from config; `StateQuery` in
    `grove-cli/src/state/query.rs` gains entity fields; unit tests cover: entity
    extraction with explicit collection, entity extraction with default collection,
    fallback to hardcoded logic
  - **Files** — `crates/grove-cli/src/tui/transcript.rs`,
    `crates/grove-cli/src/state/query.rs`,
    `crates/grove-cli/src/tui/tests.rs`

- [x] **Add focus state and `:focus`/`:unfocus` commands**
  - **Delivers** — the core focus mechanism
  - **Done when** — `TranscriptApp` gains `focus: HashMap<String, String>`;
    `CliCommand` gains `Focus(Option<String>, Option<String>)` and
    `Unfocus(Option<String>)` variants; `parse_command` handles `focus`/`f` and
    `unfocus`/`uf`; `PALETTE_COMMANDS` includes both; `cmd_focus` handles three
    modes (list all, picker, direct set); `cmd_unfocus` handles two modes (clear
    one, clear all); `execute_cli_command` dispatches to both; tests cover: parse
    round-trip for both commands, focus set/get/clear, unfocus-all
  - **Files** — `crates/grove-cli/src/tui/command_line.rs`,
    `crates/grove-cli/src/tui/transcript.rs`,
    `crates/grove-cli/src/tui/tests.rs`

- [x] **Auto-fill focused args in `:run`**
  - **Delivers** — focus actually changes behavior for command execution
  - **Done when** — `cmd_run` checks each arg's `options_from` against
    `self.focus`; missing args with a matching focus are injected; explicit args
    override focus; a status message ("Using focused slices: retry-logic") is
    pushed to the transcript; tests cover: auto-fill from focus, explicit override,
    no focus no auto-fill, multi-arg with partial focus
  - **Files** — `crates/grove-cli/src/tui/transcript.rs`,
    `crates/grove-cli/src/tui/tests.rs`

- [x] **Show focus in header and add completions**
  - **Delivers** — visual feedback for focus state + discoverability
  - **Done when** — `HeaderData` gains `focus: &'a HashMap<String, String>`;
    `render_header` appends focus entries after branch indicators (separator dark
    gray, query name dim, value cyan); stale detection checks in-memory state
    opportunistically; `compute_completions` handles `:focus` — first arg
    completes query names, second arg completes entity values via
    `resolve_options_from`; tests cover: header with focus, header with stale
    focus, completions for focus args
  - **Files** — `crates/grove-cli/src/tui/header.rs`,
    `crates/grove-cli/src/tui/prompt.rs`,
    `crates/grove-cli/src/tui/tests.rs`
