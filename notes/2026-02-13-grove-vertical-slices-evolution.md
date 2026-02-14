---
title: "Grove Vertical Slices - Evolution & Future Vision"
date: 2026-02-13
status: planning
participants: ["human", "agent"]
tags: [planning, grove, vertical-slices, architecture, vision]
---

# Grove Vertical Slices - Evolution & Future Vision

## Context

This document reviews the [original vertical slices plan](./2026-02-06-grove-vertical-slices.md) from February 6, 2026, assesses current implementation status, reconsiders unimplemented slices, and proposes new slices based on evolved understanding of Grove's potential.

**Key Evolution**: Grove has emerged as a **workspace orchestration hub** with excellent command execution UX, not just a passive repository viewer. This shifts our priorities toward operational capabilities.

---

## Current State Review

### ✅ Fully Implemented Slices (3/7)

#### Slice 1: Workspace Config + Repo List TUI
**Status**: 100% complete, production-ready

**What works exceptionally well**:
- Robust workspace.yaml parsing with validation
- Fast git status via gitoxide + subprocess hybrid
- Excellent compact path formatting (fish-style abbreviation)
- Comprehensive error handling with graceful degradation
- Empty workspace guidance
- Status indicators: branch, dirty (●/○), ahead (↑n), behind (↓n)

**Beyond original scope**:
- 5-second configurable timeout for git operations
- Tilde expansion and environment variables in paths
- Adaptive display for narrow terminals
- Comprehensive test coverage

#### Slice 2: Repo Detail Pane
**Status**: 100% complete, production-ready

**What works exceptionally well**:
- Clean 40/60 split-pane layout
- Recent commits (last 10) with hash, subject, author, relative date
- Changed files with status indicators (M/A/D/R/C/?)
- Scrollable detail pane (j/k navigation)
- Detail caching to avoid redundant queries
- Partial data display on errors (resilient)

#### Slice 7: Command Execution
**Status**: 100% complete + Phase 1 UX enhancements

**What works exceptionally well**:
- graft.yaml command discovery and parsing
- Command picker overlay (j/k nav, Enter to select)
- **Argument input dialog** (2026-02-13 Phase 1):
  - Cursor navigation (←→, Home, End)
  - Character insertion at cursor position
  - Visual cursor indicator (▊ in middle, _ at end)
  - **Real-time command preview** with parsed arguments
  - Shell-style parsing (respects quotes via shell-words)
  - **Parse validation** blocking execution on errors
  - Error feedback in status bar
- Smart graft discovery (uv-managed or system PATH)
- Streaming output with j/k scrolling
- Stop confirmation dialog with SIGTERM
- Output buffer limiting (10K lines max)
- Completion status (✓ success / ✗ failure)
- Comprehensive test coverage (15 tests)

**Key insight**: Command execution with arguments is now **best-in-class** for TUI command runners. This is a core competency to build on.

---

### ⚠️ Partially Implemented Slices (1/7)

#### Slice 5: Graft Metadata Display
**Status**: ~30% complete (parsing only, no UI)

**What works**:
- graft.yaml can be loaded and parsed
- Commands section extraction (used in Slice 7)
- GraftYaml domain model exists

**What's missing** (the entire point of Slice 5):
- Dependencies view in detail pane
- .gitmodules parsing
- Dependency sync status indicators
- No 'd' keybinding
- No visual display of dependencies

**Recommendation**: This is **high priority to complete** — graft integration is a core differentiator.

---

### ❌ Not Implemented Slices (3/7)

#### Slice 3: Quick Capture
**Status**: 0% complete

**Original scope**:
- Press 'c', type note, save to inbox with auto-commit
- Timestamp-based markdown files
- Modal capture mode

**Why it hasn't been implemented**:
- Command execution took priority
- Unclear if bottom-bar input is right UX (vs argument dialog pattern)

**Recommendation**: **Reconsider approach** — use argument dialog pattern from Slice 7 instead of bottom-bar input. Makes capture consistent with command UX.

#### Slice 4: File Navigation + $EDITOR
**Status**: 0% complete

**Original scope**:
- File tree in detail pane
- Directory traversal
- Press Enter to open in $EDITOR
- gitignore filtering

**Why it hasn't been implemented**:
- Command execution and argument input took priority
- File navigation is lower value than commands

**Recommendation**: **Medium priority** — valuable workflow enhancement, but not blocking.

#### Slice 6: Cross-Repo Search
**Status**: 0% complete

**Original scope**:
- Press '/', type query
- Tantivy indexing
- Results overlay with navigation

