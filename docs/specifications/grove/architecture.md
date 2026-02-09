---
title: "Grove Architecture"
status: draft
date: 2026-02-08
---

# Grove Architecture

## Overview

Grove is a workspace management tool that helps developers work with multiple git repositories as a unified workspace, with graft-awareness as a feature layer. Think of it as a **departure board** — it shows you where to go, not the destination itself.

Grove provides:

- Multi-repo status and navigation
- Quick capture to inbox with routing
- Cross-repo search
- Graft-aware dependency visualization
- Command execution with rich output
- Agentic integration via CLI and structured output

## Core Concepts

### Workspace

A **workspace** is a named collection of git repositories that you work with together. It is Grove's core organizing primitive — declared in `workspace.yaml`, it defines which repos are in scope and basic settings like capture routing and search exclusions.

See: [Workspace Configuration](./workspace-config.md)

### Departure Board

Grove's mental model is a departure board at a train station: glance at the board, see what needs attention, go there. You don't live in the station — you pass through it. Grove surfaces status and signals, then gets out of the way by handing off to `$EDITOR`, a shell, or whatever tool fits the task.

### Graft Awareness

Grove doesn't require graft — it's useful for plain git repos. When `graft.yaml` exists in a repo, Grove reads it to display dependencies and their status. This is **loose coupling**: Grove reads graft's data files but doesn't depend on graft as a library. Mutations (upgrade, apply) delegate to the `graft` CLI.

## Three-Layer Architecture

```
┌─────────────────────────────────────────────────────────┐
│  Layer 3: User Interfaces                                │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐               │
│  │   TUI    │  │  Native  │  │   Web    │               │
│  │          │  │  (macOS)  │  │          │               │
│  └────┬─────┘  └────┬─────┘  └────┬─────┘               │
│       └──────────────┴─────────────┘                     │
│                      │                                    │
├──────────────────────┼────────────────────────────────────┤
│  Layer 2: Workspace Engine                                │
│  ┌────────────────────────────────────────────────────┐   │
│  │  Multi-repo registry · Search index · File watcher │   │
│  │  Capture routing · Command dispatch · Git ops      │   │
│  │  Graft metadata reader                             │   │
│  └────────────────────────┬───────────────────────────┘   │
│                           │                               │
├───────────────────────────┼───────────────────────────────┤
│  Layer 1: Foundation Libraries                            │
│  ┌─────────┐  ┌──────────┐  ┌──────────┐                 │
│  │  Git    │  │  Search  │  │ FS Watch │                 │
│  └─────────┘  └──────────┘  └──────────┘                 │
└───────────────────────────────────────────────────────────┘
```

**Layer 1: Foundation Libraries** — Existing libraries for git operations, full-text search, and file system watching. These are implementation choices, not architectural commitments.

**Layer 2: Workspace Engine** — The core logic we build:
- Multi-repo registry (tracks repositories in workspace)
- Search index (maintains cross-repo search capability)
- File watcher (detects changes across workspace)
- Capture routing (directs quick captures to correct locations)
- Command dispatch (executes commands in repo contexts)
- Git operations (status, commit, push)
- Graft metadata reader (parses graft.yaml/graft.lock when present)

**Layer 3: User Interfaces** — Interchangeable UIs consuming the engine:
- **TUI** — Terminal interface, primary focus
- **Native** (macOS/iOS) — Native Apple apps, future
- **Web** — Browser-based interface, future

The workspace engine is a **library** that UIs link against directly. All business logic lives in the engine; UIs only handle display and user interaction. Whether the engine maintains state between invocations or rebuilds on launch is an open question.

### CLI and Agentic Integration

Every core operation has both a TUI interface and a CLI/machine-readable interface. This enables scripting, agent integration, and testing without UI. Structured output (`--json`) is available for machine consumption.

## Design Principles

### 1. Git-native

Grove works with any git repository. Graft awareness is additive — repos without `graft.yaml` are first-class.

### 2. Engine-UI separation

The engine is a library that UIs consume. No business logic in the UI layer. This enables multiple frontends and makes the engine independently testable.

### 3. CLI-first for every feature

Every engine capability is exposed through CLI with structured output. The TUI is one consumer of the engine, not the only way in.

### 4. No telemetry or phoning home

Grove is a local-only tool. It does not phone home, collect telemetry, or make network requests beyond what git operations require (fetch, push, pull).

### 5. Simple distribution

Minimal installation ceremony. Self-contained with embedded search index, no external database.

## Open Questions

- [ ] Should the engine maintain state between invocations (daemon mode) or rebuild on launch?
- [ ] Should search indexing be incremental (file watcher) or full rebuild?
- [ ] Should MCP server support be built-in or a separate wrapper?
- [ ] How should the engine handle concurrent operations (multiple UIs)?

## Sources

- [Workspace UI Exploration (2026-02-06)](../../../notes/2026-02-06-workspace-ui-exploration.md) — Original architecture design, three-layer model, workspace concept
- [Grove Workflow Hub Primitives (2026-02-07)](../../../notes/2026-02-07-grove-workflow-hub-primitives.md) — CLI integration, agentic workflows, engine-UI separation
- [Grove Vertical Slices (2026-02-06)](../../../notes/2026-02-06-grove-vertical-slices.md) — Implementation roadmap
