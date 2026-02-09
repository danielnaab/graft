---
title: "Exploration: Workspace User Interface for Graft"
date: 2026-02-06
status: working
participants: ["human", "agent"]
tags: [exploration, ui, workspace, tui, rust, multi-repo, capture, search]
---

# Exploration: Workspace User Interface for Graft

## Context

This document explores a **workspace user interface** for Graft — a tool that helps interact with content and workflows across multiple git repositories. It should provide:

- **Multi-repo awareness**: See and operate across multiple git repos as a unified workspace
- **Quick capture**: Frictionless note/idea capture into git-backed repos (inbox-style)
- **Command execution**: Run graft commands and other repo commands with rich output
- **Search**: Fast full-text search across all workspace repositories
- **Graft awareness**: Understand graft.yaml/graft.lock metadata when present

**User direction**: Start with a fast TUI, with native macOS/iOS and web interfaces as future targets. Capture is primarily quick notes/inbox style.

**This is exploratory** — no specification changes should result from this document until ideas are validated and refined.

**Graduated to specifications:**
- [Grove Architecture](../docs/specifications/grove/architecture.md) - System design, three-layer architecture, workspace concept
- [Workspace Configuration](../docs/specifications/grove/workspace-config.md) - workspace.yaml format (partial)

---

## 1. Key Architectural Insight

**The workspace UI is not primarily a "graft dashboard."** It's a **developer workspace tool** that is graft-aware. Most of its value — multi-repo navigation, capture, search, git operations — exists independently of graft. Graft awareness is a feature (understanding dependencies, changes, commands) layered on top.

This is an important framing because it means:
1. The tool is useful even for repos without graft.yaml
2. Graft integration reads existing graft config files — it doesn't need deep coupling to a graft library
3. The architecture is simpler: a workspace tool that knows how to read graft metadata

### Three-Layer Architecture

```
┌─────────────────────────────────────────────────────────┐
│  Layer 3: User Interfaces                                │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐               │
│  │   TUI    │  │  macOS   │  │   Web    │               │
│  │(ratatui) │  │ (SwiftUI)│  │          │               │
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
│  │gitoxide │  │ tantivy  │  │ notify   │                 │
│  │ (git)   │  │ (search) │  │(fs watch)│                 │
│  └─────────┘  └──────────┘  └──────────┘                 │
└───────────────────────────────────────────────────────────┘
```

**Layer 1** is existing Rust libraries for git, search, and file watching.

**Layer 2** is the workspace engine — the core logic that aggregates repos, manages the search index, handles capture routing, and reads graft metadata. This is the main thing we'd build. It's a library, not a server or daemon.

**Layer 3** is interchangeable UIs. The TUI is first. Native macOS/iOS and web come later, consuming the same workspace engine via UniFFI (for Swift) or HTTP/WebSocket (for web).

---

## 2. The Workspace Concept

A **workspace** is a named collection of git repositories that you work with together.

```yaml
# ~/.config/grove/workspace.yaml (or similar)
name: "my-project"
repositories:
  - path: ~/src/graft-knowledge
  - path: ~/src/meta-knowledge-base
  - path: ~/src/my-app

capture:
  inbox: ~/src/graft-knowledge/notes/inbox/
  template: |
    ---
    date: {{date}}
    ---
    {{content}}
  auto_commit: true

search:
  exclude: ["node_modules", ".git", "vendor", "target"]
```

The workspace config is minimal — it knows which repos are in scope, where captures go, and search preferences. Everything else is derived from the repos themselves (graft.yaml if present, git state, file contents).

### Workspace Operations

| Operation | Description |
|-----------|-------------|
| **Status** | Aggregate git status across all repos (clean/dirty/ahead/behind) |
| **Search** | Full-text search across all repo content, with repo/type faceting |
| **Capture** | Create a timestamped note in the inbox, optionally routed by prefix |
| **Navigate** | Browse repos, files, git history, diffs |
| **Execute** | Run graft commands or arbitrary commands in a repo context |
| **Dependencies** | Show graft dependency state, available upgrades (when graft.yaml exists) |

---

## 3. TUI Design

### Layout Concept

```
┌─ Repos ─────────────┬─ graft-knowledge ────────────────────────┐
│ ● graft-knowledge   │ main · clean · 3 commits ahead           │
│   meta-knowledge-b… │                                          │
│   my-app            │ Recent activity:                          │
│                     │   2h  Updated flat-only specs             │
│                     │   1d  Added decision 0007                 │
│                     │   3d  Fix HIGH priority issues            │
│                     │                                          │
│                     │ Graft deps:                               │
│                     │   meta-knowledge-base  main (up to date)  │
│                     │                                          │
│                     │ Commands:                                  │
│                     │   (none defined)                          │
├─────────────────────┴──────────────────────────────────────────┤
│ > capture: _                                                    │
│                                                    [?] help     │
└─────────────────────────────────────────────────────────────────┘
```

