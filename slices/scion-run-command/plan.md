---
status: done
created: 2026-03-08
depends_on: []
completed: 2026-03-08
completed_note: "Source validation against local state section was removed because source may reference a dependency's state query (e.g., slices from software-factory). The source field is parsed and stored but not yet consumed for completion — :scion run <tab> completes from existing scion names, not from state query results. Source-based completion is a follow-up."
---

# Collapse scion create + start into `:scion run`

## Story

As a developer implementing a slice, I want to type one command —
`:scion run grove-smart-column-widths` — instead of the two-step
`:scion create` then `:scion start` ceremony, so that starting work
on a slice is a single action.

As a graft.yaml author, I want to declare `source:` on my scions
config so that `:scion run` provides tab completion for scion names
from a state query, without graft needing to know what domain concept
(slice, suite, task) those names represent.

## Approach

Add `source: Option<String>` to `ScionHooks` — a state query name
used for tab completion of the scion name argument. Parse it from
the `scions:` section of graft.yaml alongside `start:`.

Add a `scion run` subcommand to graft-cli and a `:scion run` command
to grove. The implementation is straightforward: call `scion_create`,
then call `scion_start`. If the scion already exists (worktree
present), skip creation and just start. If a session is already
active, report it and offer to attach instead.

In grove, `:scion run <tab>` completes from the `source` state query
when configured. In graft-cli, `graft scion run <name>` does the
same create-then-start sequence.

The root `graft.yaml` adds `source: slices` to its scions config so
that the software-factory slice names are available for completion.

## Acceptance Criteria

- `graft scion run <name>` creates the scion if it doesn't exist,
  then starts it; exits cleanly with session ID
- `graft scion run <name>` when the scion already exists but has no
  active session just starts it (no error about existing worktree)
- `graft scion run <name>` when a session is already active prints
  a message suggesting `:attach` instead (does not start a second)
- `:scion run <name>` in grove does the same: create if needed,
  start, clear scion completions cache
- `scions.source` is parsed from graft.yaml as an optional string;
  validation rejects a source that doesn't exist in the `state:`
  section
- `:scion run <tab>` in grove completes from the `source` state
  query when configured; without `source`, no completion
- Existing `scion create` and `scion start` commands are unchanged
- `cargo test` passes with no regressions

## Steps

- [x] **Add `source` field to `ScionHooks` and parse from graft.yaml**
  - **Delivers** — scions config can declare a state query for name completion
  - **Done when** — `ScionHooks` gains `pub source: Option<String>`;
    `config.rs` parses `source:` from the scions section as an optional
    string; validation rejects a `source` value that isn't a key in the
    `state:` section (same pattern as context entry validation); a parsing
    test asserts `source: slices` round-trips correctly; a test asserts
    an unknown source name is rejected
  - **Files** — `crates/graft-engine/src/domain.rs`,
    `crates/graft-engine/src/config.rs`

- [x] **Add `scion_run` engine function**
  - **Delivers** — shared create-then-start logic for CLI and grove
  - **Done when** — `scion.rs` has a `pub fn scion_run()` with the same
    signature as `scion_start` plus the same config/dep_configs args as
    `scion_create`; it calls `scion_create` and if the error is "already
    exists" (worktree path exists), it continues; it then calls
    `scion_start`; if `scion_start` fails because a session is already
    active, it returns a specific error (not a generic one) so callers
    can suggest attach; a test creates and runs a scion in one call; a
    test asserts that running an existing scion just starts it; a test
    asserts that running a scion with an active session returns the
    appropriate error
  - **Files** — `crates/graft-engine/src/scion.rs`

- [x] **Add `graft scion run` CLI subcommand**
  - **Delivers** — one-command scion workflow from the terminal
  - **Done when** — `ScionCommands` enum gains a `Run { name: String }`
    variant; the handler calls `scion_run` from the engine; on success
    prints the worktree path and session ID; on "session already active"
    error prints a message suggesting `graft scion attach <name>`;
    `graft scion run --help` shows usage
  - **Files** — `crates/graft-cli/src/main.rs`

- [x] **Add `:scion run` grove command with source-based completion**
  - **Delivers** — one-command scion workflow from grove with tab completion
  - **Done when** — `CliCommand` enum gains a `ScionRun(String)` variant;
    `parse_command` handles `:scion run <name>`; `cmd_scion_run` calls
    the engine's `scion_run`, pushes a success or error block to the
    transcript, and clears the scion completions cache; tab completion
    for `:scion run ` resolves completions from the `scions.source`
    state query (using the same `options_from`-style resolution as
    command args); without `source` configured, no completions are
    offered
  - **Files** — `crates/grove-cli/src/tui/transcript.rs`,
    `crates/grove-cli/src/tui/command_line.rs`

- [x] **Add `source: slices` to root graft.yaml**
  - **Delivers** — tab completion for slice names when running scions
  - **Done when** — root `graft.yaml` has `scions: { start: "software-factory:agent", source: slices }`; `graft scion run <tab>` and
    `:scion run <tab>` complete from the slices state query
  - **Files** — `graft.yaml`
