---
status: working
last-verified: 2026-02-10
owners: [human, agent]
---

# TUI Behavior

## Intent

Define the interactive terminal interface for Grove — how users navigate repositories, view status, inspect detail, and manage focus across panes. This spec covers the ratatui-based TUI that serves as Grove's primary user interface.

## Non-goals

- **Not a design system** — Colors and typography are implementation choices, not specified here
- **Not a styling spec** — Visual polish (spacing, alignment) is left to the implementation
- **Not an accessibility spec** — Screen reader support and contrast requirements are future concerns
- **Not the CLI interface** — Machine-readable `--json` output is a separate concern

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

### List Navigation [Slice 1]

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
Given the repo list is focused
When the user presses "q" or Esc
Then the application quits
```

```gherkin
Given the TUI has just launched with repositories loaded
When the initial render occurs
Then the first repository in the list is selected
```

The selected item is highlighted with a "▶" prefix.

### Help Overlay [Slice 1]

```gherkin
Given the repo list is focused
When the user presses "?"
Then a help overlay appears centered on screen
And shows all keybindings with descriptions
And shows status indicator legend
And shows Grove version
```

```gherkin
Given the help overlay is displayed
When the user presses a printable key (letter, number, etc.) or Esc or Enter
Then the help overlay closes
And focus returns to the repo list
```

```gherkin
Given the help overlay is displayed
When the user presses a control key (Ctrl+C, Ctrl+Z, etc.)
Then the help overlay remains open (control keys are ignored)
```

```gherkin
Given the terminal size is smaller than 44 columns or 20 rows
When the help overlay would be rendered
Then the help overlay adjusts to minimum viable dimensions
And content remains readable
```

### Manual Refresh [Slice 1]

```gherkin
Given the repo list is focused
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

### Split-Pane Layout [Slice 2]

```gherkin
Given the TUI has launched
When the layout is rendered
Then the screen is split horizontally: repo list (left), detail pane (right)
```

```gherkin
Given a pane is focused
When the layout is rendered
Then the focused pane has a distinct border style from the unfocused pane
```

```gherkin
Given the TUI has just launched
When the initial render occurs
Then the repo list pane is focused
```

### Focus Management [Slice 2]

```gherkin
Given the repo list is focused
When the user presses Enter or Tab
Then focus switches to the detail pane
```

```gherkin
Given the detail pane is focused
When the user presses "q", Esc, Enter, or Tab
Then focus returns to the repo list
And the application does NOT quit
```

```gherkin
Given focus has returned to the repo list from the detail pane
When the user presses "q" or Esc
Then the application quits (normal repo-list quit behavior)
```

### Detail Pane Content [Slice 2]

The detail pane shows information about the currently selected repository.

```gherkin
Given a repository is selected with branch "main", dirty, 2 ahead, 1 behind
When the detail pane renders
Then the header shows: branch name, dirty indicator ("●" or "○"), ahead count ("↑2"), behind count ("↓1")
```

```gherkin
Given the selected repository has 2 uncommitted changes
When the detail pane renders
Then it shows "Changed Files (2)" as a section header
And each file shows a status indicator: M (modified), A (added/staged), D (deleted), R (renamed), C (copied), ? (untracked/unknown)
And each file shows its path
```

```gherkin
Given the selected repository has recent commits
When the detail pane renders
Then it shows "Recent Commits (<count>)" as a section header
And each commit shows: abbreviated hash, subject
And below each commit: author and relative date
```

```gherkin
Given no repository is selected (empty workspace)
When the detail pane renders
Then it shows "No repository selected"
```

```gherkin
Given the selected repository has no uncommitted changes
When the detail pane renders
Then the changed files section shows "No uncommitted changes"
```

```gherkin
Given the selected repository has no commits
When the detail pane renders
Then the commits section shows "No commits"
```

```gherkin
Given a detail query returned a partial error (e.g., changed files timed out but commits succeeded)
When the detail pane renders
Then it shows "Error: <message>" as a warning at the top
And still renders the remaining available data below the error
```

### Detail Scroll [Slice 2]

```gherkin
Given the detail pane is focused
When the user presses "j" or Down arrow
Then the detail content scrolls down by one line
```

```gherkin
Given the detail pane is focused
When the user presses "k" or Up arrow
Then the detail content scrolls up by one line (minimum 0)
```

```gherkin
Given the detail content is shorter than the pane height
When the user scrolls down
Then scroll is clamped — cannot scroll past the end of content
```

