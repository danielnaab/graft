---
title: "Grove: Command Line and View Stack"
date: 2026-02-18
status: working
participants: ["human", "agent"]
tags: [exploration, grove, tui, prompt, commands, interaction-model, ux, views]
---

# Grove: Command Line and View Stack

## Context

Exploration of whether a chat-style interface (like Claude Code, OpenCode) with a prompt area and slash commands would be appropriate for Grove. Concluded: not the chat part, but the prompt part — and the prompt idea leads to a deeper rethinking of the content area itself.

Starting question: does a persistent prompt/command line belong in Grove's TUI?

Conclusion: yes, as a summoned command line (`:` key, vim-style). But this also reveals that the current two-pane layout (fixed repo list + tabbed detail) is a constraint, not a requirement. The content area should be a **view stack** with a **command line** as the dispatch interface.

---

## The Dispatch Radio Metaphor

Grove's core metaphor evolved from "departure board" (passive display) to "dispatch board" (active coordination). The right prompt metaphor follows from this.

A dispatch center has:

- **The board** — current state of everything being tracked (the dashboard)
- **The radio** — short, imperative utterances to issue orders and ask questions (the command line)
- **The log** — chronological record of what happened (activity history)

The radio is not a conversation. You key the mic, say something terse, and the board updates. This is fundamentally different from Claude Code's chat model, where the conversation IS the product.

**The command line is the radio. The board is still the point.**

This means:
- **Summoned on demand** like vim's `:` — the board stays primary until you need to issue a command
- **One-shot** like a radio call — enter command, view updates, prompt disappears
- **Context-aware** — knows which repo is selected, what's running, what view you're in
- **Not** persistent/always-visible (competes with the board)
- **Not** conversational (no history, no scrollback, no agent responses inline)

---

## The `:` Command Line

Press `:` to activate. A single-line input appears at the bottom of the screen. Type a command. Enter executes. Escape cancels. The content area stays fully visible above.

### Colon, Not Slash

Two conventions exist: `:command` (vim/tmux/less) and `/command` (Claude Code/Slack). Grove should use `:` because:

1. Grove is already vim-flavored (`j`/`k`, `q` to quit, modal views)
2. The colon command line has decades of established UX
3. `/` is better reserved for search (another vim convention)
4. The metaphor is "command mode," not "chat"

Split: `:` enters command mode (structured commands), `/` enters search mode (text search across repos). Both appear in the same bottom bar, behave similarly, but have different intent.

### Discoverability

`:` with no further input shows a command palette — a list of available commands with descriptions. Typing narrows the list. This gives both discoverability (browse the list) and speed (type the command directly if you know it).

```
┌─ Commands ──────────────────────┐
│  capture    Capture a note      │
│  filter     Filter repo list    │
│  launch     Start a session     │
│  monitor    View session output │
│  refresh    Refresh all repos   │
│  run        Run a graft command │
│  search     Search across repos │
│  sessions   List sessions       │
│  workspace  Switch workspace    │
└─────────────────────────────────┘
:█
```

### Relationship to Keybindings

The command line **supplements** keybindings, doesn't replace them. `j`/`k`/`r`/`Enter`/`q` still work. The command line is for actions that don't have keybindings, for one-shot execution (`:run test --verbose` instead of `x` → pick → args dialog), and for power users who prefer typing.

Both paths to the same action: `x` opens the command picker (discoverability), `:run test` executes directly (speed). Same principle as vim: multiple paths, user chooses.

---

## The View Stack

### Why the Two-Pane Layout Is a Constraint

The current layout dedicates 40% to the repo list and 60% to tabbed detail. This creates several pressures:

- **Repo list is cramped** — at 80 columns, 32 characters for path + branch + status. Fish-style abbreviation and prefix truncation are workarounds for insufficient width.
- **Detail pane needs tabs** — Changes, State, and Commands tabs exist because 60% width can only show one thing well. Tabs are a space-management compromise, not a design ideal.
- **Modals proliferate** — argument input is a centered dialog, help is a full-screen overlay, command output replaces the detail pane. Four different interaction patterns for conceptually similar actions.
- **Scaling problem** — as Grove adds views (sessions, search results, review diffs, plans, context documents), they all have to fit into the 60% right pane or become yet another overlay pattern.

