---
status: stable
date: 2026-02-12
summary: Specification for Grove domain types (Command, CommandState, GraftYaml)
---

# Grove Domain Models

This document specifies the domain types used by Grove for command execution.

## Overview

Grove uses three primary domain types for command execution:
1. **Command** - A single executable command from graft.yaml
2. **GraftYaml** - Parsed representation of graft.yaml file
3. **CommandState** - Execution state tracking in the TUI

---

## Command

Represents an executable command from a repository's graft.yaml file.

### Structure

```rust
pub struct Command {
    pub run: String,
    pub description: Option<String>,
    pub working_dir: Option<String>,
    pub env: Option<HashMap<String, String>>,
}
```

### Fields

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `run` | `String` | Yes | Shell command to execute |
| `description` | `Option<String>` | No | Human-readable command description |
| `working_dir` | `Option<String>` | No | Relative path from repo root |
| `env` | `Option<HashMap<String, String>>` | No | Environment variables |

### Validation Rules

#### `run` field
- **Required**: Must not be empty
- **Type**: String containing shell command
- **Max length**: No explicit limit (practical: 10KB)
- **Security**: Command executed via `graft run`, not directly by Grove
- **Example**: `"pytest tests/"`, `"npm run build"`

#### `description` field
- **Optional**: Can be `None`
- **Type**: Human-readable string
- **Max length**: No explicit limit (practical: 500 characters)
- **Purpose**: Displayed in command picker for user reference
- **Example**: `"Run test suite"`, `"Build production assets"`

#### `working_dir` field
- **Optional**: Can be `None`
- **Type**: Relative path from repository root
- **Constraints**:
  - Must be relative (no leading `/`)
  - No `..` path traversal (security)
  - Path must exist at execution time
- **Default**: Repository root if not specified
- **Example**: `"src/"`, `"packages/web/"`

#### `env` field
- **Optional**: Can be `None`
- **Type**: HashMap of environment variable names to values
- **Constraints**:
  - Keys must be valid environment variable names
  - Values must be valid UTF-8 strings
  - Both keys and values serializable
- **Purpose**: Set environment variables for command execution
- **Example**: `{"NODE_ENV": "production", "DEBUG": "1"}`

### Serialization

Commands are serialized/deserialized from YAML using serde:

```yaml
commands:
  test:
    run: pytest tests/
    description: Run test suite
    working_dir: src/
    env:
      PYTEST_VERBOSE: "1"
      COVERAGE: "true"
```

**Defaults**:
- `description`: `None` if not specified
- `working_dir`: `None` if not specified (uses repo root)
- `env`: `None` if not specified (inherits environment)

### Relationship to Graft

**Grove's Command is a read-only subset of Graft's Command model**:
- Grove **parses** graft.yaml to discover available commands
- Grove **delegates execution** to `graft run <command>`
- Grove **does not modify** or persist commands
- Grove **must match** Graft's Command schema exactly

**Schema Alignment**:
```
Graft Command Schema (authoritative)
    ↓
Grove Command Model (read-only view)
    ↓
Displayed in Grove TUI
    ↓
Executed via `graft run`
```

**References**:
- [Graft Command Specification](../../graft/graft-command.md)
- [Graft YAML Format](../../graft/graft-yaml-format.md)

### Examples

#### Minimal Command
```yaml
test:
  run: npm test
```

Parsed as:
```rust
Command {
    run: "npm test".to_string(),
    description: None,
    working_dir: None,
    env: None,
}
```

#### Full Command
```yaml
build-prod:
  run: npm run build
  description: Build production bundle
  working_dir: packages/web
  env:
    NODE_ENV: production
    OPTIMIZE: "true"
```

Parsed as:
```rust
Command {
    run: "npm run build".to_string(),
    description: Some("Build production bundle".to_string()),
    working_dir: Some("packages/web".to_string()),
    env: Some(HashMap::from([
        ("NODE_ENV".to_string(), "production".to_string()),
        ("OPTIMIZE".to_string(), "true".to_string()),
    ])),
}
```

---

## GraftYaml

Represents the parsed contents of a repository's graft.yaml file (commands section only).

### Structure

```rust
pub struct GraftYaml {
    pub commands: HashMap<String, Command>,
}
```

### Fields

| Field | Type | Description |
|-------|------|-------------|
| `commands` | `HashMap<String, Command>` | Map of command names to Command structs |

### Validation Rules

#### `commands` field
- **Can be empty**: Valid to have zero commands
- **Keys**: Command names (alphanumeric, hyphens, underscores recommended)
- **Values**: Valid Command structs
- **Uniqueness**: Keys must be unique (HashMap enforces)

### Serialization

Minimal graft.yaml representation:

```yaml
apiVersion: graft/v1beta1  # Handled by parser, not stored in GraftYaml
name: my-repo              # Not stored in GraftYaml (minimal representation)
description: My project    # Not stored in GraftYaml

commands:
  test:
    run: npm test
  build:
    run: npm run build
```

**Note**: Grove only stores the `commands` section. Full graft.yaml parsing is handled by `graft run`.

### Default Behavior

Empty graft.yaml:
```yaml
commands: {}
```

Parsed as:
```rust
GraftYaml {
    commands: HashMap::new(),
}
```

**Result**: Command picker shows "No commands defined in graft.yaml"

### Relationship to Graft

**GraftYaml is a minimal view**:
- Grove parses **only the commands section**
- Graft parses **full graft.yaml** (apiVersion, name, description, etc.)
- Grove displays commands in TUI
- Graft executes commands with full context

**Why Minimal**:
- Grove only needs command names and metadata for display
- Execution is delegated to `graft run` which has full context
- Avoids duplicating Graft's YAML parsing logic

