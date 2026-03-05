---
status: living
purpose: "Index of exploration notes and session logs"
---

# Notes Index

This directory contains ephemeral session documents - plans, critiques, and implementation summaries that capture decision-making but aren't permanent documentation.

Per [temporal layers policy](../.graft/meta-knowledge-base/docs/policies/temporal-layers.md), notes are:
- **Retention**: Days to weeks
- **Archive when**: Insights extracted or session complete
- **Status**: draft → working → deprecated → archived

For durable documentation, see:
- **Specifications**: [docs/specifications/](../docs/specifications/)
- **Implementation guides**: [docs/](../docs/)
- **Architecture decisions**: [docs/decisions/](../docs/decisions/)

---

## State Queries Implementation (2026-02-13)

Session documents from state queries Stage 1 implementation (Python CLI):

**Deprecated** (implementation complete):
- [Stage 1 Summary](2026-02-13-state-queries-stage1-summary.md) - Consolidated delivery + improvements summary
- [Stage 1 Review](2026-02-13-state-queries-stage1-review.md) - Condensed critique findings

**Canonical source**: [docs/specifications/graft/state-queries.md](../docs/specifications/graft/state-queries.md)

---

## Grove Argument Input (2026-02-13)

Session documents from grove argument input Phase 1:

**Deprecated** (implementation complete):
- [Argument Input Summary](2026-02-13-grove-argument-input-summary.md) - Phase 1 delivery summary
- [Command Arguments Review](2026-02-13-grove-command-arguments-review.md) - Critique + improvement roadmap

**Canonical source**: [docs/specifications/grove/command-execution.md](../docs/specifications/grove/command-execution.md)

---

## Grove Vertical Slices (2026-02-13)

**Deprecated** (review complete):
- [Slices Review](2026-02-13-grove-slices-review.md) - Status assessment + new slice proposals

---

## State Panel Implementation (2026-02-14)

Session documents from state panel Phase 1 implementation:

**Active**:
- [Critique](2026-02-14-state-panel-critique.md) - Analysis of 12 issues, Phase 2/3 roadmap
- [Phase 1 Complete](2026-02-14-state-panel-phase1-complete.md) - Delivery summary (B+ → A-)

**Deprecated** (implementation complete):
- [Phase 1 Plan](2026-02-14-state-panel-phase1-plan.md) - Implementation blueprint (executed)
- [Documentation Review](2026-02-14-documentation-review.md) - Meta-KB compliance assessment

**Archived** (superseded):
- [Grove State Integration Critique](archive/2026-02-13-grove-state-integration-critique.md)
- [State Queries Complete](archive/2026-02-13-state-queries-complete.md)

**Status**: Phase 1 shipped (commits 2f3e159, 83a9dac). Phases 2/3 deferred pending user feedback.

