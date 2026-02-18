---
status: working
last-verified: 2026-02-18
owners: [human, agent]
---

# Command Execution

## Intent

Define how Grove discovers, presents, and executes commands from repository graft.yaml files. Commands are the primary way users run tasks (tests, builds, migrations) within repositories from the Grove TUI.

## Non-goals

- **Not a general shell** — Users can't type arbitrary commands; only pre-configured commands from graft.yaml
- **Not interactive commands** — Commands requiring stdin are out of scope
- **Not concurrent execution** — One command at a time
- **Not execution history** — No persistence of past runs across sessions

## Behavior

### Command Discovery [Slice 7]

```gherkin
Given a repository is selected in the list
When Grove loads the repository's graft.yaml
Then it parses the `commands` section
And discovers all available command names and descriptions
```

```gherkin
Given a repository does not have a graft.yaml file
When the user attempts to execute a command
Then no commands are available
And a helpful message is shown
```

```gherkin
Given a repository's graft.yaml has invalid syntax
When Grove attempts to parse commands
Then an error is shown to the user
And command execution is unavailable for that repository
```

### Command Picker UI [Slice 7]

The **Commands** section is part of the unified RepoDetail view. Commands are not shown in a separate overlay — they appear inline in the scrollable detail. Selecting and executing a command from the RepoDetail view triggers the argument input dialog.

```gherkin
Given a repository is selected and the RepoDetail view is open
Then the "Commands" section shows all available commands with descriptions
And the first command is highlighted by default
```

```gherkin
Given the RepoDetail view is current
When the user presses "n"
Then the selection moves to the next command
```

```gherkin
Given the RepoDetail view is current
When the user presses "p"
Then the selection moves to the previous command
```

```gherkin
Given a command is highlighted in the Commands section
When the user presses Enter
Then an argument input dialog appears for that command
```

```gherkin
Given a repository has no commands defined
When the RepoDetail view renders
Then the Commands section shows "No commands defined in graft.yaml"
```

### Command Line Execution (`:run`) [Phase 2]

In addition to the `x` → picker → args dialog path, commands can be executed directly via the `:` command line. This is a faster path for users who know the command name.

```gherkin
Given any view is current
When the user enters ":run <cmd>" in the command line and presses Enter
Then the named command is executed in the current repository
And the CommandOutput view is pushed onto the stack
```

```gherkin
Given the RepoDetail view is current for repository R
When the user enters ":run <cmd> [args]" in the command line
Then the command is executed in repository R's directory
```

```gherkin
Given the Dashboard is current with repository R selected
When the user enters ":run <cmd> [args]" in the command line
Then the command is executed in the selected repository R's directory
```

```gherkin
Given no repository is in context (empty workspace or no selection)
When the user enters ":run <cmd>" in the command line
Then an error message is shown: "No repository selected"
And no command is executed
```

Both the `x` picker path and the `:run` command line path use the same underlying execution logic — they are two UI surfaces for the same action.

### Argument Input [Implemented 2026-02-13]

The argument input dialog is an **overlay** over the current view (not a view itself). It appears after selecting a command via the `x` picker.

```gherkin
Given a command has been selected for execution
When the command picker closes
Then an argument input dialog appears
And shows "Arguments for '<command-name>'"
And allows the user to enter arguments or press Enter to skip
```

```gherkin
Given the argument input dialog is displayed
When the user types characters
Then they are inserted at the cursor position
And the cursor advances
And displayed in the input field with a cursor indicator (▊ in middle, _ at end)
```

```gherkin
Given the argument input dialog is displayed
When the user presses Backspace
Then the character before the cursor is removed
And the cursor moves backward
```

```gherkin
Given the argument input dialog is displayed
When the user presses Left/Right arrow keys
Then the cursor moves in that direction
And stops at buffer boundaries (0 and length)
```

```gherkin
Given the argument input dialog is displayed
When the user presses Home/End keys
Then the cursor jumps to start/end of buffer
```

```gherkin
Given the argument input dialog is displayed
When the user types or edits arguments
Then a preview line shows how the command will be executed
And shows parsed arguments with proper quoting
And appears in green for valid parsing, red for errors
```

```gherkin
Given the argument input dialog is displayed
When the user presses Enter with valid arguments
Then arguments are parsed using shell-style syntax (respecting quotes)
And Grove calls `graft run <command-name> <arg1> <arg2> ...`
And the argument dialog closes
And the CommandOutput view is pushed onto the stack
```

```gherkin
Given the argument input dialog is displayed
When the user presses Enter with invalid arguments (e.g., unmatched quote)
Then an error message is shown in the status bar
And the dialog remains open
And the command is NOT executed
```

