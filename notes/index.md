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

## Adding New Notes

When creating session logs or exploration notes:

1. **Use date prefix**: `YYYY-MM-DD-descriptive-name.md`
2. **Add frontmatter** with status and purpose
3. **Update this index**: Add entry to relevant section
4. **Link to sources**: Add `## Sources` section citing code/specs
5. **Mark deprecated when done**: Change status with `archived-reason`

**Archive policy**: Move to `archive/` when session complete and insights extracted.
