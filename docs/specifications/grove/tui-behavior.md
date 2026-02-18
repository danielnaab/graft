---
status: working
last-verified: 2026-02-18
owners: [human, agent]
---

# TUI Behavior

## Intent

Define the interactive terminal interface for Grove — how users navigate repositories, view status, inspect detail, and manage focus across views. This spec covers the ratatui-based TUI that serves as Grove's primary user interface.

## Non-goals

- **Not a design system** — Colors and typography are implementation choices, not specified here
- **Not a styling spec** — Visual polish (spacing, alignment) is left to the implementation
- **Not an accessibility spec** — Screen reader support and contrast requirements are future concerns
- **Not the CLI interface** — Machine-readable `--json` output is a separate concern

## Architecture: View Stack

Grove's content area is a **view stack**. Every screen (repo list, repo detail, command output, help) is a full-width **view**. There are no split panes, tabs, or persistent side panels.

### View Types

| View | Description |
|------|-------------|
| `Dashboard` | Full-width repo list (home view) |
| `RepoDetail(idx)` | Full-width detail for a specific repository |
| `CommandOutput` | Full-width streaming command or session output |
| `Help` | Full-width keybindings reference |

### Stack Navigation

- `Enter` (from Dashboard) — pushes `RepoDetail` onto the stack
- `q` — pops the current view (go back one level); quits from Dashboard
- `Escape` — resets to Dashboard from any view (go home)
- `:repo <name>` — jumps directly to a `RepoDetail` view, replacing the stack

The stack invariant: always at least one element (`Dashboard`). Direct jumps reset the stack rather than pushing — so `:repo X` from deep in the stack takes you to `[RepoDetail(X)]`, not `[Dashboard, RepoDetail(X), ...]`.

### Overlays

Some interactions are **overlays** over the current view, not views themselves:

- **Argument input dialog** — appears when selecting a command via `x`; overlays the current view
- **Command line** — appears at the bottom when `:` is pressed; overlays any view

Overlays are dismissed with `Escape` (cancel) or `Enter` (submit). They do not push/pop the view stack.

---

## Behavior

### Repo List Display [Slice 1]

Each repository in the workspace is displayed as a single line in the list.

```gherkin
Given a workspace config with name "my-project"
When the TUI renders
Then the title bar shows "Grove: my-project"
And includes navigation hints
```

```gherkin
Given a workspace with repositories that have been status-checked
When the TUI renders the repo list
Then each repo shows: path, branch name (in brackets), dirty indicator, and ahead/behind arrows
```

```gherkin
Given a repository with no branch (detached HEAD)
When the TUI renders that repo's line
Then it shows "[detached]" instead of a branch name
```

```gherkin
Given a repository with uncommitted changes
When the TUI renders that repo's line
Then it shows a "●" dirty indicator
```

```gherkin
Given a repository with a clean working tree
When the TUI renders that repo's line
Then it shows a "○" clean indicator
```

```gherkin
Given a repository that is 3 commits ahead and 2 behind its remote
When the TUI renders that repo's line
Then it shows "↑3" and "↓2" after the dirty indicator
```

```gherkin
Given a repository with zero ahead/behind counts
When the TUI renders that repo's line
Then it does not show ahead/behind arrows (zero counts are hidden)
```

```gherkin
Given a repository whose status query returned an error
When the TUI renders that repo's line
Then it shows "[error: <message>]" with error styling differentiated by error type
```

```gherkin
Given a repository whose status has not yet been loaded
When the TUI renders that repo's line
Then it shows "[loading...]" as a pending state
```

```gherkin
Given a repository with a path that exceeds the available pane width
When the TUI renders that repo's line
Then the path is abbreviated to fit within the pane
And home directory is shown as "~"
And parent directory components are abbreviated
And the final path components are preserved
```

```gherkin
Given available space is very limited (narrow terminal or long status indicators)
When the TUI would severely compact the path (using "[..]" prefix or very short display)
Then the branch name is dropped from the display
And more space is allocated to show the repository path
And status indicators (dirty, ahead, behind) are still shown
```