```gherkin
Given the user navigates to a different repository in the list
When the detail pane updates
Then scroll position resets to 0
```

### Detail Query Behavior [Slice 2]

```gherkin
Given a repository is selected and its detail has been loaded
When the detail pane renders again without changing selection
Then the cached detail is reused (no re-query)
```

```gherkin
Given the user navigates to a different repository
When the detail pane renders
Then a fresh detail query is made for the new selection
And the result is cached for reuse
```

```gherkin
Given a detail provider returns an error
When the detail is loaded
Then an error state with the error message is cached (not a panic)
And the detail pane renders the error state
```

### Edge Cases

#### Empty workspace

```gherkin
Given a workspace with no repositories
When the TUI launches
Then the repo list is empty
And no item is selected (selection is None)
And the detail pane shows "No repository selected"
And navigation keys (j/k) do not panic
And selection remains None when navigation keys are pressed
```

#### Single repository

```gherkin
Given a workspace with exactly one repository
When the user presses "j" (down) or "k" (up)
Then the selection stays on the same item (wrapping to self)
```

#### Provider failure isolation

```gherkin
Given the detail provider fails for a selected repository
When the detail pane attempts to render
Then the detail pane shows the error message
And the repo list continues to function normally
And navigation between repos still works
```

### State Panel [Phase 1]

The state panel provides an overlay view of state queries defined in a repository's graft.yaml file.

#### Opening State Panel from Detail View

```gherkin
Given the user is viewing repository detail
And the repository has a graft.yaml file
When the user presses 's'
Then the state panel overlay appears
And the state queries are discovered from graft.yaml
And the cached results are loaded for each query
```

```gherkin
Given the user is viewing repository detail
And the repository has no graft.yaml file
When the user presses 's'
Then the state panel overlay appears
And shows an empty state message
```

```gherkin
Given the user is viewing repository detail
And the graft.yaml file has invalid YAML syntax
When the user presses 's'
Then an error message is shown in the status bar
And the state panel shows empty state
And the error is logged
```

#### Navigating State Panel

```gherkin
Given the state panel is open with multiple queries
And the first query is selected
When the user presses 'j' or Down arrow
Then the selection moves to the next query
```

```gherkin
Given the state panel is open with multiple queries
And the second query is selected
When the user presses 'k' or Up arrow
Then the selection moves to the previous query
```

```gherkin
Given the state panel is open
And the last query is selected
When the user presses 'j' or Down
Then the selection remains on the last query
```

```gherkin
Given the state panel is open
And the first query is selected
When the user presses 'k' or Up
Then the selection remains on the first query
```

#### State Panel Display

```gherkin
Given the state panel is open
And a query has cached results
Then the query name is displayed
And a summary of the data is shown (e.g., "5000 words total, 250 today")
And the cache age is displayed (e.g., "5m ago", "2h ago")
```

