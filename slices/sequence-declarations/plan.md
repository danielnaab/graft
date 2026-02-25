---
status: accepted
created: 2026-02-24
---

# Declare and run named command sequences in graft.yaml

## Story

Multi-step command flows (implement then verify, plan then review) are currently
composed ad-hoc in shell scripts that are invisible to graft, unobservable in
grove, and must be rewritten for each new workflow. This slice adds a `sequences:`
section to graft.yaml so multi-step flows are named, validated, and runnable as
first-class graft primitives — showing up in grove's Commands section alongside
single commands.

## Approach

Add `sequences:` as a new top-level key in `graft.yaml`, parsed by graft-common
and represented in grove-core as a `Sequence` domain type. A sequence declares an
ordered list of command references (`steps`) and its own `args:` that are passed
through to every step (steps ignore args they don't accept — "pass-all" semantics).
`graft run <sequence-name> [args]` resolves the sequence, validates that all
referenced commands exist, and executes them in order via the existing
`execute_command_by_name`. First failure stops the sequence.

The sequence executor writes `sequence-state.json` to `$GRAFT_STATE_DIR` at the
start of each step, giving grove live visibility into which step is running. When
all steps succeed, `sequence-state.json` records `phase: complete`. Grove's
existing Run State section picks this up without code changes.

In grove, sequences appear in the Commands section rendered with a `»` prefix on
their name. Executing a sequence works identically to executing a command — Enter
shows the args form, which calls `graft run <sequence-name>` with the assembled
args. No new `DetailItem` variant is needed in this slice.

Design decisions (resolved from the prior draft):
- **Argument passing**: "pass-all" — sequence declares `args:`; all args are made
  available to each step for `{name}` template substitution. Steps whose `run:`
  template does not include a `{slice}` placeholder simply don't receive that value —
  no extra positional args are appended to the command line. This means `verify`
  (`run: "bash scripts/verify.sh"`) never sees the `slice` arg; `implement`
  (`run: "bash scripts/implement.sh {slice}"`) receives it correctly.
- **Shape**: new `sequences:` top-level key, not overloaded `run:` lists
- **Retry**: not in this slice — basic sequential execution only; retry comes in
  `sequence-retry`

## Acceptance Criteria

- A `sequences:` block in graft.yaml is parsed without error; unknown fields are
  rejected; all referenced command names are validated at parse time
- `graft run <sequence-name> [args]` executes all steps in order, printing each
  step's output as it runs; a non-zero exit from any step stops the sequence and
  exits with the same code
- Args declared on the sequence are passed to each step; steps that don't accept
  a given arg ignore it without error
- `sequence-state.json` is written to `$GRAFT_STATE_DIR` at the start of each step
  with `{sequence, step, step_index, step_count, phase: "running"}` and updated
  to `{phase: "complete"}` when all steps succeed or `{phase: "failed", step}` on
  failure
- Grove's Commands section shows sequences prefixed with `»` and they are
  executable via Enter exactly like single commands
- A sequence that references a nonexistent command is rejected at parse time with a
  clear error naming the missing command
- `cargo test` passes with no regressions

## Steps

- [ ] **Parse sequences: section in graft-common and grove-core**
  - **Delivers** — graft.yaml files can declare sequences without parse errors;
    the domain type is available for the execution layer
  - **Done when** — `graft-common` parses `sequences:` from graft.yaml into a
    `HashMap<String, SequenceDef>` where `SequenceDef` has `steps: Vec<String>`,
    `description: Option<String>`, and `args: Vec<ArgDef>`; `grove-core` defines
    a `Sequence` struct with `name: String`, `description: Option<String>`, and
    `args: Vec<ArgDef>` — identical shape to `Command` but a distinct type for
    domain clarity; `GraftConfig` gains a `sequences` field; a unit test parses a
    graft.yaml with a two-step sequence and asserts correct field values; referencing
    a non-existent command produces a validation error
  - **Files** — `crates/graft-common/src/config.rs`,
    `crates/grove-core/src/domain.rs`

- [ ] **Execute sequences in graft-engine with live state writing**
  - **Delivers** — `graft run <sequence-name>` works end-to-end; sequence progress
    is observable in grove via `sequence-state.json`
  - **Done when** — a new `execute_sequence(config, sequence_name, base_dir, args)`
    function in `graft-engine` iterates steps, calls `execute_command_by_name` for
    each, writes `sequence-state.json` to `$GRAFT_STATE_DIR` before each step and
    on completion/failure; `execute_command_by_name` is extended to try sequences
    if the name isn't found in `config.commands`; the `sequence-state.json` payload
    always includes the sequence name as `"sequence": "<name>"` so grove can derive
    the producer annotation by reading the file itself (rather than a `writes:` map);
    a unit test executes a two-step sequence of `echo` commands and asserts both run
    in order; a test asserts that a failing step stops the sequence
  - **Files** — `crates/graft-engine/src/sequence.rs` (new),
    `crates/graft-engine/src/command.rs`

- [ ] **Show sequences in grove Commands section**
  - **Delivers** — sequences are discoverable in grove and executable via Enter
  - **Done when** — `load_commands_for_selected_repo()` in `repo_detail.rs` also
    loads sequences from the parsed config (root + deps); each `Sequence` is
    converted to a `Command` value (mapping `name`, `description`, `args`) and
    appended to `available_commands` with the display name prefixed `» ` — no new
    `DetailItem` variant is introduced; the conversion is a simple struct-to-struct
    mapping since `Sequence` and `Command` have the same fields; selecting a
    sequence and pressing Enter shows the args form (or executes directly if no
    args); the `» ` prefix is stripped before calling `execute_command_with_args`
    so the actual graft call is `graft run <sequence-name>` not
    `graft run » <sequence-name>`; sequences appear in the Commands section below
    single commands; when a run-state entry named `sequence-state` is present,
    grove reads its `"sequence"` field and displays it as the producer annotation
    (e.g., `(← implement-verified)`) — this is handled in the run-state rendering
    path, not via the runs/writes producer map; the loaded sequences come from
    `graft_loader.load_graft()` which now returns them via `GraftConfig.sequences`;
    **note**: `graft run <sequence-name>` dispatch assumes the graft-cli's
    `run` subcommand reaches `execute_command_by_name` — verify the CLI entry point
    in `crates/graft-cli` passes the full config (with sequences) to the engine;
    existing command tests continue to pass
  - **Files** — `crates/grove-cli/src/tui/repo_detail.rs`,
    `crates/grove-engine/src/config.rs`
