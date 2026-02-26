---
status: done
created: 2026-02-26
depends_on: [sequence-declarations]
---

# Expose command and sequence metadata via `graft help`

## Story

A consuming repo currently has no way to discover what software-factory commands do,
what arguments they require, what state they consume or produce, or how sequences are
structured — without reading raw YAML. The data is fully parsed into Rust structs at
runtime; it just isn't surfaced. `graft run software-factory` lists command names and
descriptions. `graft run software-factory:implement --help` doesn't work. There is no
way to ask "what does `verify` write?" or "what does `implement-verified` do step by
step?" without opening `.graft/software-factory/graft.yaml` directly.

This matters especially for orchestration: an agent or CI script deciding which
commands to run, in what order, and with what arguments, has no machine-readable
interface to reason about the command catalog. The `reads`/`writes` data-flow graph is
the richest piece — it encodes which commands are prerequisites for which — and it is
completely invisible today.

## Approach

Three additions:

**1. `category` and `example` fields in the schema**


Add two optional fields to `CommandDef` and `SequenceDef`:

```yaml
commands:
  implement:
    category: core        # core | diagnostic | optional | advanced
    example: "graft run software-factory:implement slices/my-feature"
    description: ...
    run: ...
```

`category` classifies commands by role:
- `core` — primary workflow steps (`implement`, `verify`, `approve`)
- `diagnostic` — run when something is wrong (`diagnose`, `resume`)
- `optional` — enrichment steps (`spec-check`, `review`, `diagnose`)
- `advanced` — power-user tools (`implement-parallel`)

`example` is a concrete invocation string shown in help output.

**2. `graft help <dep>` and `graft help <dep>:<name>`**

`graft help` is a single subcommand with two modes depending on whether a specific
command name is provided.

`graft help software-factory` (dep only) prints the full catalog — all commands and
sequences with key metadata:

```
$ graft help software-factory

Commands:
  implement          [core]      Implement next slice step with Claude Code
                                 Reads: session  Writes: session, context-snapshot
  verify             [core]      Run consumer project verification
                                 Reads: —        Writes: verify
  resume             [diagnostic] Resume implementation after verify failure
                                 Reads: session, verify  Writes: session

Sequences:
  implement-verified [core]      Implement a slice and verify it passes, with retry
                                 Steps: implement → verify → review  Checkpoint: yes
  implement-reviewed [core]      ...
```

`graft help software-factory:implement` (dep + name) prints full metadata for that
specific command or sequence:

```
$ graft help software-factory:implement

  implement — Implement next slice step with Claude Code
  Category: core
  Example:  graft run software-factory:implement slices/my-feature

  Arguments:
    slice  (string, required, positional)  Path to the slice directory

  Reads:   session
  Writes:  session, context-snapshot
```

For sequences:

```
$ graft help software-factory:implement-verified

  implement-verified — Implement a slice and verify it passes, with retry
  Category: core
  Example:  graft run software-factory:implement-verified slices/my-feature

  Steps:    implement → verify → review
  Retry:    verify fails → resume (max 3)
  Checkpoint: yes (human approval required)

  Arguments:
    slice  (string, required, positional)  Path to the slice directory
```

`--json` works on both modes:
- `graft help software-factory --json` → full catalog as JSON
- `graft help software-factory:implement --json` → single command as JSON

The JSON catalog shape:

```json
{
  "commands": {
    "implement": {
      "description": "...",
      "category": "core",
      "example": "...",
      "args": [{"name": "slice", "type": "string", "required": true, "positional": true, "description": "..."}],
      "reads": ["session"],
      "writes": ["session", "context-snapshot"]
    }
  },
  "sequences": {
    "implement-verified": {
      "description": "...",
      "category": "core",
      "steps": ["implement", "verify", "review"],
      "on_step_fail": {"step": "verify", "recovery": "resume", "max": 3},
      "checkpoint": true,
      "args": [...]
    }
  }
}
```

Both modes resolve the dep, load its config, and print metadata only — no execution.

**4. Reads/writes shown before command execution**

When `graft run` executes a command that has non-empty `reads` or `writes`, print
them before the command starts:

```
Executing: verify
  Run consumer project verification
  Reads:   —
  Writes:  verify
```

This gives operators immediate confirmation of what state the command will consume
and produce, without having to consult the catalog separately.

**5. Populate `category` and `example` in software-factory's `graft.yaml`**

Add `category` and `example` to every command and sequence in
`.graft/software-factory/graft.yaml`. This is the concrete payoff that makes the
new fields useful immediately.

## Acceptance Criteria

