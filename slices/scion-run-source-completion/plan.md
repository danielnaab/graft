---
status: done
created: 2026-03-08
depends_on:
  - scion-run-command
---

# Source-based completion and structured errors for scion run

## Story

As a developer using `:scion run <tab>`, I want to see all available
slice names — not just scions that already exist — so I can start a
new scion without knowing the exact name upfront.

As a maintainer of the graft codebase, I want scion error handling to
use structured error variants instead of string matching, so that
error dispatch doesn't silently break when messages change.

## Approach

### Source-based completion

The `scions.source` field already stores a state query name (e.g.,
`slices`). The `resolve_options_from` infrastructure in grove already
does three-tier lookup (disk cache → in-memory → subprocess fallback)
and entity-aware extraction. Wire `scion_completions()` to also
resolve the source query and merge those names into the completion
list alongside existing scions.

The merge strategy: source query names are the primary completions.
Existing scions that don't appear in the source results (orphaned
scions) are appended. Each completion shows its origin — source names
show the query status (e.g., `draft`, `accepted`), existing scions
show ahead count and session status. This gives the user both "what
can I start" and "what already exists" in one list.

To get the source query name at completion time, read
`scions.source` from the parsed graft config. The config is already
loaded in several scion command handlers; cache it in `RepoContext`
so `scion_completions()` can access it without re-parsing.

### Structured error variant

Add `SessionAlreadyActive { name: String }` to `GraftError`. Use it
in `scion_run()` instead of `CommandExecution`. Match on it in the
CLI handler and grove handler instead of string matching. This follows
the existing pattern of semantic variants like `DependencyNotFound`.

## Acceptance Criteria

- `:scion run <tab>` shows names from the source state query (e.g.,
  all slice slugs) when `scions.source` is configured
- `:scion run <tab>` still shows existing scions alongside source
  names (merged, deduplicated)
- Without `scions.source` configured, completion falls back to
  existing scions only (unchanged behavior)
- Source query resolution uses the existing three-tier lookup (disk
  cache, in-memory, subprocess fallback)
- `GraftError::SessionAlreadyActive { name }` variant exists
- `scion_run()` returns `SessionAlreadyActive` instead of
  `CommandExecution` for active sessions
- CLI and grove match on the variant, not on string content
- `cargo test && cargo clippy -- -D warnings && cargo fmt --check`
  passes

## Steps

- [x] **Add `SessionAlreadyActive` error variant**
  - **Delivers** — structured error dispatch for session conflicts
  - **Done when** — `GraftError` has `SessionAlreadyActive { name:
    String }` with display message "scion '{name}' already has an
    active session"; `scion_run()` uses it; CLI `scion_run_command`
    matches on `GraftError::SessionAlreadyActive { .. }` instead of
    string contains; grove `cmd_scion_run` matches on the variant;
    existing tests pass
  - **Files** — `crates/graft-engine/src/error.rs`,
    `crates/graft-engine/src/scion.rs`,
    `crates/graft-cli/src/main.rs`,
    `crates/grove-cli/src/tui/transcript.rs`

- [x] **Cache parsed graft config in RepoContext**
  - **Delivers** — scion completion can access source query name
  - **Done when** — `RepoContext` has `cached_graft_config:
    Option<GraftConfig>`; populated lazily on first access (same
    pattern as `cached_state_queries`); invalidated on repo switch
    and `:refresh`; `scion_completions()` reads `source` from the
    cached config
  - **Files** — `crates/grove-cli/src/tui/transcript.rs`

- [x] **Resolve source query in scion_completions and merge results**
  - **Delivers** — `:scion run <tab>` shows all available names
  - **Done when** — `scion_completions()` checks
    `cached_graft_config.scion_hooks.source`; if set, calls
    `resolve_options_from(source, repo_name)` to get source names;
    merges with existing scion names (source names first, then
    orphaned scions); deduplicates by name; source-only names show
    description from query data (e.g., status); existing scions show
    ahead/session info; result is cached in
    `cached_scion_completions`; a test asserts source names appear
    in completions
  - **Files** — `crates/grove-cli/src/tui/transcript.rs`,
    `crates/grove-cli/src/tui/tests.rs`