**Why it hasn't been implemented**:
- Complex (indexing, tantivy integration)
- Unclear ROI vs ripgrep in shell

**Recommendation**: **Reconsider approach** — start with streaming ripgrep instead of tantivy. Simpler, faster to ship.

---

## Reconsidering Unimplemented Slices

### Slice 3: Quick Capture → **Redesign Required**

**Original design issues**:
- Bottom-bar input doesn't match current UX patterns
- Unclear how it relates to command execution
- Limited to single-line capture

**New design proposal** (leverages Slice 7 architecture):
```
Press 'c' → Capture dialog appears (similar to argument input)
  > Capture to: [inbox▊]     ← Tab-completable destination
  > Template: [quick▊]         ← Optional template selection
  > Content: _                 ← Multi-line input (Ctrl+E → $EDITOR)

Preview: Will create: inbox/2026-02-13T15-30-quick-note.md

Enter → Creates file, auto-commits, shows confirmation
Esc → Cancel
```

**Benefits of new design**:
- Consistent with argument input UX (cursor nav, preview)
- Supports multi-line via $EDITOR integration
- Template selection enables structured capture (meetings, todos, ideas)
- Routing to different destinations (@repo:path)
- Preview shows what will be created

**Priority**: **High** — capture is core to workflow hub vision

---

### Slice 4: File Navigation → **Defer, Replace with Link Following**

**Original design issues**:
- File tree in detail pane competes with commits/files/dependencies views
- File navigation available via shell/editor already
- Unclear unique value in Grove

**Alternative proposal**: Replace with **Link Following** (new slice)
- Instead of generic file nav, focus on *graft-aware navigation*
- Jump to dependency source in workspace
- Follow references in markdown files
- Navigate between related repos

**Priority**: **Low to Medium** — file tree is "nice to have", link following is unique value

---

### Slice 6: Cross-Repo Search → **Simplify to Streaming Ripgrep**

**Original design issues**:
- Tantivy adds complexity (indexing, maintenance, storage)
- Indexing on launch delays startup
- Unclear advantage over ripgrep in shell for most use cases

**Simplified proposal**: **Streaming Ripgrep Search**
```
Press '/' → Search overlay appears
  > Query: [function▊]
  > Scope: [All repos ▼]     ← Dropdown: all / current / tagged

Results stream in real-time as ripgrep executes:
  graft/src/main.rs:42: function main() {
  grove/src/tui.rs:156: function handle_key(&mut self) {

j/k navigate, Enter → open in $EDITOR at line
```

**Benefits of simplified design**:
- No indexing = instant startup
- Leverages proven ripgrep
- Real-time streaming results (no wait for index build)
- Simpler implementation (spawn ripgrep, stream output)
- Can add tantivy later if ripgrep proves insufficient

**Priority**: **Medium** — valuable for large workspaces, but not blocking

---

## New Slice Proposals

Based on evolved understanding of Grove as a **workspace orchestration hub**, here are new slice ideas:

---

### 🆕 Slice 8: Workspace Health Dashboard

**User story**: "What needs attention in my workspace?"

**Vision**: Aggregate view showing workspace-wide health at a glance.

**What the user can do**:
Press 'h' to see health dashboard overlay:
```
┌ Workspace Health ─────────────────────────────────┐
│ ⚠️  3 repos with uncommitted changes              │
│ 📦 2 repos behind remote                          │
│ ✗  1 repo with failing tests (last known)        │
│ 🔗 0 dependency conflicts                         │
│                                                    │
│ Dirty Repos:                                      │
│   • graft (12 files modified)                     │
│   • grove (3 files modified)                      │
│   • notebook (1 file modified)                    │
│                                                    │
│ Behind Remote:                                    │
│   • graft (↓3 commits)                            │
│   • python-starter (↓1 commit)                    │
└────────────────────────────────────────────────────┘
```

**Scope**:
- Aggregate dirty count across repos
- Aggregate behind/ahead counts
- Test status from command history (if available)
- Dependency conflict detection (graft-aware)
- Quick navigation to problem repos

**Priority**: **High** — provides workspace-level situational awareness

---

### 🆕 Slice 9: Bulk Operations

**User story**: "Run this command across multiple repos."

**Vision**: Multi-select repos and execute commands across selection.

**What the user can do**:
```
1. Press 'm' to enter multi-select mode
2. Use 'space' to toggle selection, 'a' to select all
3. Press 'x' to execute command across selection
4. See aggregated output with per-repo results
```

