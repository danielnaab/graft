---
status: working
last-updated: 2026-02-10
---

# Grove Implementation Roadmap

## Slice Status

| # | Slice | Status | Target | Blockers |
|---|-------|--------|--------|----------|
| 1 | Workspace Config + Repo List TUI | completed | 2026-02-10 | - |
| 2 | Repo Detail Pane | not-started | TBD | Slice 1 |
| 3 | Quick Capture | not-started | TBD | Slice 1 |
| 4 | File Navigation + $EDITOR | not-started | TBD | Slice 2 |
| 5 | Graft Metadata Display | not-started | TBD | Slice 2 |
| 6 | Cross-Repo Search | not-started | TBD | Slice 1 |
| 7 | Command Execution | not-started | TBD | Slice 2 |

## Overview

Grove is a multi-repo workspace manager with graft awareness, built incrementally through vertical slices. Each slice delivers end-to-end functionality that can be used immediately.

## Current Focus: Slice 1

**Goal:** Launch grove, see list of configured repositories with git status, navigate with j/k, quit with q.

**Deliverables:**
- YAML workspace config loading
- Git status querying (branch, clean/dirty, ahead/behind)
- Terminal UI with repository list
- Vim-style navigation

## Sources

- [Grove Vertical Slices](../../../../notes/2026-02-06-grove-vertical-slices.md)
- [Architecture Spec](../../../../docs/specifications/grove/architecture.md)
- [Workspace Config Spec](../../../../docs/specifications/grove/workspace-config.md)