```gherkin
Given available space is extremely limited (pane width < 15 columns)
When the TUI renders a repository line
Then only the repository basename is shown (e.g., "graft" not "~/src/graft")
And branch name is omitted
And status indicators (dirty, ahead, behind) are still shown
```

```gherkin
Given a workspace with no repositories configured
When the TUI launches
Then the repository list shows "No repositories configured"
And displays helpful instructions to edit workspace.yaml
And shows an example configuration
And no list item is selected (selection is None)
```

### Dashboard Navigation [Slice 1]

```gherkin
Given a repo list with 3 repositories and the first is selected
When the user presses "j" or Down arrow
Then the selection moves to the second repository
```

```gherkin
Given a repo list with 3 repositories and the last is selected
When the user presses "j" or Down arrow
Then the selection wraps to the first repository
```

```gherkin
Given a repo list with 3 repositories and the first is selected
When the user presses "k" or Up arrow
Then the selection wraps to the last repository
```

```gherkin
Given the Dashboard is the current view
When the user presses "q"
Then the application quits
```

```gherkin
Given the Dashboard is the current view
When the user presses Escape
Then the view stays on Dashboard (no-op — already home)
And the application does NOT quit
```

```gherkin
Given the TUI has just launched with repositories loaded
When the initial render occurs
Then the first repository in the list is selected
```

The selected item is highlighted with a "▶" prefix.

### Dashboard — Opening Repo Detail

```gherkin
Given the Dashboard is focused and a repository is selected
When the user presses Enter, Tab, "x", or "s"
Then the RepoDetail view is pushed onto the view stack
And the detail for the selected repository is shown
```

### Help View [Slice 1]

```gherkin
Given any view is current
When the user presses "?"
Then the Help view is pushed onto the view stack
And shows all keybindings with descriptions
And shows status indicator legend
And shows Grove version
```

```gherkin
Given the Help view is current
When the user presses "q"
Then the Help view is popped and the previous view is shown
```

```gherkin
Given the Help view is current
When the user presses Escape
Then the view stack resets to Dashboard
```

```gherkin
Given the terminal size is smaller than 44 columns or 20 rows
When the help view would be rendered
Then the help view adjusts to minimum viable dimensions
And content remains readable
```

### Manual Refresh [Slice 1]

The `r` key has **view-specific** behavior:
- In the **Dashboard**: refreshes all repository statuses
- In the **RepoDetail** view: refreshes state queries for the current repository

```gherkin
Given the Dashboard is current
When the user presses "r"
Then a "Refreshing..." message appears in the title
And the UI renders to show the message immediately
And all repository statuses are re-queried
And the display updates with fresh status
And a "Refreshed N repositories" confirmation message is shown
And the confirmation message clears automatically after 3 seconds
```

```gherkin
Given a refresh operation fails
When the error occurs
Then an error message is shown in the title bar
And the message describes what went wrong
And the message clears automatically after 3 seconds
```

```gherkin
Given the TUI is launching
When repositories are being loaded initially
Then "Loading N repositories..." is shown in the console
And "✓ Loaded N repositories" is shown when complete
And the TUI launches with fresh status
```

### RepoDetail View [Slice 2]

The RepoDetail view shows full-width information about the currently selected repository. All sections are visible simultaneously in a single scrollable view — no tabs.

```gherkin
Given a repository is selected and its RepoDetail view is rendered
Then the block title shows: branch name, dirty indicator, ahead/behind counts
And the view renders these sections vertically:
  - Changed Files (with count)
  - Recent Commits (with count)
  - State Queries
  - Available Commands
```

```gherkin
Given a repository is selected with branch "main", dirty, 2 ahead, 1 behind
When the RepoDetail view renders
Then the block title shows: branch name, dirty indicator ("●" or "○"), ahead count ("↑2"), behind count ("↓1")
```