```gherkin
Given the state panel is open
And a query has no cached results
Then the query name is displayed
And "(no cached data)" is shown
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

#### Closing State Panel

```gherkin
Given the state panel is open
When the user presses Esc
Then the panel closes
And the detail view is shown
And the state query data is cleared
```

```gherkin
Given the state panel is open
When the user presses 'q'
Then the panel closes
And the detail view is shown
And the application does not quit
```

#### Refreshing State Queries

```gherkin
Given the state panel is open
And a query is selected
When the user presses 'r'
Then the query is re-executed via graft CLI
And the status bar shows "Refreshing <query>..."
And the cache is updated with fresh results
And the panel displays the updated data
And a success message is shown
```

```gherkin
Given the user presses 'r' to refresh a query
And the graft command fails
Then an error message is shown in the status bar
And the error details are logged
And the cached data remains unchanged
```

```gherkin
Given the user presses 'r' to refresh a query
And graft is not installed
Then an error message is shown: "Failed to run graft command. Is graft installed?"
And the cached data remains unchanged
```

```gherkin
Given the state panel shows cached results
Then each query displays its cache age (e.g., "5m ago", "2h ago", "3d ago")
And users can see data freshness at a glance
And can decide which queries need refreshing
```

#### Error Handling

```gherkin
Given the state panel is open
And all queries have no cached results
Then an info message is shown: "No cached state data found. Run 'graft state query <name>' to populate cache."
```

```gherkin
Given the user opens the state panel
And the graft.yaml cannot be parsed
Then an error message is shown in the status bar
And the error details are logged
And the panel shows empty state
```

## Constraints

- **Poll interval**: 100ms timeout for key event polling
- **Detail query timeout**: Detail provider uses 5-second git timeout
- **Max commits**: Detail pane shows up to 10 recent commits by default
- **Status message timeout**: Status messages auto-clear after 3 seconds
- **Help overlay minimum size**: 44 columns × 20 rows minimum viable dimensions

## Keybindings Reference

| Key | Context | Action |
|-----|---------|--------|
| j, Down | Repo list focused | Move selection down |
| k, Up | Repo list focused | Move selection up |
| Enter, Tab | Repo list focused | Switch focus to detail pane |
| r | Repo list focused | Manually refresh repository status |
| ? | Repo list focused | Show help overlay |
| q, Esc | Repo list focused | Quit application |
| j, Down | Detail pane focused | Scroll detail down |
| k, Up | Detail pane focused | Scroll detail up |
| s | Detail pane focused | Open state queries panel |
| q, Esc, Enter, Tab | Detail pane focused | Return focus to repo list |
| j, Down | State panel active | Select next query |
| k, Up | State panel active | Select previous query |
| r | State panel active | Refresh selected query |
| q, Esc | State panel active | Close panel and return to detail |
| Printable keys, Esc, Enter | Help overlay active | Close help and return to repo list |

## Open Questions

**High Priority (UX critical):**
- [ ] How should the TUI behave when terminal width is very narrow (<80 cols)?
- [ ] Should there be a keybinding to manually refresh status (beyond app restart)?

**Medium Priority (Feature requests):**
- [ ] Should the repo list display tags from workspace.yaml config?
- [ ] Should there be visual indication when detail is loading/refreshing?
- [ ] Should path abbreviation be configurable (number of full components, abbreviation length)?

**Low Priority (Nice to have):**
- [ ] Should the detail pane header show the repository path in addition to branch?

## Decisions

- **2026-02-11**: Workspace name shown in title bar
  - Helps users distinguish which workspace config they're viewing
  - Format: "Grove: {workspace-name} (↑↓/jk navigate, ?help)"
  - Status messages temporarily replace hint text (e.g., "Refreshing...")

- **2026-02-11**: `?` key shows help overlay
  - Centered modal overlay with all keybindings and status legend
  - Shows Grove version for bug reports
  - Printable keys, Esc, and Enter dismiss (control keys ignored to prevent accidental dismissal)
  - Minimum viable size enforced (44x20) for small terminals
  - Preferred over footer hints (which get cut off in narrow terminals)

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

- **2026-02-10**: q/Esc in detail pane returns to list, does not quit
  - Prevents accidental quit when exploring detail
  - Consistent with common TUI patterns (nested focus returns to parent)
  - All four keys (q, Esc, Enter, Tab) return from detail for discoverability

- **2026-02-10**: 40/60 horizontal split for list and detail
  - Gives detail pane enough room for file paths and commit info
  - Repo paths are short enough to fit in 40% in most terminals

- **2026-02-10**: Cache detail by selection index
  - Avoids re-querying git on every render cycle
  - Simple invalidation: cache is cleared when selection changes
  - Scroll resets to 0 on selection change (natural UX)

- **2026-02-10**: Separate RepoDetailProvider trait
  - Decouples detail fetching from registry (single responsibility)
  - Enables testing with mock providers
  - Allows different timeout and error handling strategies

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
  - Repository name takes priority over branch name (branch visible in detail pane)
  - Ensures users can identify repos even in extremely constrained layouts
  - Uses unicode-aware width calculation (not byte count) for threshold checks

- **2026-02-14**: State panel for viewing cached state query results
  - Accessible via 's' key from detail pane
  - Overlays the detail pane as a centered modal (80% width/height)
  - Discovers state queries from graft.yaml in selected repository
  - Shows cached results for each query with smart summary formatting
  - Handles errors gracefully (YAML parse errors, missing cache, etc.)
  - Returns to detail pane with q/Esc (q does NOT quit app from state panel)
  - Query-specific summary formats: writing metrics, task counts, graph stats
  - Generic fallback for unknown query data formats
  - Provides actionable feedback when no cache exists

## Sources

- [Workspace UI Exploration (2026-02-06)](../../../notes/2026-02-06-workspace-ui-exploration.md) — Original TUI design, departure board concept
- [Grove Vertical Slices (2026-02-06)](../../../notes/2026-02-06-grove-vertical-slices.md) — Slice 1 and Slice 2 scope definition
