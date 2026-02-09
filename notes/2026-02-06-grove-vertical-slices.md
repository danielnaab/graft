---
title: "Grove Vertical Slices"
date: 2026-02-06
status: working
participants: ["human", "agent"]
tags: [exploration, grove, tui, rust, vertical-slices, implementation]
---

# Grove Vertical Slices

## Context

The [Workspace UI Exploration](./2026-02-06-workspace-ui-exploration.md) laid out Grove's architecture (config → engine → TUI) and five broad horizontal phases (Foundation → Search → Commands → Native → Web). That's useful for scoping but dangerous for building — horizontal phases encourage building invisible infrastructure before delivering user value.

This document reframes the plan as **narrow vertical slices**: thin, end-to-end features that each cut through all layers and deliver a demoable, usable capability. Each slice ships something a person can use. The focus is on the `grove` tool itself; graft modifications happen reactively when a slice needs them.

### What "vertical" means here

Every slice touches all three layers:

```
Config (workspace.yaml)  →  Engine (Rust lib)  →  TUI (ratatui)
```

No slice is "just plumbing" or "just UI." Each one wires a user-visible capability from config through logic to screen.

---

## Slice 1: Workspace Config + Repo List TUI

**User story**: "Show me my repos and their git status."

### What the user can do after this ships

Launch `grove`, see a list of configured repositories with per-repo git status (branch, clean/dirty, ahead/behind). Navigate the list with `j/k`. Quit with `q`.

### Scope

**In:**
- `workspace.yaml` parsing (repo paths, workspace name)
- Repo registry in the engine layer (enumerate repos, validate paths exist)
- Git status per repo via gitoxide (branch name, working tree dirty, ahead/behind tracking branch)
- Ratatui app scaffold: event loop, repo list widget, status indicators
- Basic keybindings: `j/k` navigate, `q` quit

**Out:**
- Repo detail pane (Slice 2)
- Capture, search, commands (later slices)
- File watching / live refresh
- Any graft awareness

### Key technical decisions

- **Crate scaffold**: Cargo workspace with `grove` binary crate and `grove-engine` library crate. Keep the split from day one so the engine is testable without the TUI.
- **Config format**: YAML via `serde` + `serde_yaml`. Start minimal — just `name` and `repositories[].path`.
- **Git**: `gitoxide` (`gix`) for status queries. No shelling out to git.
- **TUI**: `ratatui` with `crossterm` backend. Single-threaded event loop to start.

### What this validates

- Is the workspace config ergonomic?
- Can gitoxide deliver status information fast enough for interactive use?
- Is the ratatui scaffold sound for incremental extension?

---

## Slice 2: Repo Detail Pane

**User story**: "What's happening in this repo?"

### What the user can do after this ships

Select a repo in the list and see a detail pane showing: recent commits (last ~10), changed files in the working tree, and the current branch with tracking info. Press `Enter` to toggle focus between list and detail.

### Scope

**In:**
- Split-pane TUI layout (repo list left, detail pane right)
- Recent commit log via gitoxide (hash, subject, author, relative date)
- Working tree changed files list
- Focus management between panes (`Tab` or `Enter` to switch)

**Out:**
- Diff viewing (could be a future enhancement)
- File content preview
- Any git mutation operations (commit, push, pull)

### Key technical decisions

- **Layout**: ratatui `Layout::horizontal` with configurable split ratio. The detail pane is the primary extension point for later slices.
- **Commit log**: gitoxide rev-walk, limited to HEAD~10. Keep it simple — this isn't lazygit.
- **Async**: Decide whether git queries run on a background thread or block. For ~10 commits this is likely fast enough synchronous, but design the engine API as `async`-ready.

### What this validates

- Is the split-pane layout workable for progressive enhancement?
- Is gitoxide's commit walking API ergonomic for our use case?

---

## Slice 3: Quick Capture

**User story**: "Capture this thought, right now."

### What the user can do after this ships

Press `c`, type a note in the bottom bar, press `Enter`. The note is saved as a timestamped markdown file in the configured inbox directory and auto-committed. The user sees confirmation inline.

### Scope

**In:**
- Capture config in `workspace.yaml` (`capture.inbox` path, `capture.auto_commit`)
- Bottom-bar text input widget (ratatui `Paragraph` in input mode)
- File creation: `{inbox}/{timestamp}-{slug}.md` with optional frontmatter template
- Auto-commit via gitoxide: stage file, create commit with message `capture: {first line}`
- Mode switching: normal mode → capture mode → normal mode