```gherkin
Given the selected repository has 2 uncommitted changes
When the RepoDetail view renders
Then it shows "Changed Files (2)" as a section header
And each file shows a status indicator: M (modified), A (added/staged), D (deleted), R (renamed), C (copied), ? (untracked/unknown)
And each file shows its path
```

```gherkin
Given the selected repository has recent commits
When the RepoDetail view renders
Then it shows "Recent Commits (<count>)" as a section header
And each commit shows: abbreviated hash, subject
And below each commit: author and relative date
```

```gherkin
Given no repository is selected (empty workspace)
When the RepoDetail view would render
Then it shows "No repository selected"
```

```gherkin
Given the selected repository has no uncommitted changes
When the RepoDetail view renders
Then the changed files section shows "No uncommitted changes"
```

```gherkin
Given the selected repository has no commits
When the RepoDetail view renders
Then the commits section shows "No commits"
```

```gherkin
Given a detail query returned a partial error (e.g., changed files timed out but commits succeeded)
When the RepoDetail view renders
Then it shows "Error: <message>" as a warning at the top
And still renders the remaining available data below the error
```

### RepoDetail Scroll [Slice 2]

```gherkin
Given the RepoDetail view is current
When the user presses "j" or Down arrow
Then the detail content scrolls down by one line
```

```gherkin
Given the RepoDetail view is current
When the user presses "k" or Up arrow
Then the detail content scrolls up by one line (minimum 0)
```

```gherkin
Given the detail content is shorter than the view height
When the user scrolls down
Then scroll is clamped — cannot scroll past the end of content
```

```gherkin
Given the user navigates to a different repository in the list
When the RepoDetail view updates
Then scroll position resets to 0
```

### RepoDetail Navigation

```gherkin
Given the RepoDetail view is current
When the user presses "q" or Tab
Then the RepoDetail view is popped (go back to Dashboard)
And the application does NOT quit
```

```gherkin
Given the RepoDetail view is current
When the user presses Escape
Then the view stack resets to Dashboard
```

```gherkin
Given the RepoDetail view is current and the Commands section is visible
When the user presses "n" or "p"
Then the selected command moves forward or backward in the Commands list
```

```gherkin
Given the RepoDetail view is current and a command is selected
When the user presses Enter
Then the argument input dialog appears for the selected command
```

```gherkin
Given the RepoDetail view is current
When the user presses "r"
Then state queries are refreshed for the current repository
```

### Detail Query Behavior [Slice 2]

```gherkin
Given a repository is selected and its detail has been loaded
When the RepoDetail view renders again without changing selection
Then the cached detail is reused (no re-query)
```

```gherkin
Given the user navigates to a different repository
When the RepoDetail view renders
Then a fresh detail query is made for the new selection
And the result is cached for reuse
```

```gherkin
Given a detail provider returns an error
When the detail is loaded
Then an error state with the error message is cached (not a panic)
And the RepoDetail view renders the error state
```

### State Queries in RepoDetail

State query results are shown as a section within the RepoDetail view (not as a separate overlay or panel).

```gherkin
Given the RepoDetail view is rendering
And the repository has a graft.yaml file with state queries
Then the "State Queries" section shows each query name and cached result summary
And the cache age is displayed (e.g., "5m ago", "2h ago")
```

```gherkin
Given the RepoDetail view is rendering
And the repository has no graft.yaml file or no state queries
Then the "State Queries" section shows an empty state message
```

```gherkin
Given a state query returns data with "total_words" and "words_today"
Then the summary shows "X words total, Y today"
```

```gherkin
Given a state query returns data with "open" and "completed"
Then the summary shows "X open, Y done"
```

```gherkin
Given a state query returns data with "broken_links" and "orphaned"
Then the summary shows "X broken links, Y orphans"
```

```gherkin
Given a state query returns data in an unknown format
Then the summary shows generic information
And does not panic or show raw JSON
```

```gherkin
Given the RepoDetail view is current
When the user presses "r"
Then state queries are re-executed via graft CLI
And the status bar shows "Refreshing <query>..."
And the cache is updated with fresh results
And a success message is shown
```