### Everything Is a View

Inspired by vim buffers and Emacs's "everything is a buffer" philosophy: Grove has exactly one primitive for the content area — the **view**. Everything is a view.

- **Dashboard view**: the repo list, full-width
- **Repo view**: detail for a specific repo (changes, commits, state, commands)
- **Output view**: streaming command or session output
- **Search view**: cross-repo search results
- **Session list view**: all sessions across the workspace
- **Review view**: diff for a completed session
- **Help view**: keybindings and commands

No panes. No tabs. No modals. No overlays. Just views.

### Navigation: Hybrid Stack with Shortcuts

Primary navigation is a **stack**. Enter pushes a view, `q` pops back. Dashboard → Repo → Output. You always know where `q` takes you: backwards.

Direct jumps for common destinations: Escape goes home (dashboard) from anywhere. `:repo finances` jumps directly to a repo view. Direct jumps **reset** the stack rather than pushing deeper — you don't end up 8 levels deep.

Typical flow:
```
[Dashboard]                          # scan repos
  Enter → [Dashboard, Repo:graft]   # focus on one
    :run test → [Dashboard, Repo:graft, Output:test]  # run command
      q → [Dashboard, Repo:graft]   # see result, go back
    q → [Dashboard]                  # back to scanning

Escape from anywhere → [Dashboard]   # go home directly
:repo finances → [Repo:finances]     # jump directly, stack resets
```

This gives the simplicity of a stack with the directness of buffer switching. Grove isn't an editor where you constantly flip between 8 open files — the typical flow is scan → focus → act → go back. A stack handles that perfectly. Direct jumps handle the exception.

### What Full Width Gives the Dashboard

At 40% width (current), each repo line is cramped and abbreviated. At full width:

```
┌─ Grove: my-workspace ───────────────────────────────────────────────────┐
│                                                                          │
│  ~/src/graft           main ○ ↑1   2h ago "Fix parse error in config"   │
│  ~/src/grove-brain     main ●      15m    "Add session primitives"      │
│  ~/finances            main ○      32d    ⚠ Monthly close overdue       │
│  ~/notes               main ● ↑3   5m     📥 12 captures to organize   │
│                                                                          │
│                                                                          │
│                                                                          │
│                                                                          │
├──────────────────────────────────────────────────────────────────────────┤
│ j/k navigate  Enter open  : command  / search  ? help                   │
└──────────────────────────────────────────────────────────────────────────┘
```

Path, branch, dirty/clean, ahead/behind, recency, last commit subject, AND status script signals — all on one line. The departure board becomes genuinely informative at a glance without needing a detail pane for basic status. You only drill into a repo when you need file-level changes or to run commands.

### What Full Width Gives the Repo View

At 60% width (current), detail needs tabs to show different categories. At full width:

```
┌─ ~/src/graft ── main ○ ↑1 ──────────────────────────────────────────────┐
│                                                                           │
│  Changed Files              │ Recent Commits                              │
│    M src/engine.rs          │   a1b2c3 Fix parse error in config  (2h)   │
│    A src/new_module.rs      │   d4e5f6 Add session primitives     (5h)   │
│    D old_file.rs            │   789abc Refactor config loader     (1d)   │
│                             │   def012 Update workspace spec      (2d)   │
│  State Queries              │                                             │
│    words/day: 1,247  (5m)   │ Commands                                    │
│    open/done: 3/12   (2h)   │   test     Run test suite                   │
│                             │   lint     Check formatting                 │
│                             │   deploy   Push to staging                  │
│                                                                           │
├───────────────────────────────────────────────────────────────────────────┤
│ q back  :run <cmd>  r refresh  j/k scroll                                │
└───────────────────────────────────────────────────────────────────────────┘
```

Changes, commits, state, and commands all visible at once. No tabs needed.

### Ambient Awareness

The two-pane layout provides ambient awareness of other repos while focused on one. Losing that is a tradeoff. Counterarguments:

1. The dashboard is one keystroke away (`q` or `Escape`).
2. In practice, repo status only changes on manual refresh — you won't miss a change while looking at detail.
3. For session monitoring (wanting to know when an agent finishes), a status bar indicator solves this without dedicating 40% of the screen to a permanent list.
4. The full-width dashboard is *better* for scanning than the 40% version. Trade always-visible-but-cramped for on-demand-but-complete.

