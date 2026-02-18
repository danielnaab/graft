# Grove Specifications

Living specifications for the Grove workspace management tool.

## What is Grove?

Grove is a workspace management tool for working with multiple git repositories as a unified workspace. It provides multi-repo status, quick capture with routing, cross-repo search, graft-aware dependency visualization, and agentic integration via CLI.

## Reading Guide

Specifications follow the [living-specifications](../../../.graft/living-specifications/) methodology. Each spec has: Intent, Non-goals, Behavior (Given/When/Then scenarios with edge cases), Constraints, Open Questions, Decisions, and Sources. Behavior headings are tagged with `[Slice N]` to connect to the [vertical slices](../../../notes/2026-02-06-grove-vertical-slices.md) implementation plan.

## Status Lifecycle

- **draft** — Initial design, not yet validated
- **working** — Being implemented, verified against code
- **stable** — Implemented and verified, trusted reference
- **deprecated** — No longer applicable

## Specifications

- [**Workspace Configuration**](./workspace-config.md) — workspace.yaml format, repo declarations, tags, capture routing, status scripts
- [**TUI Behavior**](./tui-behavior.md) — Repo list display, view stack navigation, full-width Dashboard and RepoDetail views, command line (`:` key)
- [**Command Execution**](./command-execution.md) — Discovering and running commands from graft.yaml

## Design Documents

- [**Architecture**](./architecture.md) — System design, three-layer architecture, core concepts, design principles

## Related Documentation

- [Workspace UI Exploration](../../../notes/2026-02-06-workspace-ui-exploration.md) — Original architecture design
- [Grove Vertical Slices](../../../notes/2026-02-06-grove-vertical-slices.md) — Implementation roadmap
- [Grove Workflow Hub Primitives](../../../notes/2026-02-07-grove-workflow-hub-primitives.md) — Core design primitives
- [Status Check Syntax Exploration](../../../notes/2026-02-08-status-check-syntax-exploration.md) — Status script design
- [Living Specifications Guide](../../../.graft/living-specifications/docs/guides/writing-specs.md) — How to write and maintain specs