**UI example**:
```
┌ Repositories (Multi-Select Mode) ─────────────────┐
│ [x] ~/src/graft [main] ● ↑2                       │
│ [x] ~/src/grove [main] ○                          │
│ [ ] ~/src/notebook [main] ●                       │
│ [x] ~/src/python-starter [main] ○ ↓1              │
│                                                    │
│ 3 selected | space: toggle | a: all | x: execute │
└────────────────────────────────────────────────────┘
```

**Scope**:
- Multi-select state management
- Bulk command execution (parallel or sequential)
- Aggregated results display
- Success/failure summary

**Priority**: **High** — common workflow (git pull across all, run tests, etc.)

---

### 🆕 Slice 10: Dependency Graph Navigation

**User story**: "How are my repos connected?"

**Vision**: Navigate workspace via graft dependency relationships.

**What the user can do**:
Press 'd' in detail pane to see dependencies view:
```
┌ Dependencies for graft ───────────────────────────┐
│ meta-knowledge-base                               │
│   Source: git@github.com:yourorg/meta-kb.git      │
│   Ref: main @ abc1234                             │
│   Status: ✓ In sync                               │
│   [Press Enter to jump to repo]                   │
│                                                    │
│ python-starter                                    │
│   Source: git@github.com:yourorg/python.git       │
│   Ref: v1.2.0 @ def5678                           │
│   Status: ⚠️  Outdated (v1.3.0 available)         │
│   [Press Enter to jump to repo]                   │
│                                                    │
│ Used by (reverse deps):                           │
│   • graft-knowledge (needs graft)                 │
│   • example-project (needs graft)                 │
└────────────────────────────────────────────────────┘
```

**Scope**:
- Parse dependencies from graft.yaml
- Cross-reference with .gitmodules
- Detect sync status (submodule ref matches graft.yaml ref)
- Jump to dependency source repo in workspace
- Show reverse dependencies (who depends on this repo)
- Dependency update suggestions (via git tags/branches)

**Priority**: **High** — core graft integration feature (completes Slice 5)

---

### 🆕 Slice 11: Activity Timeline

**User story**: "What's changed recently across everything?"

**Vision**: Chronological view of recent activity across all workspace repos.

**What the user can do**:
Press 't' to see activity timeline:
```
┌ Workspace Activity (Last 7 Days) ────────────────┐
│ Today, 3:28 PM - graft (main)                    │
│   feat(grove): Phase 1 - Critical UX improvements│
│   daniel                                          │
│                                                   │
│ Today, 2:15 PM - notebook (main)                 │
│   capture: Quick notes from meeting              │
│   daniel                                          │
│                                                   │
│ Yesterday, 4:00 PM - grove (main)                │
│   docs(grove): Add vertical slices               │
│   daniel                                          │
│                                                   │
│ [Enter to jump to repo/commit]                   │
└────────────────────────────────────────────────────┘
```

**Scope**:
- Aggregate recent commits from all repos
- Sort chronologically
- Filter by date range, author, repo
- Jump to commit context

**Priority**: **Medium** — nice for team awareness, less critical for solo

---

### 🆕 Slice 12: Workspace Snapshots

**User story**: "Save this working state for later."

**Vision**: Record and restore workspace state (all repos at specific refs).

**What the user can do**:
```
Press 's' → Create snapshot dialog
  > Snapshot name: [release-2.1▊]
  > Description: [Working state before refactor▊]

Saves: release-2.1.snapshot.yaml with all repo refs

Press 'S' → Load snapshot dialog
  > Select snapshot: [release-2.1]
  > Confirm: Checkout all repos to snapshot refs? [Y/n]

Restores: All repos to exact refs from snapshot
```

**Snapshot file format**:
```yaml
name: release-2.1
created: 2026-02-13T15:30:00Z
author: daniel
description: Working state before refactor
repos:
  - path: ~/src/graft
    ref: c192d4566f27066c4388698b7a5427cd0ebeea5d
    branch: main
  - path: ~/src/grove
    ref: a1b2c3d4e5f6
    branch: main
```

**Scope**:
- Capture current refs of all workspace repos
- Save snapshot to ~/.grove/snapshots/{workspace-name}/
- Load snapshot and checkout all repos
- Share snapshots with team (commit to workspace repo)

**Priority**: **Low to Medium** — powerful for reproducibility, niche use case

---

### 🆕 Slice 13: Smart Repo Filtering

**User story**: "Show me just the dirty repos / just graft repos / etc."

**Vision**: Filter workspace view by repo attributes.

**What the user can do**:
```
Press 'f' → Filter menu appears
  [ ] Dirty only
  [ ] Behind remote
  [ ] Ahead of remote
  [ ] Has graft.yaml
  [x] Tagged: work
  [ ] Tagged: personal

Apply → Repo list shows only matching repos
```