### Keybindings Are View-Specific

Each view defines its own keybindings. The hint bar updates to show what's available in the current view. This is already how Grove works (context-sensitive hint bar), but making it explicit and per-view clarifies the model.

- **Dashboard**: `j`/`k` navigate repos, `Enter` opens repo, `/` searches, `:` commands
- **Repo view**: `j`/`k` scroll, `r` refresh, `:run` executes, `q` back
- **Output**: `j`/`k` scroll, `q` close/stop, `y`/`n` confirm stop
- **Search**: `j`/`k` navigate results, `Enter` opens result, `q` back

No mode confusion because the view IS the mode, and the hint bar always tells you what keys do.

---

## Mapping Existing and New Features to Commands

### Existing Features

| Feature | Current Interaction | With Command Line |
|---------|---------------------|-------------------|
| Navigate repos | `j`/`k` | `j`/`k` (unchanged) |
| View repo detail | `Enter` | `Enter` or `:repo <name>` |
| Refresh | `r` | `r` or `:refresh` |
| View state | `s` | `s` or `:state` |
| Run command | `x` → pick → args dialog | `:run <cmd> [args]` (one-shot) |
| Help | `?` | `?` or `:help` |

### New Features Enabled

| Feature | Command | View Pushed |
|---------|---------|-------------|
| Capture (Slice 3) | `:capture @prefix text` | None (status bar confirms) |
| Search (Slice 6) | `/pattern` | Search results view |
| Quick repo filter | `:filter dirty` or `:filter #tag` | Filters dashboard in-place |
| Launch session (Slice 8) | `:launch "cmd"` | Output or session list view |
| List sessions | `:sessions` | Session list view |
| Monitor session (Slice 9) | `:monitor <id>` | Output view (streaming) |
| Attach session (Slice 9) | `:attach <id>` | Suspends TUI, attaches tmux |
| Context (Slice 10) | `:context [repo]` | Context document view |
| Review (Slice 11) | `:review <id>` | Review/diff view |
| Plan (Slice 12) | `:plan run <name>` | Plan status view |
| Switch workspace | `:workspace <name>` | Reloads dashboard |

---

## Design Decisions

- **2026-02-18**: Command line uses `:` prefix, not `/`
  - Consistent with vim-flavored keybinding model
  - `/` reserved for search
  - "Command mode" metaphor, not "chat" metaphor

- **2026-02-18**: Content area uses view stack, not fixed panes
  - Everything is a view — no modals, overlays, or tabs
  - Primary navigation is stack-based (`q` pops back)
  - Direct jumps (`:repo X`, `Escape` for home) reset the stack
  - Full-width views eliminate need for tabs and path abbreviation hacks

- **2026-02-18**: Command line supplements keybindings, doesn't replace them
  - Common actions keep their single-key shortcuts
  - Command line adds a surface for less-common and parameterized actions
  - Two paths to the same action (modal picker vs. direct command) is intentional

---

## Open Questions

- What is the full set of views and how do they chain? (dashboard → repo → output is clear; what about session list → session detail → review?)
- Should some views support internal splits (e.g., repo view showing changes and commits side by side as shown above), or should the view always be a single scrollable content area?
- How does the command registry architecture look? (`CommandDef` struct with name, aliases, completion function, execute function?)
- Should `:` with partial input show inline completions (like fish) or a dropdown menu (like VS Code)?
- How does filtering interact with the view stack? Is filtered-dashboard a separate view, or the same view with a filter applied?
- What role does the status bar play across views? (transient messages, session status indicators, etc.)

---

## Sources

- [Grove Agentic Orchestration (2026-02-18)](2026-02-18-grove-agentic-orchestration.md) — dispatch board metaphor, session/plan primitives
- [Grove Workflow Hub Primitives (2026-02-07)](2026-02-07-grove-workflow-hub-primitives.md) — capture routing, status scripts, workspace map metaphor
- [Grove TUI Behavior Spec](../docs/specifications/grove/tui-behavior.md) — current keybinding and interaction model
- [Grove Command Execution Spec](../docs/specifications/grove/command-execution.md) — current command picker and argument dialog
