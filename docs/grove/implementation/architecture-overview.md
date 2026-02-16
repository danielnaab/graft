---
status: stable
version: 0.1.0
updated: 2026-02-10
---

# Grove Architecture Overview

> **Authority Note:** Implementation architecture document describing Grove's three-layer design. For canonical requirements, see [Grove Specifications](../../../specifications/grove/).

## Table of Contents

1. [Three-Layer Architecture](#three-layer-architecture)
2. [TUI Event Loop](#tui-event-loop)
3. [Git Status Querying](#git-status-querying)
4. [Error Handling Philosophy](#error-handling-philosophy)
5. [Testing Strategy](#testing-strategy)

---

## Three-Layer Architecture

Grove follows a clean architecture pattern with three distinct layers:

```
┌─────────────────────────────────────────────┐
│         Binary Layer (src/)                 │
│  • CLI argument parsing (clap)              │
│  • TUI implementation (ratatui)             │
│  • Wiring/composition of services           │
│  • Environment-specific logic               │
└──────────────────┬──────────────────────────┘
                   │ depends on
┌──────────────────▼──────────────────────────┐
│      Engine Layer (crates/grove-engine/)    │
│  • Business logic implementations           │
│  • Adapters (YAML, Git, Registry)           │
│  • I/O operations (file, subprocess)        │
│  • Concrete implementations of traits       │
└──────────────────┬──────────────────────────┘
                   │ depends on
┌──────────────────▼──────────────────────────┐
│       Core Layer (crates/grove-core/)       │
│  • Domain types (WorkspaceName, RepoPath)   │
│  • Trait definitions (protocols)            │
│  • Error types (CoreError)                  │
│  • Pure domain logic (validation)           │
└─────────────────────────────────────────────┘
```

### Layer Responsibilities

**Core Layer** (`grove-core`)
- **Purpose:** Define the domain model and contracts
- **Key Types:**
  - `WorkspaceName` - Validated workspace name newtype
  - `RepoPath` - Validated repository path newtype
  - `WorkspaceConfig` - Workspace configuration model
  - `RepoStatus` - Repository status snapshot
- **Traits:**
  - `ConfigLoader` - Load workspace configuration
  - `GitStatus` - Query git repository status
  - `RepoRegistry` - Manage collection of repositories
- **Error Types:** `CoreError` enum with thiserror
- **Dependencies:** Standard library + thiserror only
- **Rules:**
  - NO I/O operations
  - NO external dependencies (except error handling)
  - Pure functions where possible
  - Domain validation in newtypes

**Engine Layer** (`grove-engine`)
- **Purpose:** Implement business logic with concrete adapters
- **Key Implementations:**
  - `YamlConfigLoader` - Parses YAML config files with serde
  - `GitoxideStatus` - Git status queries (hybrid gix + subprocess)
  - `WorkspaceRegistry` - Multi-repo status cache management
- **Dependencies:** grove-core, serde, gix, subprocess APIs
- **Rules:**
  - Implement core traits
  - Accept trait bounds for testability
  - Encapsulate I/O complexity
  - Return domain-specific Result types

**Binary Layer** (`grove/src/`)
- **Purpose:** User-facing CLI and TUI
- **Key Components:**
  - `main.rs` - CLI entry point, argument parsing, composition
  - `tui.rs` - Terminal UI with ratatui, event handling
- **Dependencies:** All layers + clap, ratatui, crossterm
- **Rules:**
  - Wire together engine implementations
  - Handle environment-specific concerns (terminal, env vars)
  - Minimal business logic (just presentation)

### Dependency Direction

```
Binary → Engine → Core
```

**Key Principle:** Dependencies point inward. Core has no knowledge of Engine or Binary.

This enables:
- **Testability:** Mock implementations via traits
- **Flexibility:** Swap implementations without changing core
- **Clarity:** Business rules isolated from I/O concerns

---

## TUI Event Loop

Grove uses `ratatui` with `crossterm` backend for terminal UI.

### Event Loop Pattern

```rust
// 1. Setup
enable_raw_mode()?;
let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;

// 2. Event loop
loop {
    terminal.draw(|frame| {
        // Render UI
        let repos = registry.list_repos();
        let items = repos.map(|repo| format_status(repo));
        frame.render_stateful_widget(list, area, &mut state);
    })?;

    if should_quit {
        break;
    }

    // Poll for events with 100ms timeout
    if event::poll(Duration::from_millis(100))? {
        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press {
                handle_key(key.code);  // Update state
            }
        }
    }
}

// 3. Cleanup
disable_raw_mode()?;
terminal.backend_mut().execute(LeaveAlternateScreen)?;
```

### State Management

**App State:**
```rust
struct App<R: RepoRegistry> {
    registry: R,           // Status data (injected)
    list_state: ListState, // Selected item
    should_quit: bool,     // Exit flag
}
```

**State Flow:**
1. **Initialize:** Load config, create registry, refresh status
2. **Render:** Query registry for current status, format for display
3. **Handle Input:** Update list_state based on keybindings
4. **Repeat:** Re-render on next frame

**Key Design Choice:** Status is read-only in TUI. Refresh happens on startup, not during TUI session (Slice 1 limitation).

### Keybinding Handling

```rust
fn handle_key(&mut self, code: KeyCode) {
    match code {
        KeyCode::Char('q') | KeyCode::Esc => self.should_quit = true,
        KeyCode::Char('j') | KeyCode::Down => self.next(),
        KeyCode::Char('k') | KeyCode::Up => self.previous(),
        _ => {}  // Ignore unknown keys
    }
}
```

**Navigation wraps around:** Last item → j → First item, First item → k → Last item

---

## Git Status Querying

Grove uses a **hybrid approach**: gitoxide for repository discovery + git CLI for status details.

### Architecture Decision

See [ADR-001](./adr-001-git-status-implementation.md) for full rationale.

**Summary:**
- **Gitoxide (gix):** Discover repo, read branch name
- **Git CLI:** Dirty status, ahead/behind counts
- **Reason:** Gitoxide 0.66.x API is pre-1.0 and complex; git CLI is stable and well-understood

### Query Flow

```
User starts grove
    ↓
main.rs: registry.refresh_all()
    ↓
Registry: for each repo in config
    ↓
GitoxideStatus::get_status(repo_path)
    ├─→ gix::discover(path) → Repository
    ├─→ repo.head()?.referent_name() → branch name
    ├─→ check_dirty(path) → git status --porcelain (subprocess)
    ├─→ check_ahead_behind(path) → git rev-list --count (subprocess)
    └─→ RepoStatus { path, branch, dirty, ahead, behind, error }
    ↓
Registry: cache status in HashMap<RepoPath, RepoStatus>
    ↓
TUI: render cached status
```

### Timeout Protection

All git subprocesses have 5-second timeout:

```rust
const GIT_TIMEOUT_MS: u64 = 5000;

fn run_git_with_timeout(mut cmd: Command) -> Option<Output> {
    let mut child = cmd.spawn()?;
    match child.wait_timeout(Duration::from_millis(GIT_TIMEOUT_MS)) {
        Ok(Some(_)) => child.wait_with_output().ok(),  // Completed
        Ok(None) => {
            child.kill();  // Timeout - kill process
            None
        }
        Err(_) => None,  // Error
    }
}
```

**Known Issue (Phase 3):** Timeouts are silent (return None, no distinct error). Will be fixed in Phase 3C.

---

## Error Handling Philosophy

Grove follows **graceful degradation** principles:

### Principle 1: Continue on Failure

**Bad:**
```rust
for repo in repos {
    let status = git_status.get_status(&repo)?;  // ❌ Stops on first error
    cache.insert(repo, status);
}
```

**Good:**
```rust
for repo in repos {
    let status = match git_status.get_status(&repo) {
        Ok(status) => status,
        Err(e) => {
            log::warn!("Failed: {}: {}", repo, e);
            RepoStatus::with_error(repo.clone(), e.to_string())  // ✅ Continue
        }
    };
    cache.insert(repo, status);
}
```

**Result:** One broken repo doesn't prevent others from loading.

### Principle 2: Surface Errors to User

Errors are shown in TUI:
```
~/src/broken-repo [error: Failed to open repository: No such file or directory]
```

User can:
- See which repos failed
- Debug individually
- Remove from config if permanently broken

### Principle 3: Log for Debugging

```rust
log::warn!("Failed to get status for {}: {}", repo_path, e);
```

Users enable with `RUST_LOG=grove=debug grove` for troubleshooting.

### Error Type Hierarchy

```
anyhow::Error (top-level, main.rs)
    ↓
CoreError (domain errors)
    • EmptyWorkspaceName
    • EmptyRepoPath
    • InvalidRepoPath
    • InvalidConfig
    • GitError
    • RepoNotFound
    ↓
std::io::Error (wrapped in CoreError::GitError)
```

**Design Choice:** Use `Result<T>` type alias at core layer, `anyhow::Result` at binary layer for flexibility.

---

## Testing Strategy

### Unit Tests

**Location:** Same file as code under test

**Scope:** Test individual functions in isolation

**Example:**
```rust
// grove-core/src/domain.rs
#[test]
fn rejects_empty_workspace_name() {
    let result = WorkspaceName::new("".to_string());
    assert!(matches!(result, Err(CoreError::EmptyWorkspaceName)));
}
```

**Coverage:** Core domain validation, engine business logic

### Integration Tests

**Location:** `grove/tests/integration_test.rs`

**Scope:** End-to-end workflows with real filesystem

**Example:**
```rust
#[test]
fn end_to_end_workspace_with_real_repos() {
    // Create temp git repo
    let temp = TempDir::new().unwrap();
    init_git_repo(temp.path());

    // Create config
    let config = create_test_config(temp.path());

    // Load and refresh
    let loader = YamlConfigLoader::new();
    let config = loader.load_workspace(&config_path).unwrap();
    let mut registry = WorkspaceRegistry::new(config, GitoxideStatus::new());
    registry.refresh_all().unwrap();

    // Verify
    assert_eq!(registry.list_repos().len(), 1);
}
```

**Coverage:** Config loading, registry refresh, error handling

### Test Doubles

**Approach:** Trait-based fakes (no mocking frameworks)

**Example:**
```rust
struct FakeGitStatus {
    should_fail: bool,
}

impl GitStatus for FakeGitStatus {
    fn get_status(&self, _path: &RepoPath) -> Result<RepoStatus> {
        if self.should_fail {
            Err(CoreError::GitError { details: "fake error".into() })
        } else {
            Ok(RepoStatus::new(_path.clone()))
        }
    }
}
```

**Benefits:**
- Simple, no magic
- Fast compilation
- Easy to understand

### Test Coverage Goals

**Current (Slice 1):**
- Core: ~90% (domain validation well-tested)
- Engine: ~80% (adapters + registry tested)
- Binary: ~20% (TUI untested, tracked in Phase 3B)

**Target (After Phase 3B):**
- Core: 90%+
- Engine: 85%+
- Binary: 70%+ (TUI unit tests + integration tests)

---

## Future Architecture Evolution

### Planned Improvements

**Slice 2: Repository Detail Pane**
- Add `RepoDetails` domain type (commit log, changed files)
- Extend `GitStatus` trait or add `GitDetails` trait
- Two-pane layout in TUI

**Performance Optimization:**
- Parallel git queries (rayon)
- Persistent status cache (~/.cache/grove/)
- Background refresh thread in TUI

**Extensibility:**
- Plugin system for custom git providers (libgit2, jgit)
- Custom status formatters
- Remote workspace configurations

### Architectural Constraints

**Must maintain:**
- Three-layer separation
- Trait-based DI
- Graceful degradation
- Core layer has no I/O

**May change:**
- Parallel execution model
- Caching strategy
- Status refresh triggers

---

## References

- [ADR-001: Git Status Implementation](./adr-001-git-status-implementation.md)
- [Slice 1 Planning](../planning/slices/slice-1-workspace-config.md)
- [Rust Starter Patterns](../../../.graft/rust-starter/docs/architecture/architecture.md)
- [Ratatui Documentation](https://docs.rs/ratatui/0.29.0/ratatui/)
- [Gitoxide Documentation](https://docs.rs/gix/0.66.0/gix/)

---

## Changelog

- **2026-02-10:** Initial architecture overview for Slice 1
  - Three-layer pattern documented
  - TUI event loop explained
  - Git querying strategy described
  - Error handling philosophy defined
  - Testing approach outlined