**Scope**:
- Filter by git status (dirty, ahead, behind, clean)
- Filter by graft presence
- Filter by workspace.yaml tags
- Combine filters (AND/OR logic)
- Persist filter across sessions

**Priority**: **Medium** — valuable for large workspaces (10+ repos)

---

### 🆕 Slice 14: Command History & Favorites

**User story**: "Re-run that command from yesterday."

**Vision**: Command execution history with quick re-run.

**What the user can do**:
```
Press 'H' → Command history overlay
  Recent Commands:
    1. graft run capture Personal "Meeting notes"
    2. graft run test
    3. grove run build --release

  Favorites:
    ⭐ graft run capture Personal
    ⭐ graft run test

Press Enter on history item → Re-run with same args
Press 's' on history item → Save as favorite
```

**Scope**:
- Persist command execution history (~/.grove/history.jsonl)
- Show recent commands per repo
- Quick re-run (Enter)
- Favorite commands (s to star)
- Edit args before re-run (e to edit)

**Priority**: **Medium** — power user productivity feature

---

### 🆕 Slice 15: Integration Hub (External Status)

**User story**: "What's the CI/CD status for these repos?"

**Vision**: Surface external status (CI, tests, PRs) directly in Grove.

**What the user can do**:
```
Detail pane shows:
  Branch: main ● ↑2
  CI Status: ✓ All checks passed (GitHub Actions)
  PRs: 2 open (PR #42, PR #43)
  Coverage: 85% (↑3% from main)
  Last Deploy: 2 hours ago (production)
```

**Scope**:
- GitHub Actions status via gh CLI
- Open PR count and titles
- Coverage badges (via codecov API)
- Deploy status (via configured webhooks)
- Slack/Discord notifications on command completion

**Priority**: **Low** — nice for teams, requires external integrations

---

## Revised Priority Roadmap

### Phase 1: Complete Core Graft Integration (Next Sprint)
1. ✅ **Slice 7: Command Execution** (Done + Phase 1 UX)
2. 🎯 **Slice 10: Dependency Graph Navigation** (Complete Slice 5)
   - Parse dependencies section
   - Show dependency status
   - Jump to dependency repos
   - Reverse dependency display

### Phase 2: Workspace Operations (Following Sprint)
3. 🎯 **Slice 8: Workspace Health Dashboard**
   - Aggregate dirty/behind counts
   - Problem repo highlighting
   - Quick navigation to issues

4. 🎯 **Slice 9: Bulk Operations**
   - Multi-select mode
   - Bulk command execution
   - Aggregated results

### Phase 3: Enhanced Workflows (Future)
5. 🎯 **Slice 3v2: Smart Capture** (Redesigned)
   - Leverage argument dialog UX
   - Template support
   - Routing to destinations

6. 🎯 **Slice 6v2: Streaming Ripgrep Search** (Simplified)
   - No indexing, instant results
   - Real-time streaming
   - $EDITOR integration

7. 🎯 **Slice 13: Smart Repo Filtering**
   - Filter by status, tags, graft presence
   - Persist filters

### Phase 4: Power User Features (Future)
8. 🎯 **Slice 14: Command History & Favorites**
9. 🎯 **Slice 11: Activity Timeline**
10. 🎯 **Slice 4v2: Link Following** (Replace file nav)
11. 🎯 **Slice 12: Workspace Snapshots**

### Phase 5: Team Features (Future)
12. 🎯 **Slice 15: Integration Hub**

---

## Architectural Considerations

### Pattern: Modal Overlays
- Argument input, command picker, help → all use centered overlay pattern
- **Consistent UX** across all modal interactions
- New slices should follow this pattern:
  - Health dashboard → overlay
  - Search results → overlay
  - Capture dialog → overlay
  - Filter menu → overlay

### Pattern: Detail Pane Views
- Current: commits, changed files
- Coming: dependencies (Slice 10)
- Future: external status (Slice 15)
- **View switching**: Use enum for detail pane modes, cycle with keys

### Pattern: Background Threads
- Command execution uses background thread + channel
- Search, bulk operations should follow same pattern
- Keeps TUI responsive during long operations

### Pattern: Graceful Degradation
- Git operations timeout after 5s
- Partial data display on errors
- **Continue this pattern** for all external integrations

---

## Technology Additions Needed

### For Slice 10 (Dependency Graph)
- Already have: graft.yaml parsing
- Need: .gitmodules parsing
- Need: git tag/branch listing (for update suggestions)