- `CommandDef` and `SequenceDef` in `graft-common/src/config.rs` each have
  `category: Option<String>` and `example: Option<String>`
- `graft help software-factory:implement` prints description, category, example,
  args (with type, required, description), reads, writes
- `graft help software-factory:implement-verified` prints description, category,
  example, steps, retry config, checkpoint flag, args
- `graft help software-factory` lists all commands and sequences with description,
  category, reads/writes (commands) or steps + checkpoint (sequences)
- `graft help software-factory --json` emits valid JSON with full metadata for all
  commands and sequences
- `graft help software-factory:implement` prints description, category, example,
  args (with type, required, description), reads, writes
- `graft help software-factory:implement-verified` prints description, category,
  example, steps, retry config, checkpoint flag, args
- `graft help software-factory:implement --json` emits the single command as JSON
- `graft help <dep>:<name>` when the name doesn't exist exits 1 with
  `"unknown command: <name>"`
- `graft run` prints `Reads:` / `Writes:` lines before executing any command that
  declares them (empty reads/writes prints nothing for that line)
- Every command and sequence in `.graft/software-factory/graft.yaml` has a
  `category` and `example`
- `docs/specifications/graft/graft-yaml-format.md` documents `category` and `example`
  fields before implementation
- `cargo test && cargo clippy -- -D warnings && cargo fmt --check` passes

## Steps

- [ ] **Spec: add `category` and `example` to `graft-yaml-format.md`**
  - **Delivers** — unambiguous spec for new fields before any code is written
  - **Done when** — `graft-yaml-format.md` documents `category` (type: string,
    valid values: `core | diagnostic | optional | advanced`, optional) and `example`
    (type: string, a complete invocation example, optional) in both the `commands:`
    and `sequences:` sections; documents `graft help <dep>` (catalog mode) and
    `graft help <dep>:<name>` (detail mode) with `--json` flag behavior; spec
    complete before implementation begins
  - **Files** — `docs/specifications/graft/graft-yaml-format.md`

- [ ] **Add `category` and `example` to `CommandDef` and `SequenceDef` in
  `config.rs` and `domain.rs`**
  - **Delivers** — schema supports new fields end-to-end
  - **Done when** — `graft-common/src/config.rs` adds `category: Option<String>`
    and `example: Option<String>` to `CommandDef` and `SequenceDef`; corresponding
    fields added to `Command` and `SequenceDef` in `graft-engine/src/domain.rs`
    and `grove-core/src/domain.rs`; existing graft.yaml files without these fields
    parse without error (optional fields); unit tests confirm round-trip parse;
    `cargo test && cargo clippy -- -D warnings && cargo fmt --check` passes
  - **Files** — `crates/graft-common/src/config.rs`,
    `crates/graft-engine/src/domain.rs`, `crates/grove-core/src/domain.rs`

- [ ] **Add `graft help` subcommand (catalog and detail modes) and pre-execution
  reads/writes output**
  - **Delivers** — consuming repos can discover full command metadata via CLI and
    machine-readable JSON; execution output confirms data-flow before commands run
  - **Done when** — `graft-cli/src/main.rs` handles `graft help <dep>` (no `:`):
    resolves dep, loads config, prints all commands then all sequences in tabular
    format with description, category, reads/writes (commands) or steps/checkpoint
    (sequences); handles `graft help <dep>:<name>`: resolves dep, loads config,
    looks up command or sequence by name, prints full metadata block; exits 1 with
    clear error if name not found; `--json` flag works on both modes: catalog mode
    emits the full commands+sequences JSON object, detail mode emits the single
    command or sequence as JSON (using `serde_json`);
    `run_current_repo_command` and `run_dependency_command` print `Reads: <list>`
    and `Writes: <list>` lines before executing any command with non-empty reads or
    writes (omit the line entirely when the list is empty); unit tests assert help
    output for catalog mode, detail mode (command), detail mode (sequence), and
    `--json` round-trip; `cargo test && cargo clippy -- -D warnings && cargo fmt
    --check` passes
  - **Files** — `crates/graft-cli/src/main.rs`

- [ ] **Populate `category` and `example` in software-factory `graft.yaml`**
  - **Delivers** — every software-factory command and sequence is immediately
    self-documenting via `graft help` and `graft catalog`
  - **Done when** — every command in `.graft/software-factory/graft.yaml` has both
    `category` (one of: `core`, `diagnostic`, `optional`, `advanced`) and `example`
    (a complete `graft run software-factory:<name> [args]` string); every sequence
    likewise; manual verification: `graft help software-factory` produces complete,
    accurate output for all entries; `graft help software-factory --json` is valid JSON
  - **Files** — `.graft/software-factory/graft.yaml`