```gherkin
Given the user wants to pass an argument with spaces
When they enter: Personal "This is a test"
Then the preview shows: graft run capture Personal 'This is a test'
And it is parsed as 2 arguments: ["Personal", "This is a test"]
```

```gherkin
Given the argument input dialog is displayed with empty buffer
When the user presses Enter
Then Grove calls `graft run <command-name>` without arguments
And execution proceeds normally
```

```gherkin
Given the argument input dialog is displayed
When the user presses Esc
Then the dialog closes without executing
And focus returns to the current view (not necessarily the Dashboard)
And the command is not executed
```

### Command Execution [Slice 7]

```gherkin
Given a command has been selected for execution
When execution begins
Then Grove calls `graft run <command-name>` as a subprocess
And the repository's directory is used as the working directory
And stdout and stderr are captured
```

```gherkin
Given a command is executing
When output is produced
Then it is streamed to the CommandOutput view in real-time
And the user can scroll through output with j/k
```

```gherkin
Given a command is executing
When the user presses "q" or Escape
Then a confirmation prompt appears
And asks: "Stop running command?"
```

```gherkin
Given a command is executing and user confirms stop
When the user presses "y" on the confirmation prompt
Then the subprocess is terminated
And the CommandOutput view is popped (previous view restored)
```

```gherkin
Given a command is executing and user cancels stop
When the user presses "n" or Esc on the confirmation prompt
Then the confirmation closes
And the command continues executing
```

```gherkin
Given a command completes successfully (exit code 0)
When execution finishes
Then "✓ Command completed successfully" is shown
And the CommandOutput view remains visible
And the user can press "q" to close it (pop view)
And the user can press "Escape" to reset to Dashboard
```

```gherkin
Given a command fails (exit code non-zero)
When execution finishes
Then "✗ Command failed with exit code N" is shown
And the CommandOutput view remains visible with full output
And the user can press "q" to close it (pop view)
And the user can press "Escape" to reset to Dashboard
```

### CommandOutput View [Slice 7]

The CommandOutput view is a full-width view on the view stack (not an overlay or side pane). It replaces the previous view's content area while the command runs.

```gherkin
Given the CommandOutput view is current
When output exceeds the visible height
Then the user can scroll with "j" (down) and "k" (up)
```

```gherkin
Given the CommandOutput view is current
When the user scrolls down
Then scroll is clamped at the end of output
```

```gherkin
Given the CommandOutput view is current
When the user scrolls up
Then scroll is clamped at the beginning (line 0)
```

```gherkin
Given a command has finished executing
When the user presses "q"
Then the CommandOutput view is popped
And the previous view is shown (e.g., RepoDetail or Dashboard)
```

```gherkin
Given a command has finished executing
When the user presses Escape
Then the view stack resets to Dashboard
```

```gherkin
Given command output exceeds 10,000 lines
When rendering the CommandOutput view
Then only the most recent 10,000 lines are retained
And a message indicates "Output limited to last 10,000 lines"
```

### Navigation from CommandOutput

```gherkin
Given the CommandOutput view is on the stack preceded by RepoDetail
When the user presses "q" (command finished)
Then the stack pops to [Dashboard, RepoDetail]
And the user can press "q" again to return to Dashboard
```

```gherkin
Given the CommandOutput view is on the stack
When the user presses Escape (command finished)
Then the stack resets to [Dashboard]
```

## Edge Cases

### No graft.yaml

```gherkin
Given a repository without graft.yaml
When the RepoDetail view renders
Then the Commands section shows "No graft.yaml found"
And the user can still see changes, commits, and state queries
```

### graft not in PATH

```gherkin
Given the `graft` command is not available in PATH
When Grove attempts to execute a command
Then an error is shown: "graft command not found"
And suggests installing graft or checking PATH
```

### Command execution timeout

```gherkin
Given a command runs for more than 5 minutes
When the timeout is reached
Then Grove does NOT automatically kill the command
And continues streaming output
And the user can manually stop with "q" → "y"
```

## Constraints

- **Output buffer**: Maximum 10,000 lines retained in memory
- **No timeout**: Commands can run indefinitely (user can stop manually)
- **Subprocess**: Uses `std::process::Command` with piped stdout/stderr
- **Working directory**: Command executed in repository root directory

## Keybindings

### Command Selection in RepoDetail

| Key | Context | Action |
|-----|---------|--------|
| `n` | RepoDetail (Commands section) | Select next command |
| `p` | RepoDetail (Commands section) | Select previous command |
| `Enter` | RepoDetail (command highlighted) | Open argument input dialog |

### Argument Input Overlay

