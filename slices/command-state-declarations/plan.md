---
status: draft
created: 2026-02-23
---

# Add writes/reads state declarations to graft Command schema

## Story

Commands in graft.yaml have no way to declare what state they produce or
require, making data dependencies between commands invisible. This slice adds
`writes:` and `reads:` fields to the Command struct so authors can express
these relationships — and graft can validate them — before the runtime
enforcement is implemented.

## Approach

Extend `Command` in `crates/graft-engine/src/domain.rs` with two new optional
`Vec<String>` fields: `writes` and `reads`. Wire parsing in `config.rs` using
the same pattern as the existing `context` field (sequence of strings). Add
validation in `Command::validate()` ensuring entries are non-empty and
well-formed. Extend the CLI's command description output to show declared
dependencies when present. No behavior change to command execution — this
slice is purely structural, setting up the next slice's runtime implementation.

The two fields are intentionally symmetric in shape to `context:` so authors
see a consistent vocabulary: `context` = state this command reads at execution
time, `reads` = state this command *requires* to exist before running,
`writes` = state this command produces after running. The distinction between
`context` and `reads` will matter when enforcement lands; for now, parsing
both correctly is what matters.

## Acceptance Criteria

- A command in graft.yaml can declare `writes: [session]` and `reads:
  [session]` without error
- `graft validate` (or equivalent config validation path) accepts these fields
  on valid configs and rejects empty-string entries
- Existing graft.yaml files without these fields are unaffected
- The fields round-trip through the domain type correctly (parsed from YAML,
  accessible as `command.writes` / `command.reads`)
- `cargo test` passes with no regressions

## Steps

- [ ] **Add writes/reads fields to Command struct and validate**
  - **Delivers** — the schema accepts and validates the new fields
  - **Done when** — `Command` has `pub writes: Vec<String>` and `pub reads:
    Vec<String>` (both `#[serde(default)]`); `Command::validate()` rejects
    empty-string entries in either list; existing tests pass; a new unit test
    in `domain.rs` confirms a command with valid writes/reads validates
    successfully and one with an empty-string entry returns an error
  - **Files** — `crates/graft-engine/src/domain.rs`

- [ ] **Parse writes/reads from graft.yaml**
  - **Delivers** — real configs can use the new fields
  - **Done when** — `parse_graft_yaml_str` in `config.rs` parses `writes:` and
    `reads:` from command objects using the same sequence-of-strings pattern as
    `context:`; a new config-parsing test exercises a command with both fields
    and asserts the parsed values; an unknown field in either list produces a
    validation error (not a panic)
  - **Files** — `crates/graft-engine/src/config.rs`
