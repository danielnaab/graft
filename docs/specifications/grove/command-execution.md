---
status: working
last-verified: 2026-02-12
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

```gherkin
Given a repository is selected with available commands
When the user presses "x"
Then a command picker overlay appears
And shows all available commands with descriptions
And the first command is selected
```

```gherkin
Given the command picker is displayed
When the user presses "j" or Down
Then the selection moves to the next command
```

```gherkin
Given the command picker is displayed
When the user presses "k" or Up
Then the selection moves to the previous command
```

```gherkin
Given the command picker is displayed
When the user presses "q" or Esc
Then the command picker closes
And no command is executed
```

```gherkin
Given a command is selected in the picker
When the user presses Enter
Then the selected command is executed
And the command picker closes
And an output pane appears showing execution
```

```gherkin
Given a repository has no commands defined
When the user presses "x"
Then a message is shown: "No commands defined in graft.yaml"
And the command picker does not appear
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
Then it is streamed to the output pane in real-time
And the user can scroll through output with j/k
```

```gherkin
Given a command is executing
When the user presses "q"
Then a confirmation prompt appears
And asks: "Stop running command?"
```

```gherkin
Given a command is executing and user confirms stop
When the user presses "y" on the confirmation prompt
Then the subprocess is terminated
And the output pane closes
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
And the output pane remains visible
And the user can press "q" to close it
```

```gherkin
Given a command fails (exit code non-zero)
When execution finishes
Then "✗ Command failed with exit code N" is shown
And the output pane remains visible with full output
And the user can press "q" to close it
```

### Output Pane [Slice 7]

```gherkin
Given the output pane is displayed
When output exceeds the visible height
Then the user can scroll with "j" (down) and "k" (up)
```

```gherkin
Given the output pane is displayed
When the user scrolls down
Then scroll is clamped at the end of output
```

```gherkin
Given the output pane is displayed
When the user scrolls up
Then scroll is clamped at the beginning (line 0)
```

```gherkin
Given a command has finished executing
When the user presses "q"
Then the output pane closes
And focus returns to the repository list
```

```gherkin
Given command output exceeds 10,000 lines
When rendering the output pane
Then only the most recent 10,000 lines are retained
And a message indicates "Output limited to last 10,000 lines"
```

## Edge Cases

### No graft.yaml

```gherkin
Given a repository without graft.yaml
When the user presses "x"
Then a message is shown: "No graft.yaml found"
And no command picker appears
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

| Key | Context | Action |
|-----|---------|--------|
| x | Repo list focused | Open command picker for selected repo |
| j, Down | Command picker | Move selection down |
| k, Up | Command picker | Move selection up |
| Enter | Command picker | Execute selected command |
| q, Esc | Command picker | Close picker without executing |
| j, Down | Output pane | Scroll output down |
| k, Up | Output pane | Scroll output up |
| q | Output pane (command running) | Show stop confirmation |
| q | Output pane (command finished) | Close output pane |
| y | Stop confirmation | Confirm stop, terminate command |
| n, Esc | Stop confirmation | Cancel stop, continue command |

## Open Questions

**High Priority:**
- [ ] Should command arguments be supported (prompt for args before execution)?
- [ ] Should there be a keybinding to re-run the last command?

**Medium Priority:**
- [ ] Should workspace.yaml support workspace-wide commands (available in all repos)?
- [ ] Should command output be colorized (ANSI escape code support)?

**Low Priority:**
- [ ] Should execution history be saved (list of past command runs)?
- [ ] Should there be a "run in all repos" option?

## Decisions

- **2026-02-12**: Use `x` key for command execution
  - `r` already used for refresh
  - `x` is mnemonic for "execute"
  - Common in vim-style UIs

- **2026-02-12**: Commands executed via `graft run` subprocess
  - Reuses graft's command execution logic
  - No need to duplicate command parsing, env vars, working_dir handling
  - Subprocess approach keeps Grove and graft loosely coupled

- **2026-02-12**: Modal UI for command picker and output
  - Command picker is full-screen overlay (like help)
  - Output pane replaces detail pane temporarily
  - Prevents UI complexity of multiple simultaneous panes

- **2026-02-12**: Manual stop only (no automatic timeout)
  - Long-running commands (builds, tests) should not be killed arbitrarily
  - User has explicit control with "q" → "y" confirmation
  - Prevents accidental termination

- **2026-02-12**: 10,000 line output buffer limit
  - Prevents memory exhaustion from verbose commands
  - 10,000 lines ≈ 1MB of output (reasonable for terminal display)
  - Notify user when limit reached

## Sources

- [Grove Vertical Slices (2026-02-06)](../../../notes/2026-02-06-grove-vertical-slices.md) — Slice 7 scope definition
- [Graft Core Operations Spec](../../graft/core-operations.md) — `graft run` command specification