**Out:**
- Multi-line capture (Ctrl+E → $EDITOR is a nice follow-up but not in this slice)
- Capture routing with `@repo:path/` prefix (later enhancement)
- Capture history / review UI

### Key technical decisions

- **Input handling**: Modal input — `c` enters capture mode, `Esc` cancels, `Enter` confirms. Keep it vim-like since the rest of the keybindings are vim-style.
- **File naming**: `2026-02-06T14-30-00-slug.md` where slug is derived from the first few words. Use a simple slugify (lowercase, hyphens, truncate).
- **Git commit via gitoxide**: This is the first write operation through gitoxide. If the API is painful, fall back to shelling out to `git` as a pragmatic escape hatch, but try gitoxide first.

### What this validates

- Is the capture flow fast enough to be frictionless? (Target: < 2 seconds from keypress to committed.)
- Does modal input feel natural in the TUI?
- Can gitoxide handle staging + committing reliably?

---

## Slice 4: File Navigation + $EDITOR

**User story**: "Open that file."

### What the user can do after this ships

In the detail pane, navigate the file tree of the selected repo. Press `Enter` on a file to open it in `$EDITOR`. Press `Backspace` or `-` to go up a directory. See file types with simple indicators.

### Scope

**In:**
- File tree widget in the detail pane (replaces or augments the commit log view)
- Directory traversal using `std::fs` (not git — show the working tree as-is)
- View switching in the detail pane: `l` for log, `f` for files
- `$EDITOR` integration: suspend TUI, launch editor, restore TUI on exit
- Respect `.gitignore` for filtering (via `gix` ignore rules or `ignore` crate)

**Out:**
- In-TUI file preview / syntax highlighting (too much scope)
- File operations (create, delete, rename)
- Git diff view for individual files

### Key technical decisions

- **TUI suspend/resume**: `crossterm` supports `LeaveAlternateScreen` / `EnterAlternateScreen`. The pattern is: leave screen → spawn `$EDITOR` as child process → wait → re-enter screen. Well-established pattern.
- **File tree**: Lazy-loaded. Only read directory entries when a directory is expanded. Use `ignore` crate for gitignore filtering.
- **Detail pane modes**: The detail pane now has multiple views (log, files). Use an enum to track the current mode. This is the pattern for all future detail pane content.

### What this validates

- Does $EDITOR integration work smoothly across terminals?
- Is the detail pane mode-switching pattern extensible?

---

## Slice 5: Graft Metadata Display

**User story**: "What are the dependencies?"

### What the user can do after this ships

For repos that contain `graft.yaml`, see a "Dependencies" view in the detail pane (press `d`). Shows each dependency with its source, current ref, and submodule status. For non-graft repos, the `d` key shows "No graft.yaml found."

### Scope

**In:**
- `graft.yaml` parsing in the engine (just the `dependencies` section)
- `.gitmodules` parsing for submodule state
- Dependencies detail pane view: name, source URL, pinned ref, submodule sync status
- Visual indicators: in-sync, out-of-sync, missing submodule

**Out:**
- `graft.lock` parsing (adds complexity, defer until graft CLI exists to generate it)
- Dependency mutations (upgrade, add, remove — that's the graft CLI's job)
- Recursive dependency display (flat-only model means one level)
- Commands from graft.yaml (Slice 7)

### Key technical decisions

- **Parsing**: Use `serde` + `serde_yaml` to deserialize the `dependencies` section of `graft.yaml`. Define minimal Rust structs — don't try to model all of graft.yaml, just what we display.
- **Submodule state**: Parse `.gitmodules` and cross-reference with `graft.yaml` dependencies. Check if the submodule checkout matches the expected ref.
- **Graft as optional**: The engine should treat graft metadata as entirely optional. A repo without `graft.yaml` is first-class.

### What this validates

- Is reading graft files directly sufficient, or do we need structured graft CLI output sooner?
- Is the dependency display useful without mutation capabilities?
- Does the flat-only model make the display straightforward?

---

## Slice 6: Cross-Repo Search

**User story**: "Find this across everything."

### What the user can do after this ships