### Edge Cases

#### Empty workspace

```gherkin
Given a workspace with no repositories
When the TUI launches
Then the repo list is empty
And no item is selected (selection is None)
And the RepoDetail view shows "No repository selected"
And navigation keys (j/k) do not panic
And selection remains None when navigation keys are pressed
```

#### Single repository

```gherkin
Given a workspace with exactly one repository
When the user presses "j" (down) or "k" (up) on the Dashboard
Then the selection stays on the same item (wrapping to self)
```

#### Provider failure isolation

```gherkin
Given the detail provider fails for a selected repository
When the RepoDetail view attempts to render
Then the detail view shows the error message
And the Dashboard continues to function normally
And navigation between repos still works
```

---

## Command Line [Phase 2]

Grove has a vim-style `:` command line for issuing commands. It supplements (does not replace) keybindings.

### Activation and Dismissal

```gherkin
Given any view is current (Dashboard, RepoDetail, Help, CommandOutput)
When the user presses ":"
Then the command line activates at the bottom of the screen
And the content area remains fully visible above
And the hint bar is replaced by the command line input
```

```gherkin
Given the command line is active
When the user presses Escape
Then the command line is dismissed
And the hint bar is restored
And no command is executed
```

```gherkin
Given the command line is active with a non-empty buffer
When the user presses Enter
Then the command is submitted and executed
And the command line is dismissed
```

```gherkin
Given the command line is active with an empty buffer
When the user presses Enter on a highlighted palette entry
Then the command line buffer is filled with that command's name
And the command line stays open (ready for arguments)
```

### Command Line Input

```gherkin
Given the command line is active
When the user types characters (excluding j/k)
Then each character is inserted at the cursor position
And the cursor advances
```

```gherkin
Given the command line is active
When the user presses Backspace
Then the character before the cursor is removed
And the cursor moves backward
```

```gherkin
Given the command line is active
When the user presses Left or Right arrow keys
Then the cursor moves in that direction
And stops at buffer boundaries
```

```gherkin
Given the command line is active
When the user presses Home or End
Then the cursor jumps to start or end of buffer
```

### Command Palette

```gherkin
Given the command line is active
Then a command palette popup is shown above the command line
And lists available commands with descriptions
```

```gherkin
Given the command line buffer contains partial text
When the palette renders
Then only commands whose name contains the text (case-insensitive) are shown
```

```gherkin
Given the command palette is visible
When the user presses "j", "k", Up, or Down
Then the palette selection moves accordingly with wraparound
```

```gherkin
Given the command palette is visible with a selection
When the user presses Enter and the buffer is empty
Then the selected command name is filled into the buffer
And the command line stays open
```

### Available Commands

| Command | Description |
|---------|-------------|
| `:help` | Push Help view |
| `:quit` | Quit the application |
| `:refresh` | Refresh all repository statuses |
| `:repo <name>` | Jump to RepoDetail for named or indexed repository |
| `:run <cmd> [args]` | Execute a graft command in the current repository |
| `:state` | Refresh state queries for the current repository |

```gherkin
Given the user enters ":help" in the command line
When Enter is pressed
Then the Help view is pushed onto the view stack
```

```gherkin
Given the user enters ":quit" in the command line
When Enter is pressed
Then the application quits
```

```gherkin
Given the user enters ":refresh" in the command line
When Enter is pressed
Then all repository statuses are refreshed
```

```gherkin
Given the user enters ":repo <name>" in the command line
When Enter is pressed
Then the view stack resets to RepoDetail for the matching repository
And matching is done by case-insensitive substring of path, or by 1-based index
```

```gherkin
Given the user enters ":run <cmd> [args]" in the command line
When Enter is pressed
Then the command is executed in the current repository
And the CommandOutput view is pushed onto the stack
```

```gherkin
Given the user enters an unrecognized command in the command line
When Enter is pressed
Then an error message is shown in the status bar
And the application remains on the current view
```

---

## Constraints

