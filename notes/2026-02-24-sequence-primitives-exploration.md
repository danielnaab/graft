---
title: "Sequence Primitives and Workflow Slice Planning"
date: 2026-02-24
status: working
participants: ["human", "agent"]
tags: [exploration, graft, grove, primitives, workflows, sequences, state, design]
---

# Sequence Primitives and Workflow Slice Planning

## Context

Continuation of the [command output state mapping](2026-02-23-command-output-state-mapping.md)
exploration. That session established the two-layer model (Claude explores, Grove
crystallizes) and identified four open design questions. This session resolves
those questions, proposes vertical slices, critiques them, and arrives at a
concrete plan.

---

## Resolving the Four Open Questions

### 1. Where do workflow declarations live?

**Resolution: graft.yaml, with Grove as a native consumer.**

graft.yaml is the portable unit. Dependencies ship their own graft.yaml, so
workflow declarations in graft.yaml travel with the dependency. Grove doesn't
need its own workflow config layer — it reads graft.yaml the same way it already
reads commands and state queries.

Cross-repo orchestration (workspace-level) is a real future use case, but
premature. Start with single-repo workflows in graft.yaml; Grove can add
cross-repo composition later without conflict.

### 2. State granularity

**Resolution: JSON files in `$GRAFT_STATE_DIR` are sufficient. No new machinery.**

The current model — command writes `$GRAFT_STATE_DIR/session.json`, graft reads
it back — works. Tempting additions and why they're premature:

- **Schema validation**: useful when many commands produce/consume the same state
  name with incompatible shapes. We have one pair. Wait for the second.
- **Versioning**: useful for rollback. Run logging already captures per-run
  output. State is "latest wins" — fine for now.
- **Namespacing by slice**: the script can write `session-{slice}.json` itself.
  This is the script's concern, not the framework's.

Convention to establish (documentation, not code): state files are JSON objects,
not arrays or bare values. Makes them extensible without breaking consumers.

### 3. The AI step gradient

**Resolution: no special treatment. A workflow step is a command.**

If that command invokes Claude, that's the command's business. The workflow
layer doesn't know or care. The two-layer model is clean precisely because it
draws a hard line: Grove runs deterministic sequences, Claude provides judgment
in an outer loop.

The migration path already handles the gradient:
1. Fully AI-driven: Claude decides what to run (current ad-hoc usage)
2. Crystallized with AI inside: Grove runs a deterministic sequence where one
   step happens to be `claude --prompt "..."` (deterministic sequence, stochastic
   step)
3. Fully deterministic: no AI steps

Approval gates (pause for human review) are orthogonal to AI-vs-deterministic.
They're a future workflow feature that applies to any step.

### 4. Implicit vs explicit dependencies

**Resolution: explicit only. Drop implicit derivation.**

The build-system analogy breaks down because graft's "state" is a mix of JSON
files, working tree changes, and side effects. Inferring that two commands share
state because they both touch the working tree requires modeling what "touching
the working tree" means — which is unbounded.

`reads:`/`writes:` are already implemented and working. Temporal ordering comes
from sequence declarations, not inferred from state overlap.

---

## Dependency Tracking and Cache Invalidation

A follow-up question: does the explicit reads/writes mechanism imply cache
invalidation or staleness tracking?

**The dependency graph itself falls out and is useful.** The `state_name →
producer_command` and `command → required_states` maps enable sequence
validation, observability, and better error messages. Currently the "produced by"
lookup is a linear scan — making it structural is cheap.

**Cache invalidation does not follow and would be harmful.** Graft commands
aren't pure functions — `implement` invokes Claude Code, producing different
outputs from identical inputs. "Staleness" for working-tree-dependent commands is
undefined. Auto-invalidation would push graft toward being a build system, which
is the wrong abstraction.

**Resumability from failure does follow.** When a sequence fails mid-way, the
reads/writes declarations tell you which steps completed (their state files
exist) and which didn't. "Re-run the sequence" can mean "skip steps whose writes
already exist." This uses the state store as a progress marker — not cache
invalidation but resumability.

---

## Proposed Slices

### Slice 1: Sequence Declarations

Add `sequences:` to graft.yaml — a named, ordered list of command references.
`graft run <sequence>` executes linearly, stopping on failure.

### Slice 2: Dependency Graph + Sequence Validation

Compute `state_name → producer` as a first-class structure. Use it to validate
sequences at parse time.

### Slice 3: Sequence Resumability

On re-run, skip steps whose `writes:` state already exists. Uses the run-state
store as progress markers.

### Slice 4: Grove Run-State View

Show run-state in Grove's repo detail: which states exist, which commands are
satisfied, producer/consumer relationships.

---

## Critique

### The argument passing problem (Slice 1)

The current command system has a real argument model: `args:` with `type: choice`,
`options_from:`, `positional: true`. A sequence composing commands inherits this
complexity. `implement` takes `{slice}`, `verify` takes nothing. A three-step
sequence with different argument needs creates combinatorial complexity: union
the args? Name collisions? Different values for the same arg?

Build systems avoid this by not having args. Workflow engines solve it with
explicit per-step bindings. Both are more machinery than we want for the first
version.

### Are sequences even a primitive? (Slice 1)

`graft run implement my-slice && graft run verify` already works. Shell has `&&`.
The value of naming this in graft.yaml is: visibility (in config, not hidden in a
script), resumability (graft knows where it stopped), and Grove observability.

**Alternative: sequences are just commands.** A command's `run:` could be
`"graft run implement {slice} && graft run verify"` — ugly (graft calling graft)
but zero new machinery. The author declares args explicitly, writes composition
in shell.

**Alternative: compound commands.** A command's `run:` is a list of command
references instead of a shell string. No new top-level key. Argument passing is
explicit. Smaller commitment than a full `sequences:` primitive.

### YAGNI concern (Slice 2)

The dependency graph serves slices 1, 3, and 4. With one producer-consumer pair,
the linear scan works fine. A 20-line inline validation function in the sequence
parser would suffice. Build the graph structure when two consumers need it.

### Circular justification (Slices 1 + 3)

Resumability requires sequences (can't skip steps in a manual `&&` chain). But
sequences are partly justified by enabling resumability. Each slice justifies the
other, but neither stands entirely alone.

### The strongest standalone slice (Slice 4)

Grove run-state view requires no new primitives. It shows what's in
`.graft/run-state/` today: which states exist, their contents, which commands
produce and consume them. This gives observability into the system as it works
now, without betting on whether sequences are the right next primitive. And it
may reveal whether sequences are actually needed, or whether the ad-hoc model
plus observability is sufficient.

---

## Decision

Start with **Slice 4 (Grove run-state view)** — it's standalone, useful
immediately, requires no new primitives, and provides observability that informs
whether sequence declarations are worth the complexity.

Write the other three slices as drafts with open questions annotated so they're
ready when we return to them.

---

## Sources

- [Command Output State Mapping](2026-02-23-command-output-state-mapping.md) — predecessor design note
- [Agentic Orchestration](2026-02-18-grove-agentic-orchestration.md) — Grove dispatch board, sessions, plans
- [Unified Process Management](2026-02-19-unified-process-management.md) — ProcessHandle, execution substrate
- `crates/graft-engine/src/command.rs` — setup_run_state, capture_written_state, GRAFT_STATE_DIR
- `crates/graft-engine/src/state.rs` — get_run_state_entry
- `crates/grove-cli/src/tui/repo_detail.rs` — section rendering pattern, DetailItem enum