**Canonical sources**:
- **Spec**: [docs/specifications/grove/tui-behavior.md](../docs/specifications/grove/tui-behavior.md#state-panel)
- **Code**: [grove/src/tui.rs](../grove/src/tui.rs)
- **Tests**: [grove/src/tui_tests.rs](../grove/src/tui_tests.rs), [grove/tests/test_state_panel.rs](../grove/tests/test_state_panel.rs)

---

## Graft Rust Rewrite (2026-02-15)

Session documents for rewriting graft in Rust via Ralph loop (autonomous AI agent loop).
All artifacts in [2026-02-15-rust-rewrite/](2026-02-15-rust-rewrite/).

**Active**:
- [Implementation Plan](2026-02-15-rust-rewrite/plan.md) - Spec-driven task list (living, updated by loop)
- [Progress Log](2026-02-15-rust-rewrite/progress.md) - Append-only learnings from each iteration
- [Prompt](2026-02-15-rust-rewrite/prompt.md) - Agent instructions for each iteration
- [Ralph Script](2026-02-15-rust-rewrite/ralph.sh) - Loop runner (`./notes/2026-02-15-rust-rewrite/ralph.sh`)

**Canonical sources**:
- **Specs**: [docs/specifications/graft/](../docs/specifications/graft/)
- **Python reference**: [src/graft/](../src/graft/)
- **Rust crates**: [crates/graft-core/](../crates/graft-core/), [crates/graft-engine/](../crates/graft-engine/), [crates/graft-cli/](../crates/graft-cli/)

---

## Grove Agentic Orchestration (2026-02-18)

Design exploration of how agentic workflow automation fits into grove and graft.

**Active**:
- [Agentic Orchestration](2026-02-18-grove-agentic-orchestration.md) — Sessions, plans, process management, new vertical slices 8-13
- [Command Line and View Stack](2026-02-18-grove-command-prompt-exploration.md) — `:` command line, view stack replacing two-pane layout, dispatch radio metaphor

---

## Command Prompt and View Stack Implementation (2026-02-18)

Ralph loop for evolving the TUI from fixed two-pane layout to view stack with command line.
All artifacts in [2026-02-18-command-prompt-view-stack/](2026-02-18-command-prompt-view-stack/).

**Active**:
- [Implementation Plan](2026-02-18-command-prompt-view-stack/plan.md) - 10 tasks in 3 phases (living, updated by loop)
- [Progress Log](2026-02-18-command-prompt-view-stack/progress.md) - Append-only learnings from each iteration
- [Prompt](2026-02-18-command-prompt-view-stack/prompt.md) - Agent instructions for each iteration
- [Ralph Script](2026-02-18-command-prompt-view-stack/ralph.sh) - Loop runner (`./notes/2026-02-18-command-prompt-view-stack/ralph.sh`)

**Design sources**:
- [Command Line and View Stack](2026-02-18-grove-command-prompt-exploration.md) — design exploration
- [Agentic Orchestration](2026-02-18-grove-agentic-orchestration.md) — dispatch board metaphor

**Canonical sources**:
- **Specs**: [docs/specifications/grove/tui-behavior.md](../docs/specifications/grove/tui-behavior.md), [command-execution.md](../docs/specifications/grove/command-execution.md)
- **Code**: [crates/grove-cli/src/tui/](../crates/grove-cli/src/tui/)

---

## Unified Process Management (2026-02-19)

Design session: consuming graft as a library from grove, with shared observable process management.

**Active**:
- [Unified Process Management](2026-02-19-unified-process-management.md) — ProcessHandle, ProcessRegistry, unified execution model

**Relevant code**:
- `crates/graft-common/` — target location for ProcessHandle and ProcessRegistry
- `crates/graft-engine/src/command.rs`, `crates/graft-engine/src/state.rs` — current execution paths to unify
- `crates/grove-cli/src/tui/command_exec.rs` — current grove subprocess spawning to replace

---

## Command Output State Mapping (2026-02-23)

Design exploration of multi-step graft processes: how commands compose, where
Grove fits, and a new primitive where command outputs are first-class state.

**Active**:
- [Command Output State Mapping](2026-02-23-command-output-state-mapping.md) — Grove/Claude two-layer model, state mapping primitive, build-system analogy, open questions

**Relates to**:
- [Agentic Orchestration](2026-02-18-grove-agentic-orchestration.md) — Grove as dispatch board
- [Unified Process Management](2026-02-19-unified-process-management.md) — execution substrate this sits above

---

## Sequence Primitives and Workflow Slices (2026-02-24)

Resolves the four open questions from the state mapping session. Critiques
proposed vertical slices (sequences, dependency graph, resumability). Decides
to start with Grove run-state observability rather than committing to a
sequence primitive prematurely.

**Active**:
- [Sequence Primitives Exploration](2026-02-24-sequence-primitives-exploration.md) — open question resolutions, slice proposals, critique, decision

**Relates to**:
- [Command Output State Mapping](2026-02-23-command-output-state-mapping.md) — predecessor (this session resolves its open questions)
- [Agentic Orchestration](2026-02-18-grove-agentic-orchestration.md) — dispatch board, sessions

---

## Graft as Context Provider (2026-02-28)

Exploration of inverting the architecture: Claude instances as autonomous workers,
graft as a queryable world model, grove as mission control with local branch/merge
review.

**Active**:
- [Graft as Context Provider](2026-02-28-graft-as-context-provider.md) -- worker model,
  artifacts over actors, local PR workflow, component evolution

**Relates to**:
- [Agentic Orchestration](2026-02-18-grove-agentic-orchestration.md) -- dispatch board metaphor
- [Command Output State Mapping](2026-02-23-command-output-state-mapping.md) -- state mapping primitive
- [Sequence Primitives](2026-02-24-sequence-primitives-exploration.md) -- sequence design decisions
- [Entity Focus Slice](../slices/grove-entity-focus/plan.md) -- step zero of the evolution

---

## Scion Lifecycle Design (2026-03-01)

Design session refining the worker model from the context provider exploration.
Establishes scion/fuse/prune vocabulary, composable lifecycle hooks in graft.yaml,
and minimal Claude Code integration surface.

**Active**:
- [Scion Lifecycle Design](2026-03-01-shoot-lifecycle-design.md) — scion commands,
  composable hooks, failure semantics, Claude Code integration layers

**Relates to**:
- [Graft as Context Provider](2026-02-28-graft-as-context-provider.md) — parent
  exploration
- [Agentic Orchestration](2026-02-18-grove-agentic-orchestration.md) — dispatch
  board metaphor
- [Sequence Primitives](2026-02-24-sequence-primitives-exploration.md) — sequence
  design decisions

---

## Scion Orchestration Design (2026-03-01)

Architecture for scion worker management: graft owns a runtime abstraction
(tmux today, docker/SSH future), grove is a switchboard for observation and
review. Revisits agentic orchestration proposals (Slices 8-13) in light of
implemented scions.

**Active**:
- [Scion Orchestration Design](2026-03-01-scion-orchestration-design.md) —
  runtime abstraction, grove switchboard, worker handoff, prompt assembly

**Relates to**:
- [Scion Lifecycle Design](2026-03-01-shoot-lifecycle-design.md) — implemented
  primitives this builds on
- [Graft as Context Provider](2026-02-28-graft-as-context-provider.md) —
  worker model, artifacts over actors
- [Agentic Orchestration](2026-02-18-grove-agentic-orchestration.md) — original
  Slices 8-13 proposals

---

## Checkpoint Analysis (2026-03-04)

Analysis of checkpoint.json: ownership split, use case exploration, four candidate
models for human-in-the-loop gates, and resolution that scions-as-branches ARE the
checkpoint mechanism. Fuse is approve. Checkpoint.json is redundant with structural
git state and should be removed.

**Active**:
- [Checkpoint Analysis](2026-03-04-checkpoint-analysis.md) — full analysis,
  model comparison, scions-as-checkpoints resolution

**Relates to**:
- [Sequence Primitives](2026-02-24-sequence-primitives-exploration.md) — sequence
  design decisions
- [Scion Orchestration Design](2026-03-01-scion-orchestration-design.md) — parallel
  workers, grove as switchboard
- [Pain Point Strategy](2026-03-04-pain-point-strategy.md) — checkpoint UI deferred

---

## Adding New Notes

When creating session logs or exploration notes:

1. **Use date prefix**: `YYYY-MM-DD-descriptive-name.md`
2. **Add frontmatter** with status and purpose
3. **Update this index**: Add entry to relevant section
4. **Link to sources**: Add `## Sources` section citing code/specs
5. **Mark deprecated when done**: Change status with `archived-reason`

**Archive policy**: Move to `archive/` when session complete and insights extracted.