- **Poll interval**: 100ms timeout for key event polling
- **Detail query timeout**: Detail provider uses 5-second git timeout
- **Max commits**: Detail view shows up to 10 recent commits by default
- **Status message timeout**: Status messages auto-clear after 3 seconds
- **Help view minimum size**: 44 columns × 20 rows minimum viable dimensions

## Keybindings Reference

### Dashboard

| Key | Action |
|-----|--------|
| `j`, Down | Move selection down (wraps) |
| `k`, Up | Move selection up (wraps) |
| `Enter`, `Tab`, `x`, `s` | Push RepoDetail view |
| `r` | Refresh all repository statuses |
| `?` | Push Help view |
| `q` | Quit application |
| `Escape` | No-op (already home) |
| `:` | Activate command line |

### RepoDetail View

| Key | Action |
|-----|--------|
| `j`, Down | Scroll detail down |
| `k`, Up | Scroll detail up |
| `n` | Select next command in Commands list |
| `p` | Select previous command in Commands list |
| `Enter` | Open argument input for selected command |
| `r` | Refresh state queries |
| `q`, `Tab` | Pop view (return to Dashboard) |
| `Escape` | Reset to Dashboard |
| `:` | Activate command line |

### Help View

| Key | Action |
|-----|--------|
| `q` | Pop view (return to previous) |
| `Escape` | Reset to Dashboard |
| `:` | Activate command line |

### CommandOutput View

| Key | Action |
|-----|--------|
| `j`, Down | Scroll output down |
| `k`, Up | Scroll output up |
| `q` (command running) | Show stop confirmation |
| `Escape` (command running) | Show stop confirmation |
| `q` (command finished) | Pop view |
| `Escape` (command finished) | Reset to Dashboard |
| `y` | Confirm stop (terminate command) |
| `n`, `Escape` | Cancel stop, continue command |
| `:` | Activate command line |

### Command Line (active)

| Key | Action |
|-----|--------|
| `Escape` | Dismiss command line |
| `Enter` | Submit command (or fill palette entry if buffer empty) |
| `j`, Down | Navigate palette down |
| `k`, Up | Navigate palette up |
| Printable chars (not `j`/`k`) | Insert at cursor |
| `Backspace` | Delete character before cursor |
| `Left`, `Right` | Move cursor |
| `Home`, `End` | Jump to start/end |

### Argument Input Overlay (active)

| Key | Action |
|-----|--------|
| Printable chars | Insert at cursor |
| `Backspace` | Delete character before cursor |
| `Left`, `Right` | Move cursor |
| `Home`, `End` | Jump to start/end |
| `Enter` | Execute command (if parsing valid) |
| `Escape` | Cancel, return to previous view |

## Open Questions

**High Priority (UX critical):**
- [ ] How should the TUI behave when terminal width is very narrow (<80 cols)?
- [ ] Should there be a `/` search command line variant for text search across repos?

**Medium Priority (Feature requests):**
- [ ] Should the Dashboard show last-commit subject or recency info per repo (full-width potential)?
- [ ] Should the repo list display tags from workspace.yaml config?
- [ ] Should path abbreviation be configurable (number of full components, abbreviation length)?

**Low Priority (Nice to have):**
- [ ] Should the RepoDetail header show the repository path in addition to branch?
- [ ] Should `j`/`k` also be usable as literal characters in the command line (via some escape)?

## Decisions

- **2026-02-18**: View stack replaces split-pane layout
  - Each view occupies the full terminal width
  - `q` pops, `Escape` resets to Dashboard — simple mental model
  - No tabs, no split panes, no persistent side columns
  - Eliminates path-abbreviation pressure (more columns available for repo lines)
  - Enables future views (search results, session lists) without layout complexity

- **2026-02-18**: `:` command line as vim-style overlay
  - Summoned on demand, one-shot, dismissed on Enter/Esc
  - Content area stays fully visible above the command line
  - Supplements keybindings — common actions keep their single-key shortcuts
  - Command palette shown when buffer is empty/partial for discoverability
  - `j`/`k` reserved for palette navigation when command line is active

