# Implementation Orchestration Plan: End-to-End Feature Workflow

## Progress

| Slice | Status | Notes |
|-------|--------|-------|
| 1: verify-captures-state | 🔄 in-progress | |
| 2: sequence-declarations | ⏳ pending | |
| 3: sequence-retry | ⏳ pending | |
| 4: workflow-checkpoints | ⏳ pending | Phase 2 (self-hosting) |
| 5: grove-checkpoint-ui | ⏳ pending | Phase 2 (self-hosting) |

## Pain Points Log

(Filled in as observed during implementation)

---

## Context

Five vertical slices (plans at `slices/*/plan.md`) will wire graft into a complete
self-hosting development cycle. The slices must be implemented in dependency order
so that each one delivers tooling that makes the next slice smoother to build.

This plan describes how ONE master Claude Code session orchestrates the full
multi-slice implementation using graft as the mechanics layer. It also surfaces
architectural issues to reconsider before or during implementation.

---

## Step 0: Create Session Note

Before any implementation begins, create the live tracking file:

```
notes/2026-02-24-implementation-orchestration-plan.md
```

Copy this plan there and add a **Progress** section at the top to track which
slices are in-progress / done / blocked. Update it after each slice. This file
is the single source of truth for the session.

---

## Architectural Alerts (Reconsider Before Coding)

These are real gaps between the plans and the actual codebase, surfaced by
reading the code. They must be resolved during implementation — not assumed away.

### 1. GraftConfig missing `sequences` field (graft-engine/src/domain.rs)

`GraftConfig` (line 644) has `commands` and `state` but **no `sequences` field**.
When `sequence-declarations` adds `SequenceDef` to graft-common, graft-engine's
`GraftConfig` must also gain `sequences: HashMap<String, SequenceDef>`. Otherwise
`execute_sequence()` has nowhere to look up sequence definitions from the engine.

**Files:** `crates/graft-engine/src/domain.rs`

### 2. graft-cli dispatch does NOT use execute_command_by_name (critical)

The plan says "extend `execute_command_by_name` to check `config.sequences`" but
graft-cli's `run_dependency_command()` (line 1904) and `run_current_repo_command()`
(line 1704) have **their own command lookup logic** (`config.commands.get(name)`)
that does NOT route through `execute_command_by_name`. Extending the engine
function alone will NOT make `graft run software-factory:implement-verified` work.

**Both dispatch functions in graft-cli/src/main.rs must be updated** to check
`config.sequences` when the command name is not found in `config.commands`, then
call the new `execute_sequence()` from graft-engine.

**Files:** `crates/graft-cli/src/main.rs` lines 1704 and 1904

### 3. Config loading triplication: three independent paths for sequences

Adding sequences requires updating **all three** config loading layers independently:

| Layer | File | Current | Needs |
|-------|------|---------|-------|
| Parsing | `graft-common/src/config.rs` | `parse_commands_from_str()` | `parse_sequences_from_str()` |
| Execution config | `graft-engine/src/domain.rs` | `GraftConfig { commands, state }` | + `sequences` field |
| Grove display config | `grove-core/src/domain.rs` | `GraftYaml { commands, dependency_names }` | + `sequences` field |
| Grove loader | `grove-engine/src/config.rs` | converts commands only | also convert sequences |

### 4. Sequence→Command conversion in grove is fragile (» prefix stripping)

The plan says sequences are stored as `Command` structs in `available_commands`
with a `» ` display prefix that gets stripped before calling
`execute_command_with_args`. This is fragile: a buggy strip leaves the literal
`» ` in the graft invocation.

**Consider instead**: store `(display_name, graft_name, command)` in
`available_commands`, or add a separate `available_sequences` field in App. The
current plan's approach works but is a known wart — note it in the session note
for future cleanup.

### 5. state_loaded reload mechanism must be verified

The plan says `self.state_loaded = false` reloads run-state after approve/reject.
The App struct has `state_loaded: bool`, but the variable name suggests it may
cover state *queries* not run-state *entries*. Run-state entries have separate
`run_state_entries: Vec<...>` storage. **Before writing the overlay handler**,
read the render loop to confirm `state_loaded = false` actually causes
`load_run_state_entries()` to be re-called on the next frame, or find the correct
invalidation field.

**Files:** `crates/grove-cli/src/tui/mod.rs`, `crates/grove-cli/src/tui/render.rs`

### 6. stop_confirmation dialog breaks the overlay pattern

The stop-confirmation dialog is a `show_stop_confirmation: bool` field handled
INSIDE `handle_key_command_output()`, not as a top-level guard in `handle_key()`.
The approval overlay must follow the `form_input` / `argument_input` pattern
(guard added at the top of `handle_key()`) — NOT the stop-confirmation pattern.

### 7. Producer annotation for sequence-state is special-cased

The plan says grove reads `"sequence"` field from `sequence-state.json` itself
to produce the `(← implement-verified)` annotation. This is special logic in
the run-state rendering path, not from the `writes:` map.

---

## Orchestration Strategy

The five slices split into two phases based on what tooling exists:

### Phase 1 — Bootstrap (Slices 1–3): Manual Orchestration

Use `implement → verify → resume` loop directly, since `implement-verified`
doesn't exist yet. Master Claude Code runs each step via Bash tool.

**Protocol per slice:**
```
1. graft run software-factory:implement slices/<slug>  [timeout: 600s]
2. graft run software-factory:verify                   [timeout: 600s]
3. Read .graft/run-state/verify.json (after slice 1, it exists)
4. If any field is not "OK": resume + re-verify (up to 3 times)
5. cargo test && cargo clippy -- -D warnings && cargo fmt --check
6. git add -p && git commit
```

### Phase 2 — Self-Hosting (Slices 4–5): implement-verified Loop

Use `implement-verified` which handles implement + verify + retry natively.

**Protocol per slice:**
```
1. graft run software-factory:implement-verified slices/<slug>  [timeout: 1800s]
2. Check .graft/run-state/checkpoint.json for phase: awaiting-review
3. Review output/verify.json manually if desired
4. graft run software-factory:approve                 [no args needed]
5. cargo test (quick sanity check)
6. git add -p && git commit
```

---

## Slice Sequence

### Slice 1: verify-captures-state
**Goal:** Make verify output persistent and its exit code meaningful.

### Slice 2: sequence-declarations
**Goal:** Sequences are a first-class primitive in graft and visible in grove.

### Slice 3: sequence-retry
**Goal:** Sequences can declare retry semantics; `implement-verified` is wired.

### Slice 4: workflow-checkpoints
**Goal:** Sequences write a checkpoint gate; approve/reject commands exist.

### Slice 5: grove-checkpoint-ui
**Goal:** Checkpoint entries are visually prominent and actionable in grove.