| Key | Context | Action |
|-----|---------|--------|
| Char | Argument input | Insert character at cursor position |
| `Backspace` | Argument input | Delete character before cursor |
| `Left`, `Right` | Argument input | Move cursor backward/forward |
| `Home`, `End` | Argument input | Jump to start/end of input |
| `Enter` | Argument input | Execute command (if parsing valid) |
| `Esc` | Argument input | Cancel and return to current view |

### CommandOutput View

| Key | Context | Action |
|-----|---------|--------|
| `j`, Down | CommandOutput | Scroll output down |
| `k`, Up | CommandOutput | Scroll output up |
| `q`, `Esc` | CommandOutput (command running) | Show stop confirmation |
| `q` | CommandOutput (command finished) | Pop view (return to previous) |
| `Esc` | CommandOutput (command finished) | Reset to Dashboard |
| `y` | Stop confirmation | Confirm stop, terminate command |
| `n`, `Esc` | Stop confirmation | Cancel stop, continue command |

### Command Line (`:run`)

| Key | Context | Action |
|-----|---------|--------|
| `:` | Any view | Activate command line |
| `:run <cmd> [args]` + `Enter` | Command line | Execute named command directly |

See [TUI Behavior](tui-behavior.md) for the full command line keybinding reference.

## Open Questions

**High Priority:**
- [x] Command arguments supported via text input dialog (implemented 2026-02-13)
- [ ] Should there be a keybinding to re-run the last command?

**Medium Priority:**
- [ ] Should workspace.yaml support workspace-wide commands (available in all repos)?
- [ ] Should command output be colorized (ANSI escape code support)?

**Low Priority:**
- [ ] Should execution history be saved (list of past command runs)?
- [ ] Should there be a "run in all repos" option?

## Decisions

- **2026-02-18**: CommandOutput is a view on the stack, not an overlay
  - Follows the "everything is a view" architecture from the view stack design
  - `q` pops back to the previous view (RepoDetail or Dashboard)
  - `Escape` resets to Dashboard (consistent with all views)
  - Stop confirmation gates both `q` and `Escape` while command is running

- **2026-02-18**: `:run` command line as direct execution path
  - Supplements the `x` → picker → args dialog path
  - Faster for power users who know the command name
  - Both paths use the same underlying execution logic
  - Command line path skips the picker and dialog entirely

- **2026-02-12**: Use `x` key for command execution
  - `r` already used for refresh
  - `x` is mnemonic for "execute"
  - Common in vim-style UIs

- **2026-02-12**: Commands executed via `graft run` subprocess
  - Reuses graft's command execution logic
  - No need to duplicate command parsing, env vars, working_dir handling
  - Subprocess approach keeps Grove and graft loosely coupled

- **2026-02-12**: Manual stop only (no automatic timeout)
  - Long-running commands (builds, tests) should not be killed arbitrarily
  - User has explicit control with "q" → "y" confirmation
  - Prevents accidental termination

- **2026-02-12**: 10,000 line output buffer limit
  - Prevents memory exhaustion from verbose commands
  - 10,000 lines ≈ 1MB of output (reasonable for terminal display)
  - Notify user when limit reached

- **2026-02-13**: Command arguments via text input dialog
  - Modal dialog appears after command selection
  - Simple text buffer with Char/Backspace handling
  - Arguments parsed using `shell-words` crate (respects quotes, escapes)
  - Empty input allowed (skip arguments)
  - Supports quoted arguments for strings with spaces: `arg1 "arg with spaces"`
  - Consistent with existing modal UI pattern

- **2026-02-13**: Phase 1 UX improvements
  - **Cursor navigation**: Left/Right arrows, Home/End keys for editing anywhere in buffer
  - **Visual cursor**: Shows ▊ in middle of text, _ at end
  - **Command preview**: Real-time preview of parsed command (green=valid, red=error)
  - **Parse validation**: Blocks execution on parsing errors, shows error in status bar
  - **Refactored state**: Introduced `ArgumentInputState` struct to encapsulate buffer + cursor + command
  - **Dialog size**: Increased to 70 chars wide × 9 lines tall (was 60×7) for preview
  - **Test coverage**: Added 7 new tests (cursor nav, insertion at position, error blocking)

## Sources

- [Grove Vertical Slices (2026-02-06)](../../../notes/2026-02-06-grove-vertical-slices.md) — Slice 7 scope definition
- [Graft Core Operations Spec](../../graft/core-operations.md) — `graft run` command specification
- [Grove Command Line and View Stack (2026-02-18)](../../../notes/2026-02-18-grove-command-prompt-exploration.md) — Command line design, `:run` direct execution path
