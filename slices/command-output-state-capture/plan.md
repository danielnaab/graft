---
status: draft
created: 2026-02-23
---

# Implement command output state capture and reads enforcement

## Story

Commands that declare `writes:` should automatically persist their output as
named state queryable by subsequent commands; commands that declare `reads:`
should fail with a clear error if that state doesn't exist yet — making data
dependencies between commands enforceable at runtime, not just declarative.

## Approach

Introduce a run-state store at `.graft/run-state/` in the consumer repo root.
When a command with `writes: [foo]` completes successfully, graft reads
`.graft/run-state/foo.json` (written by the command itself) and registers it
as queryable state — accessible via `state.foo` in templates and via `graft
state query foo`. Commands write to this location via a well-known env var
`GRAFT_STATE_DIR` injected at execution time, so scripts know where to write
without hardcoding paths.

Before executing a command with `reads: [foo]`, graft checks that
`.graft/run-state/foo.json` exists and is valid JSON. If not, execution is
refused with a message naming the missing state and the command that produces
it (looked up from the config's `writes` declarations across all commands).

This builds directly on the declaration slice: `writes` and `reads` fields
must already be parsed. The state store is intentionally simple (JSON files,
no database) so it's inspectable and portable. The `.graft/run-state/`
directory should be gitignored — it's machine-local runtime state, not
source truth.

The factory's `implement` command is updated as a concrete end-to-end example:
it writes `session` state that `resume` reads — replacing the ad-hoc
`.session` file mechanism with the new graft-native one.

## Acceptance Criteria

- A command with `writes: [session]` running `echo '{"id":"abc"}' >
  $GRAFT_STATE_DIR/session.json` causes `graft state query session` to return
  `{"id":"abc"}` in subsequent invocations
- A command with `reads: [session]` run before any `writes: [session]` command
  has run exits with a non-zero code and a message identifying the missing
  state and which command produces it
- `GRAFT_STATE_DIR` is injected into every command's environment pointing to
  the consumer repo's `.graft/run-state/` directory
- `.graft/run-state/` is excluded from git via the consumer's `.gitignore`
  (graft adds it if absent, or documents the convention)
- `graft state query <name>` resolves run-state entries the same way it
  resolves configured state queries (no special-casing in callers)
- The factory's `implement` command writes `session` state and `resume` reads
  it, with the old `.session` file mechanism removed
- `cargo test` passes with no regressions

## Steps

- [ ] **Inject GRAFT_STATE_DIR and implement write-side persistence**
  - **Delivers** — commands can write named state that graft tracks
  - **Done when** — `execute_command_with_context` injects `GRAFT_STATE_DIR`
    pointing to `<consumer_root>/.graft/run-state/`; after a successful
    command run, for each name in `command.writes`, graft reads
    `$GRAFT_STATE_DIR/<name>.json` (if present) and stores it in an in-process
    state map; a unit test runs a command that writes a state file and asserts
    the written value is readable from the state map
  - **Files** — `crates/graft-engine/src/command.rs`

- [ ] **Make run-state queryable as first-class state**
  - **Delivers** — written state is accessible in templates and via CLI
  - **Done when** — `get_state()` in `state.rs` checks the run-state store
    before executing a configured state query (run-state takes precedence);
    `graft state query session` returns the value written by a prior `implement`
    run; state is available as `{{ state.session }}` in templates; a test
    exercises the full path from command execution through template rendering
  - **Files** — `crates/graft-engine/src/state.rs`,
    `crates/graft-engine/src/command.rs`

- [ ] **Enforce reads: preconditions before execution**
  - **Delivers** — missing dependencies produce clear errors rather than
    silent wrong behavior
  - **Done when** — before executing any command with non-empty `reads`,
    graft checks each named state exists in the run-state store; missing state
    produces an error: `"command 'resume' requires state 'session' (produced
    by: implement)"`; the lookup of which command produces a given state uses
    the full config's `writes` declarations; a test asserts this error on a
    command whose read dependency hasn't been satisfied
  - **Files** — `crates/graft-engine/src/command.rs`,
    `crates/graft-engine/src/config.rs` (for the reverse lookup)

- [ ] **Migrate factory implement/resume to use run-state**
  - **Delivers** — a concrete end-to-end example replacing ad-hoc .session files
  - **Done when** — `implement.sh` writes session ID to
    `$GRAFT_STATE_DIR/session.json` instead of `slices/<slug>/.session`;
    `resume.sh` reads from the same location; `graft.yaml` declares `implement`
    with `writes: [session]` and `resume` with `reads: [session]`; the old
    `.session` gitignore entry is removed; manual test: run implement then
    resume on the session-resume slice and confirm session continuity
  - **Files** — `.graft/software-factory/scripts/implement.sh`,
    `.graft/software-factory/scripts/resume.sh`,
    `.graft/software-factory/graft.yaml`,
    `.graft/software-factory/.gitignore`
