---
status: draft
created: 2026-02-24
resolve_before_implementing:
  - "Argument passing: union from steps, explicit on sequence, or pass-through?"
  - "Top-level `sequences:` key vs compound commands (list of refs in `run:`)?"
  - "Is this worth the complexity vs shell `&&` composition?"
---

# Declare named command sequences in graft.yaml

## Story

Multi-step command flows (implement then verify, plan then implement then verify)
are currently composed ad-hoc in shell. The sequence is invisible to graft and
Grove — there's no way to name it, re-run it, or observe its progress. This
slice adds a `sequences:` declaration to graft.yaml so multi-step flows are
first-class.

## Open Questions

**Argument passing.** `implement` takes `{slice}` (choice, positional).
`verify` takes nothing. A sequence `[implement, verify]` needs to accept `slice`
and route it to `implement` only. Options:

1. Sequence declares its own `args:`, steps reference them — most explicit, most verbose
2. Union args from all steps, pass to each (steps ignore unknown args) — simple but fragile at scale
3. Each step in the sequence specifies its arg bindings — workflow-engine territory

**Primitive shape.** Two alternatives to a new `sequences:` top-level key:

- **Compound commands**: a command's `run:` can be a list of command names
  instead of a shell string. No new config key, but overloads `run:` semantics.
- **Shell composition**: just use `graft run X && graft run Y` in a regular
  command. Zero framework changes, but graft-calling-graft is inelegant and
  prevents resumability.

**Value proposition.** Visibility (named in config), resumability (graft tracks
progress), Grove observability (pipeline state). If Grove run-state view
(separate slice) provides sufficient observability without sequences, the
urgency drops.

## Approach (tentative)

Add `sequences:` as a sibling to `commands:` and `state:` in graft.yaml:

```yaml
sequences:
  build:
    description: "Implement and verify a slice"
    steps: [implement, verify]
    args:
      - name: slice
        type: choice
        options_from: slices
```

`graft run build my-slice` executes `implement my-slice` then `verify`, stopping
on first failure (exit non-zero).

Parsing in `config.rs`, new `Sequence` struct in `domain.rs`. Execution in
`command.rs` — iterate steps, call `execute_command_with_context` for each.

## Acceptance Criteria

- A sequence in graft.yaml is parsed and validated (all referenced commands exist)
- `graft run <sequence> [args]` executes steps in order, stopping on failure
- Arguments are passed to steps that accept them
- `cargo test` passes with no regressions

## Steps

TBD — resolve open questions first.