### Key Interactions

| Key | Action |
|-----|--------|
| `j/k` | Navigate repos or items |
| `Enter` | Select / expand |
| `/` | Search across workspace |
| `c` | Quick capture (opens inline editor) |
| `r` | Run command in selected repo |
| `s` | Git status detail |
| `d` | Dependencies view (graft) |
| `g` | Git operations (commit, push, pull) |
| `e` | Open in $EDITOR |
| `q` | Quit |
| `?` | Help |

### Capture Flow

1. Press `c` → bottom bar becomes a text input
2. Type your note (single line for quick, `Ctrl+E` to open $EDITOR for multi-line)
3. Press `Enter` → note saved as `notes/inbox/2026-02-06T14-30-00-thought.md` in the configured inbox repo
4. If `auto_commit: true`, a commit is created: `capture: <first line of note>`
5. Optional: prefix with `@repo-name:path/` to route to a specific location

### Search Flow

1. Press `/` → search input appears with live results
2. Type query → results update in real-time (ripgrep-speed via tantivy or direct file search)
3. Results grouped by repo, showing file path and matching line
4. `j/k` to navigate results, `Enter` to open in $EDITOR
5. `Tab` to toggle filters (repo, file type, date range)

### Command Execution Flow

1. Press `r` → shows available commands for selected repo
2. For graft-aware repos: shows graft commands from graft.yaml
3. For any repo: shows configured workspace commands or recent shell history
4. Select command → execution streams output in a pane
5. Output persisted for later review

---

## 4. Implementation: Why Rust

Given TUI-first with native Apple as future target, Rust is the strongest choice:

| Concern | Rust Solution | Quality |
|---------|---------------|---------|
| **TUI framework** | ratatui | Excellent — flexible, performant, huge community |
| **Git operations** | gitoxide | Excellent — pure Rust, protocol v2, no C deps |
| **Full-text search** | tantivy | Excellent — Lucene-class, embeddable |
| **File watching** | notify crate | Good — cross-platform fs events |
| **macOS/iOS bridge** | UniFFI (Mozilla) | Good — auto-generates Swift bindings |
| **Web target** | WASM compilation | Good — core logic reusable in browser |
| **Performance** | Native compilation | Excellent |
| **Distribution** | Single binary | Excellent |

The Rust stack provides a coherent path: **TUI now → native Apple via UniFFI → web via WASM**, all from one core codebase.

**Alternative considered**: Go + Bubble Tea. Faster initial development, excellent TUI (Charm ecosystem). But the path to native Apple is harder (cgo + complex FFI), and Go lacks equivalents to gitoxide and tantivy. If we were only building a TUI and never going native, Go would be competitive.

---

## 5. What Graft Core Should Provide

Looking at this from the workspace tool's perspective, most of what it needs is **reading graft metadata files** (graft.yaml, graft.lock) — not calling a graft library. The workspace tool can:

1. Parse graft.yaml to discover dependencies and commands (simple YAML reading)
2. Parse graft.lock to understand dependency state (simple YAML reading)
3. Shell out to `graft` CLI for mutations (upgrade, apply) when a graft implementation exists
4. Read .gitmodules to understand submodule state

This means the workspace tool has **minimal coupling to graft** — it reads graft's data files and optionally delegates to the graft CLI. This is the right separation of concerns.

However, as graft evolves, there are primitives that would benefit both the CLI and workspace tool if formalized:

### Near-term (read graft files directly)
- Parse graft.yaml for dependencies, commands, changes
- Parse graft.lock for dependency state
- These are just YAML files — any language can read them

### Medium-term (graft provides structured output)
- `graft status --format json` → machine-readable dependency state
- `graft changes --format json` → structured change listings
- `graft upgrade --dry-run --format json` → structured upgrade plan
- This enables the workspace tool to show rich graft information without reimplementing graft logic

### Longer-term (shared library)
- If graft is implemented in Rust, the workspace tool could depend on graft-core as a library
- Shared parsing, validation, and query logic
- This only makes sense if both tools are in the same language

**Recommendation**: Start by reading graft YAML files directly. This keeps the workspace tool independent and immediately useful. Add structured CLI output integration when graft CLI exists.

---

## 6. Naming

The workspace tool needs a name. Candidates in the grafting/horticulture metaphor:

| Name | Meaning | Fit |
|------|---------|-----|
| **grove** | A small group of trees | Excellent — a workspace is a grove of repos |
| **nursery** | Where plants are tended | Good — but implies immaturity |
| **garden** | Cultivated space | Good — but generic |
| **plot** | A piece of ground for growing | Decent — also evokes "plotting" a course |
| **canopy** | The upper layer of a forest | Good — the UI is the canopy over the repos |
| **arbor** | A garden structure | Decent |

**"Grove"** feels strongest: a grove is a coherent collection of trees (repos) that grow together. It's short, memorable, and extends the graft metaphor naturally. You graft branches; your grove is where your grafted trees live.

---

## 7. Project Scope and Phasing

### Phase 1: Foundation (MVP)
Build a Rust TUI that provides multi-repo navigation and quick capture.

**Deliverables:**
- Workspace config file parsing
- Multi-repo status view (git status across repos)
- Quick capture to inbox (create timestamped markdown, auto-commit)
- Basic file/repo navigation
- Open in $EDITOR integration

**What this validates:** Is the workspace concept useful? Is the capture flow right? Is the TUI responsive enough?

### Phase 2: Search and Graft Awareness
Add search and graft metadata integration.

**Deliverables:**
- Full-text search across workspace repos (tantivy index)
- Graft.yaml/graft.lock parsing and display
- Dependency status view
- Available upgrades display
- Command listing from graft.yaml

### Phase 3: Command Execution
Add the ability to run commands from the TUI.

**Deliverables:**
- Execute graft commands with streaming output
- Execute arbitrary shell commands in repo context
- Execution history
- Output persistence and review

### Phase 4: Native Apple UI
Build SwiftUI interfaces consuming the workspace engine via UniFFI.

**Deliverables:**
- macOS app with menu bar quick-capture
- iOS app with share sheet integration
- Spotlight integration for search
- Widgets for repo status

### Phase 5: Web Interface
Add browser-based access.

**Deliverables:**
- Local web server exposing workspace API
- React/Svelte frontend
- Same functionality as TUI in browser

---

## 8. Relationship to Previous Brainstorming

This exploration builds on and refines earlier brainstorming:

| Previous Concept | Status in This Exploration |
|-----------------|---------------------------|
| UI Architecture brainstorming (2026-01-07) | Partially incorporated — the six qualities (Observability, Connectivity, Actionability, Context Richness, Governance, Composability) remain valid but we're starting much simpler |
| Execution Records (brainstorming 3.1) | Deferred to Phase 3 — command execution history |
| Linking Infrastructure (brainstorming 3.2) | Deferred — nice to have, not essential for MVP |
| Structured Output (brainstorming 3.3) | Important for Phase 2-3 — graft CLI should output structured data |
| Policy Specification (brainstorming 3.4) | Deferred — governance comes after basic functionality |
| Event Emission (brainstorming 3.5) | Partially covered by file watching for now |
| Schema Registry (brainstorming 3.6) | Deferred — useful for domain-specific UIs later |
| Ecosystem repos (brainstorming Part 4) | Simplified — we're building one tool, not 15 repos |
| Multiple Interfaces (evolution Theme 5) | Validated — TUI → native → web progression |

**Key simplification**: The earlier brainstorming envisioned a full enterprise platform (graft-api, graft-executor, graft-policy-engine, graft-audit, etc.). This exploration focuses on a **personal developer tool** that does a few things exceptionally well: navigate repos, capture thoughts, search content, understand graft state.

---

## 9. Open Questions

1. **Workspace discovery**: Should repos be explicitly listed in config, or should the tool scan a directory tree? (e.g., "all git repos under ~/src/")
2. **Capture format**: Plain markdown? Frontmatter YAML + markdown? Something else?
3. **Search scope**: Index everything, or just markdown/yaml/common text files? Binary file handling?
4. **Graft CLI integration**: Shell out to graft for mutations, or reimplement graft operations?
5. **Daemon vs on-demand**: Should the search index be maintained by a background daemon, or rebuilt on launch?
6. **Project home**: New standalone repository (grove/graft-grove), or within the graft-knowledge ecosystem?

---

## Sources

- [Graft Architecture](../docs/specifications/architecture.md)
- [UI Architecture Brainstorming (2026-01-07)](./2026-01-07-ui-architecture-brainstorming.md)
- [Evolution Brainstorming (2026-01-05)](./2026-01-05-evolution-brainstorming.md)
- [ratatui](https://ratatui.rs/) — Rust TUI framework
- [gitoxide](https://github.com/GitoxideLabs/gitoxide) — Pure Rust git implementation
- [tantivy](https://github.com/quickwit-oss/tantivy) — Rust full-text search
- [UniFFI](https://mozilla.github.io/uniffi-rs/) — Mozilla's Rust↔Swift/Kotlin binding generator
- [lazygit](https://github.com/jesseduffield/lazygit) — TUI git interface (patterns reference)
