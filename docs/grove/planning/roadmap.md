---
status: working
last-updated: 2026-02-10
---

# Grove Implementation Roadmap

## Slice Status

| # | Slice | Status | Target | Blockers |
|---|-------|--------|--------|----------|
| 1 | Workspace Config + Repo List TUI | completed | 2026-02-10 | - |
| 2 | Repo Detail Pane | completed | 2026-02-10 | Slice 1 |
| 3 | Quick Capture | not-started | TBD | Slice 1 |
| 4 | File Navigation + $EDITOR | not-started | TBD | Slice 2 |
| 5 | Graft Metadata Display | not-started | TBD | Slice 2 |
| 6 | Cross-Repo Search | not-started | TBD | Slice 1 |
| 7 | Command Execution | not-started | TBD | Slice 2 |

## Overview

Grove is a multi-repo workspace manager with graft awareness, built incrementally through vertical slices. Each slice delivers end-to-end functionality that can be used immediately.

## Completed Slices

### Slice 1: Workspace Config + Repo List TUI

**Goal:** Launch grove, see list of configured repositories with git status, navigate with j/k, quit with q.

**Deliverables:**
- YAML workspace config loading
- Git status querying (branch, clean/dirty, ahead/behind)
- Terminal UI with repository list
- Vim-style navigation
- Living specification: [tui-behavior.md](../../../specifications/grove/tui-behavior.md)

### Slice 2: Repo Detail Pane

**Goal:** Select a repository and view detail (branch header, changed files, recent commits) in a split-pane layout.

**Deliverables:**
- 40/60 horizontal split-pane layout
- Focus management (Enter/Tab to detail, q/Esc back to list)
- Changed files display with status indicators
- Recent commits display with author and date
- Detail scroll with clamping
- Detail caching by selection index
- Living specification: [tui-behavior.md](../../../specifications/grove/tui-behavior.md)

## Next Up: Slice 3

**Quick Capture** — Allow users to quickly capture notes with prefix-based routing to different repositories.

**Goal:** `grove capture "@finances lunch $15"` routes to finances repo's notes directory.

**Dependencies:** Slice 1 (workspace config with capture routing is already specified in workspace-config.md)

**Specification:** See [workspace-config.md](../../../specifications/grove/workspace-config.md) Capture Configuration section ([Slice 3] scenarios) for full behavior specification.

## Sources

- [Grove Vertical Slices](../../../notes/2026-02-06-grove-vertical-slices.md)
- [Architecture Spec](../../../specifications/grove/architecture.md)
- [Workspace Config Spec](../../../specifications/grove/workspace-config.md)
- [TUI Behavior Spec](../../../specifications/grove/tui-behavior.md)