Press `/`, type a query, see results across all workspace repos grouped by repo and file. Navigate results with `j/k`, press `Enter` to open the file at the matching line in `$EDITOR`.

### Scope

**In:**
- Search engine integration in `grove-engine` (tantivy index)
- Index building: on first launch, index all text files across workspace repos
- Incremental indexing: re-index changed files on subsequent launches (compare git status)
- Search results TUI: full-screen overlay with query input and grouped results
- Result navigation → open in `$EDITOR` at line number
- Search config: `search.exclude` patterns from `workspace.yaml`

**Out:**
- Live / background indexing (rebuild on launch is good enough to start)
- Faceted search (filter by repo, file type — nice to have later)
- Regex search (tantivy does full-text, not regex; could add ripgrep fallback later)
- Search result preview / context lines in TUI

### Key technical decisions

- **Tantivy vs ripgrep**: Start with tantivy for structured full-text search with relevance ranking. If tantivy's setup cost is too high for the first cut, fall back to streaming `grep`-style search via the `grep` crate, then migrate to tantivy.
- **Index location**: `~/.cache/grove/{workspace-name}/search-index/`. Ephemeral — can always be rebuilt.
- **Index lifecycle**: Build on launch, skip files unchanged since last index (using git status as the change detector). No daemon, no watcher.

### What this validates

- Is tantivy's indexing fast enough for interactive use on a workspace of ~5-20 repos?
- Is search-on-launch acceptable, or do we need background indexing?
- Is the overlay pattern right for search, or should it be a persistent pane?

---

## Slice 7: Command Execution

**User story**: "Run that command."

### What the user can do after this ships

Press `r` in a repo to see available commands. For graft-aware repos, this includes commands defined in `graft.yaml`. For any repo, it includes user-configured workspace commands. Select a command, see its output streamed in a pane. Scroll through output, press `q` to dismiss.

### Scope

**In:**
- Commands from `graft.yaml` parsing (the `commands` section)
- Workspace-level command config in `workspace.yaml`
- Command picker UI (list of available commands for the selected repo)
- Command execution: spawn child process, capture stdout/stderr
- Output pane: streaming display, scrollable, dismissable
- Basic execution status: running, succeeded, failed

**Out:**
- Execution history / persistence across sessions
- Concurrent command execution
- Command composition / chaining
- Interactive commands (commands that need stdin)

### Key technical decisions

- **Process management**: `std::process::Command` with piped stdout/stderr. Read output in a background thread, push lines to the TUI via the event loop's channel.
- **Output display**: Ratatui scrollable paragraph widget. Limit retained output to prevent memory issues (last N lines, or configurable).
- **Security**: Commands from `graft.yaml` and `workspace.yaml` are explicitly configured by the user, so there's no injection risk. Don't support arbitrary command input from the TUI — use $EDITOR or a shell for that.

### What this validates

- Is the command picker useful, or do people just want a shell?
- Is streaming output display in ratatui smooth enough?
- Are graft.yaml commands a natural fit for this interaction?

---

## Slice Summary

| # | Slice | Layers touched | Key crate additions |
|---|-------|---------------|-------------------|
| 1 | Workspace config + repo list | config, engine, TUI | serde_yaml, gix, ratatui, crossterm |
| 2 | Repo detail pane | engine, TUI | (gix rev-walk) |
| 3 | Quick capture | config, engine, TUI | (gix staging/commit) |
| 4 | File navigation + $EDITOR | engine, TUI | ignore |
| 5 | Graft metadata display | config, engine, TUI | (serde_yaml for graft.yaml) |
| 6 | Cross-repo search | config, engine, TUI | tantivy |
| 7 | Command execution | config, engine, TUI | (std::process) |

Each slice is independently demoable. Together they build the full Phase 1-3 vision from the workspace UI exploration, delivered incrementally rather than in horizontal layers.

---

## Sources

- [Workspace UI Exploration (2026-02-06)](./2026-02-06-workspace-ui-exploration.md) — horizontal phase plan this refines
- [Graft Architecture](../docs/specifications/architecture.md)
- [ratatui](https://ratatui.rs/) — Rust TUI framework
- [gitoxide](https://github.com/GitoxideLabs/gitoxide) — Pure Rust git implementation
- [tantivy](https://github.com/quickwit-oss/tantivy) — Rust full-text search
