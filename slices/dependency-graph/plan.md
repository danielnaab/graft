---
status: done
created: 2026-02-24
resolve_before_implementing:
  - "Does any current consumer need this beyond inline validation?"
  - "Should this wait until sequences exist (its primary consumer)?"
---

# Compute command dependency graph from writes/reads declarations

## Story

The `state_name -> producer_command` and `command -> required_states` mappings
are currently computed inline (a linear scan in the error path of
`setup_run_state`). As more commands declare reads/writes and sequences need
validation, a first-class dependency graph structure enables validation,
observability, and better error messages without repeated ad-hoc scans.

## YAGNI Assessment

With one producer-consumer pair (implement/resume), the linear scan works fine.
This slice is justified when:

- Sequences exist and need parse-time validation ("does every reads: have a
  matching writes: from a prior step?")
- Grove needs to render dependency relationships (producer/consumer labels in
  run-state view)
- Three or more state names exist, making the linear scan inefficient

Currently none of these conditions are met. The Grove run-state view can do its
own inline scan of `available_commands` for producer/consumer info. Consider
implementing this slice when a second consumer of the graph structure emerges.

## Approach (tentative)

Add a `DependencyGraph` struct to graft-engine:

```rust
pub struct DependencyGraph {
    producers: HashMap<String, String>,      // state_name -> command_name
    consumers: HashMap<String, Vec<String>>, // state_name -> [command_names]
}
```

Computed from `GraftConfig` by scanning all commands' `writes` and `reads`
fields. Validates: no duplicate producers for the same state name, all reads
reference a known state name.

## Acceptance Criteria

- `DependencyGraph::from_config(&GraftConfig)` produces the correct mappings
- Duplicate producers for the same state name produce a validation error
- Reads referencing unknown state names produce a warning (not error — the state
  may come from run-state written outside graft)
- `setup_run_state` uses the graph instead of inline scanning
- `cargo test` passes with no regressions

## Steps

TBD — assess need when sequences or a second graph consumer emerge.
