---
status: completed
started: 2026-02-10
completed: 2026-02-10
---

# Slice 1: Workspace Config + Repo List TUI

## User Story

**As a developer working across multiple repositories,**
**I want to launch grove and see my repos with their git status,**
**So that I can quickly understand what needs attention.**

## Success Criteria

- [x] Grove loads workspace config from `~/.config/grove/workspace.yaml`
- [x] TUI displays list of repositories with:
  - [x] Repository path
  - [x] Current branch name
  - [x] Clean/dirty indicator (● dirty, ○ clean)
  - [x] Ahead/behind counts (↑ahead ↓behind)
- [x] Navigation works:
  - [x] `j` moves selection down
  - [x] `k` moves selection up
  - [x] `q` quits cleanly
- [x] Error handling:
  - [x] Non-existent repo shows warning, continues with others
  - [x] Non-git directory shows error indicator
  - [x] Missing config shows helpful error message

## Architecture: Three-Layer Pattern

```
grove/src/main.rs (CLI + TUI)
    ↓
grove-engine (business logic: config, registry, git)
    ↓
grove-core (domain types, traits, errors)
```

### Layer Responsibilities

**grove-core**: Domain types and contracts
- `WorkspaceName`, `RepoPath` newtypes with validation
- `WorkspaceConfig`, `RepositoryDeclaration` domain models
- `RepoStatus` struct (path, branch, dirty, ahead/behind)
- `ConfigLoader`, `GitStatus`, `RepoRegistry` traits
- `CoreError` enum

**grove-engine**: Business logic adapters
- `YamlConfigLoader` implementing `ConfigLoader`
- `GitoxideStatus` implementing `GitStatus`
- `WorkspaceRegistry<G>` implementing `RepoRegistry`

**grove binary**: CLI + TUI wiring
- Clap CLI argument parsing
- TUI event loop with ratatui
- Status rendering

## Implementation Progress

### Core Domain (`grove-core`)
- [x] `domain.rs`: WorkspaceName, RepoPath, WorkspaceConfig, RepositoryDeclaration, RepoStatus
- [x] `traits.rs`: ConfigLoader, GitStatus, RepoRegistry traits
- [x] `error.rs`: CoreError enum with thiserror
- [x] Unit tests for domain validation (5 tests)

### Engine Adapters (`grove-engine`)
- [x] `config.rs`: YamlConfigLoader with serde_yml
- [x] `git.rs`: GitoxideStatus with gix + git commands
- [x] `registry.rs`: WorkspaceRegistry with status cache
- [x] Integration tests with trait fakes (9 tests)

### Binary + TUI (`grove/src`)
- [x] `main.rs`: CLI arg parsing, wiring
- [x] `tui.rs`: Ratatui event loop, repo list widget, keybindings
- [x] Manual end-to-end testing

### Polish
- [x] Error handling refinement (graceful degradation)
- [x] Dirty check implementation (git status --porcelain)
- [x] Ahead/behind counts (git rev-list)
- [x] End-to-end integration tests (6 tests)
- [x] Update roadmap with completion status
- [x] User documentation (comprehensive guide)

## Dependencies

New workspace dependencies to add to `Cargo.toml`:

```toml
[workspace.dependencies]
thiserror = "2"
anyhow = "1"
clap = { version = "4", features = ["derive"] }
serde = { version = "1", features = ["derive"] }
serde_yaml = "0.9"
gix = "0.66"
ratatui = "0.29"
crossterm = "0.28"
shellexpand = "3"
```

## Technical Risks

1. **Gitoxide learning curve**: First use of gix API
   - Mitigation: Start with branch + dirty check; defer ahead/behind if complex

2. **Ratatui patterns**: First TUI implementation
   - Mitigation: Follow ratatui examples; keep UI simple (single list)

3. **Path expansion**: Cross-platform tilde expansion
   - Mitigation: Use shellexpand crate; clear docs for config location

## Open Design Questions

1. **Config location**: Default to `~/.config/grove/workspace.yaml` or support auto-discovery?
   - Decision: Start with explicit default; add discovery later if needed

2. **Status refresh cadence**: On-launch only or periodic background refresh?
   - Decision: On-launch only for Slice 1

3. **Error display**: How to show errors for individual repos in TUI?
   - Decision: Show `[error]` indicator in list, log to stderr, continue with others

## Verification

### Manual Test Plan

1. Create test workspace config at `~/.config/grove/workspace.yaml`:
   ```yaml
   name: test-workspace
   repositories:
     - path: ~/src/graft
       tags: [rust, cli]
     - path: ~/src/graft/grove
       tags: [rust, tui, grove]
   ```

2. Build and run: `cargo build && cargo run`

3. Expected behavior:
   - TUI shows 2 repositories with status
   - j/k navigation works
   - q quits cleanly

4. Edge cases:
   - Non-existent repo path
   - Non-git directory
   - Empty workspace.yaml
   - Missing workspace.yaml

5. Automated tests: `cargo test && cargo clippy && cargo fmt --check`

## Sources

- [Architecture Spec](../../../../../docs/specifications/grove/architecture.md)
- [Workspace Config Spec](../../../../../docs/specifications/grove/workspace-config.md)
- [Rust Starter Patterns](../../../.graft/rust-starter/docs/architecture.md)