### For Slice 9 (Bulk Operations)
- Already have: command execution
- Need: parallel execution (tokio or rayon)
- Need: result aggregation

### For Slice 6v2 (Streaming Search)
- Need: ripgrep as subprocess (already using std::process pattern)
- Alternative: `grep` crate (pure Rust)

### For Slice 3v2 (Smart Capture)
- Need: $EDITOR integration (TUI suspend/resume)
- Already have: file creation, git commit (via graft)

### For Slice 15 (Integration Hub)
- Need: `gh` CLI for GitHub API
- Optional: REST clients for other APIs

---

## Success Metrics

### Current State (Slice 1, 2, 7)
- ✅ Can view workspace repos with git status
- ✅ Can see repo detail (commits, changed files)
- ✅ Can execute graft commands with arguments
- ✅ Argument input has best-in-class UX (cursor nav, preview, validation)

### After Phase 1 (Add Slice 10)
- ✅ Can navigate dependency graph
- ✅ Can see dependency sync status
- ✅ Can jump between related repos
- 🎯 **Grove becomes graft's visual companion**

### After Phase 2 (Add Slices 8, 9)
- ✅ Can see workspace health at a glance
- ✅ Can run commands across multiple repos
- ✅ Can handle bulk operations efficiently
- 🎯 **Grove becomes workspace orchestration hub**

### After Phase 3 (Add Slices 3v2, 6v2, 13)
- ✅ Can capture notes from TUI
- ✅ Can search across all repos
- ✅ Can filter workspace views
- 🎯 **Grove becomes primary workspace interface**

---

## Open Questions

### Strategic
1. Should Grove support non-graft repos as first-class?
   - **Current**: Yes, graft is optional
   - **Future**: Continue this — graft features enhance but don't require

2. Should Grove have workspace-level commands (not repo-specific)?
   - **Proposal**: Yes, in workspace.yaml under top-level `commands` section
   - **Use cases**: Workspace setup, bulk git operations, documentation generation

3. Should Grove cache expensive operations (git queries, dependency checks)?
   - **Current**: Detail pane caching exists
   - **Future**: Consider persistent cache (~/.grove/cache/) for dependency status

### Tactical
1. How to handle $EDITOR integration for multi-line capture?
   - **Option A**: Ctrl+E to launch $EDITOR (like git commit)
   - **Option B**: Dedicated multi-line input widget
   - **Recommendation**: Option A (leverages existing tools)

2. How to display bulk operation results?
   - **Option A**: Tabbed pane (one tab per repo)
   - **Option B**: Aggregated view with expandable repo sections
   - **Recommendation**: Option B (keeps overview visible)

3. Should search use tantivy or ripgrep?
   - **Option A**: Tantivy (structured, ranked results, but complex)
   - **Option B**: Ripgrep (simple, instant, but no ranking)
   - **Recommendation**: Start with ripgrep, migrate to tantivy if needed

---

## Next Steps

### Immediate (This Week)
1. ✅ Review and update vertical slices documentation (this document)
2. 🎯 Update original vertical slices doc with implementation status
3. 🎯 Create Slice 10 implementation plan (Dependency Graph Navigation)
4. 🎯 Specify dependency graph UI and data model

### Short-Term (Next 2 Weeks)
5. 🎯 Implement Slice 10 (Dependencies view in detail pane)
6. 🎯 Implement Slice 8 (Health dashboard overlay)
7. 🎯 Write specification for Slice 9 (Bulk operations)

### Medium-Term (Next Month)
8. 🎯 Implement Slice 9 (Bulk operations)
9. 🎯 Redesign and spec Slice 3v2 (Smart capture)
10. 🎯 Prototype Slice 6v2 (Streaming search)

---

## Conclusion

Grove has evolved from a **repository viewer** into a **workspace orchestration hub** with exceptional command execution UX. The original vertical slices provided a solid foundation, but our understanding has deepened:

**Key Insights**:
1. **Command execution is the killer feature** — Phase 1 UX improvements proved this
2. **Graft integration is the differentiator** — dependency navigation is critical
3. **Workspace-level operations are underserved** — bulk ops, health dashboard fill this gap
4. **Modal overlays are the right pattern** — consistent with existing UX

**Recommended Path**:
- Complete Slice 10 (dependencies) to finish graft integration
- Add Slice 8 (health) and Slice 9 (bulk ops) for workspace orchestration
- Revisit capture and search with simpler designs (leverage existing patterns)
- Defer power-user features (history, snapshots, timelines) until core is solid

The vision: **Grove as the command center for polyrepo workflows, with graft as the dependency framework beneath.**
