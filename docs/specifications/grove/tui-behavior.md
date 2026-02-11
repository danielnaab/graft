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
And the detail pane shows "No repository selected"
And navigation keys (j/k) do not panic
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

## Constraints

- **Poll interval**: 100ms timeout for key event polling
- **Detail query timeout**: Detail provider uses 5-second git timeout
- **Max commits**: Detail pane shows up to 10 recent commits by default

## Keybindings Reference

| Key | Context | Action |
|-----|---------|--------|
| j, Down | Repo list focused | Move selection down |
| k, Up | Repo list focused | Move selection up |
| Enter, Tab | Repo list focused | Switch focus to detail pane |
| q, Esc | Repo list focused | Quit application |
| j, Down | Detail pane focused | Scroll detail down |
| k, Up | Detail pane focused | Scroll detail up |
| q, Esc, Enter, Tab | Detail pane focused | Return focus to repo list |

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

## Sources

- [Workspace UI Exploration (2026-02-06)](../../../notes/2026-02-06-workspace-ui-exploration.md) — Original TUI design, departure board concept
- [Grove Vertical Slices (2026-02-06)](../../../notes/2026-02-06-grove-vertical-slices.md) — Slice 1 and Slice 2 scope definition