- **2026-02-18**: State queries integrated into RepoDetail view
  - No longer a separate overlay — shown as a section in the unified scrollable view
  - Removes the `s` key overlay pattern; state is always visible in RepoDetail
  - Refresh via `r` key (same as status refresh shortcut)

- **2026-02-18**: RepoDetail is a single scrollable view (no tabs)
  - Changed files, commits, state queries, and commands all visible at once
  - Tab-switching keys (1/2/3) removed — no longer needed at full width
  - Consistent with "everything is a view" architecture

- **2026-02-11**: Workspace name shown in title bar
  - Helps users distinguish which workspace config they're viewing
  - Format: "Grove: {workspace-name} (↑↓/jk navigate, ?help)"
  - Status messages temporarily replace hint text (e.g., "Refreshing...")

- **2026-02-11**: `r` key for manual refresh
  - Users can update status without restarting Grove
  - Shows "Refreshing..." message immediately (UI renders before blocking)
  - Shows "Refreshed N repositories" confirmation on success
  - Shows error message on failure with details
  - Clears detail cache to force re-query on next selection
  - Preferred over auto-refresh timer (explicit control, no background work)

- **2026-02-11**: Empty workspace shows helpful message
  - Displays "No repositories configured" with setup instructions
  - Includes example workspace.yaml snippet
  - Selection set to None (prevents index out of bounds)
  - Navigation keys remain safe (no-op when empty)
  - Prevents confusing blank screen on first run

- **2026-02-11**: Status message auto-clear
  - Status messages (refresh confirmation, errors) expire after 3 seconds
  - Uses timestamp-based approach (message paired with Instant)
  - Auto-clear checked during render cycle (no separate timer thread)
  - Provides feedback without cluttering UI permanently

- **2026-02-11**: Initial load indicator
  - Console shows "Loading N repositories..." before TUI launches
  - Console shows "✓ Loaded N repositories" when ready
  - Provides feedback during initial refresh (1-2 second startup)
  - Prevents impression that app has hung

- **2026-02-11**: Path compaction for long repository paths
  - Home directory shown as `~` (e.g., `/home/user` → `~`)
  - Parent directory components abbreviated to first character (fish-style)
  - Final 2 components shown in full (preserves project/submodule names)
  - Fallback to prefix truncation with `[..]` if still too wide
  - Uses unicode-aware width calculation for international characters
  - Example: `/home/user/very/long/nested/project-name` → `~/v/l/n/project-name`

- **2026-02-11**: Adaptive display prioritizes repository name over branch in tight spaces
  - Overhead calculated dynamically based on actual status width (not fixed)
  - Tiered display strategy based on available width:
    - Very tight (< 15 cols): basename only (e.g., `graft ●`)
    - Tight: compacted path without branch (e.g., `~/src/graft ●`)
    - Normal: full path with branch (e.g., `~/src/graft [main] ●`)
  - Repository name takes priority over branch name (branch visible in RepoDetail)
  - Ensures users can identify repos even in extremely constrained layouts
  - Uses unicode-aware width calculation (not byte count) for threshold checks

- **2026-02-10**: Cache detail by selection index
  - Avoids re-querying git on every render cycle
  - Simple invalidation: cache is cleared when selection changes
  - Scroll resets to 0 on selection change (natural UX)

- **2026-02-10**: Separate RepoDetailProvider trait
  - Decouples detail fetching from registry (single responsibility)
  - Enables testing with mock providers
  - Allows different timeout and error handling strategies

## Sources

- [Workspace UI Exploration (2026-02-06)](../../../notes/2026-02-06-workspace-ui-exploration.md) — Original TUI design, departure board concept
- [Grove Vertical Slices (2026-02-06)](../../../notes/2026-02-06-grove-vertical-slices.md) — Slice 1 and Slice 2 scope definition
- [Grove Command Line and View Stack (2026-02-18)](../../../notes/2026-02-18-grove-command-prompt-exploration.md) — View stack + command line design decisions