### Examples

#### Empty Commands

```yaml
# No commands defined
commands: {}
```

Result: Empty command picker, shows "No commands defined"

#### Multiple Commands

```yaml
commands:
  test:
    run: pytest tests/
  lint:
    run: ruff check src/
  format:
    run: ruff format src/
  build:
    run: uv build
  deploy:
    run: ./scripts/deploy.sh
```

Result: Command picker shows 5 commands

---

## CommandState

Represents the execution state of a command in the TUI.

### States

```rust
pub enum CommandState {
    NotStarted,
    Running,
    Completed { exit_code: i32 },
    Failed { error: String },
}
```

### State Descriptions

| State | Description | Data |
|-------|-------------|------|
| `NotStarted` | Command has not been started yet | None |
| `Running` | Command is currently executing | None |
| `Completed` | Command finished execution | `exit_code: i32` |
| `Failed` | Command failed to start or error occurred | `error: String` |

### State Transitions

```
NotStarted ──┬──> Running ──┬──> Completed { exit_code }
             │               └──> Failed { error }
             └──> Failed { error }
```

**Valid Transitions**:
1. `NotStarted` → `Running` - When command spawned
2. `Running` → `Completed` - When process exits normally
3. `Running` → `Failed` - When execution error occurs
4. `NotStarted` → `Failed` - When spawn fails immediately

**Invalid Transitions**:
- Cannot go from `Completed` or `Failed` back to `Running`
- Cannot skip `Running` state (must go NotStarted → Running → Completed)
- Exception: Can go NotStarted → Failed if spawn fails

### Field Validation

#### `exit_code` field (in Completed state)
- **Type**: `i32`
- **Range**: Any i32 value
- **Success**: `0` indicates success
- **Failure**: Non-zero indicates failure
- **Signals**: Negative values indicate process killed by signal
- **Example**: `0` (success), `1` (error), `-9` (SIGKILL)

#### `error` field (in Failed state)
- **Type**: `String`
- **Content**: Human-readable error message
- **Purpose**: Displayed to user, should include recovery suggestions
- **Example**: `"Failed to spawn: No such file or directory"`
- **Best Practice**: Include context and next steps

### Usage in TUI

**Command Picker** (NotStarted):
```rust
CommandState::NotStarted
// UI shows: "x = Execute command"
```

**CommandOutput View Header** (Running):
```rust
CommandState::Running
// UI shows: "Running: test (j/k: scroll, q: close)"
```

**CommandOutput View Header** (Completed Success):
```rust
CommandState::Completed { exit_code: 0 }
// UI shows: "✓ test: Completed successfully (exit 0) - Press q to close"
```

**CommandOutput View Header** (Completed Failure):
```rust
CommandState::Completed { exit_code: 42 }
// UI shows: "✗ test: Failed with exit code 42 - Press q to close"
```

**CommandOutput View Header** (Failed):
```rust
CommandState::Failed { error: "graft not found".to_string() }
// UI shows: "✗ Failed: graft not found - Press q to close"
```

### Examples

#### Successful Execution Flow

```rust
// Initial state
let mut state = CommandState::NotStarted;

// User presses 'x' and selects command
state = CommandState::Running;

// Command completes successfully
state = CommandState::Completed { exit_code: 0 };

// User presses 'q' to close
state = CommandState::NotStarted; // Reset for next command
```

#### Failed Execution Flow

```rust
// Initial state
let mut state = CommandState::NotStarted;

// User presses 'x', command spawn fails
state = CommandState::Failed {
    error: "graft command not found".to_string()
};

// User presses 'q' to close
state = CommandState::NotStarted;
```

#### Command Failure Flow

```rust
// Initial state
let mut state = CommandState::NotStarted;

// Command starts
state = CommandState::Running;

// Command exits with error
state = CommandState::Completed { exit_code: 1 };

// User presses 'q' to close
state = CommandState::NotStarted;
```

---

## Cross-References

### Related Specifications
- [Grove Command Execution](command-execution.md) - User-facing command execution spec
- [Graft Command](../../graft/graft-command.md) - Authoritative command schema
- [Graft YAML Format](../../graft/graft-yaml-format.md) - Full graft.yaml specification

### Implementation
- **Source**: `grove/crates/grove-core/src/domain.rs`
- **Usage**: `grove/src/tui.rs` (TUI state management)

### Testing
- **Unit Tests**: `grove/crates/grove-core/src/domain.rs` (lines 248-298)
- **Integration Tests**: `grove/tests/test_command_dispatch.rs`

---

## Version History

| Date | Version | Changes |
|------|---------|---------|
| 2026-02-12 | 1.0 | Initial specification |

---

## Notes

### Design Decisions

**Why minimal GraftYaml representation?**
- Grove only displays commands, doesn't execute
- Execution delegated to `graft run` (full context)
- Avoids duplicating Graft's parsing logic
- Simpler, less error-prone

**Why CommandState enum instead of struct?**
- Clear state machine with explicit transitions
- Rust enum ensures only valid states possible
- Pattern matching makes state handling explicit
- Type-safe state transitions

**Why no validation in Command struct?**
- Grove parses for display only, doesn't validate
- Validation happens in `graft run` (execution time)
- Allows displaying even "invalid" commands
- User gets validation feedback when executing

### Future Enhancements

**Command struct**:
- Add `timeout` field for long-running commands
- Add `requires` field for command dependencies
- Add `tags` field for grouping commands

**CommandState**:
- Add `Stopping` state for cancellation in progress
- Add `timeout_seconds` field to track timeouts
- Add `started_at` timestamp for duration tracking

**GraftYaml**:
- Parse full graft.yaml for better display
- Cache parsed results for performance
- Validate schema matches Graft's schema
