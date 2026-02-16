---
status: accepted
date: 2026-02-16
tags: [architecture, rust, code-sharing, workspace]
---

# ADR 007: Workspace Common Crate

**Deciders**: Development team
**Context**: Post-Rust rewrite workspace consolidation

## Context

After completing the Rust rewrite (14 tasks, 423 tests, full parity with Python), the repository contained 6 Rust crates across two components (Graft and Grove):

```
crates/
├── grove-core/     # Domain types, traits
├── grove-engine/   # Business logic
├── grove-cli/      # CLI binary
├── graft-core/     # Domain types, error types
├── graft-engine/   # Business logic
└── graft-cli/      # CLI binary
```

Despite significant functional overlap (both tools manage git repositories, parse graft.yaml files, cache state queries), these crates shared **zero code**. This led to:

1. **Duplicate timeout-protected command execution** - Both tools needed safe git operations with configurable timeouts, but each implemented it separately
2. **Duplicate git primitives** - Both needed `git_fetch`, `git_checkout`, `get_current_commit`, implemented independently
3. **Duplicate state query types** - `StateMetadata` and `StateResult` duplicated between `graft-engine` and `grove-cli`
4. **Duplicate cache management** - Both tools used identical SHA256 workspace hashing and cache I/O logic
5. **Triple-parsed graft.yaml** - Three separate parsers for the same file format
6. **Fragmented YAML parsing** - Both `serde_yaml` and `serde_yml` used across crates

The duplication violated DRY principles, increased maintenance burden, and created risk of behavioral divergence.

## Decision

**Create a `graft-common` crate to house shared infrastructure used by both Graft and Grove.**

### What Was Extracted

1. **Timeout-Protected Command Execution** (`graft-common/src/command.rs`)
   - Generalized `run_command_with_timeout()` function
   - Supports configurable timeouts via environment variables
   - Used by all git operations and submodule commands

2. **Git Primitives** (`graft-common/src/git.rs`)
   - `is_git_repo()`, `get_current_commit()`, `git_rev_parse()`
   - `git_fetch()`, `git_checkout()`
   - All use timeout-protected runner

3. **State Query Types** (`graft-common/src/state.rs`)
   - `StateMetadata` (with `time_ago()` and `summary()` methods)
   - `StateResult` (with domain-specific formatting)
   - Cache path computation using SHA256 workspace hash
   - Cache I/O helpers (read/write/invalidate)

4. **Graft.yaml Parsing** (`graft-common/src/config.rs`)
   - `parse_commands()` - extracts `commands:` section
   - `parse_state_queries()` - extracts `state:` section
   - Returns `HashMap<String, T>` for consumer adaptation

### Migration Strategy

All consumers use the **thin wrapper pattern** to maintain API compatibility:

```rust
// Before: local implementation
pub fn run_git_with_timeout(...) -> Result<String, CoreError> {
    // 50 lines of spawn/wait/timeout logic
}

// After: thin wrapper delegating to shared code
pub fn run_git_with_timeout(...) -> Result<String, CoreError> {
    graft_common::command::run_command_with_timeout(...)
        .map_err(|e| match e {
            CommandError::Timeout(msg) => CoreError::GitTimeout(msg),
            // ... convert all error variants
        })
}
```

This preserved existing APIs while eliminating duplicate code.

### Standardization on serde_yaml

Migrated all YAML parsing from `serde_yml` to `serde_yaml`:
- Removed `serde_yml` from workspace dependencies
- Replaced 5 call sites across `grove-engine` and `graft-engine`
- API-compatible migration (no behavioral changes)

**Rationale**: `serde_yaml` is the de-facto standard Rust YAML library with wider ecosystem support. Having two YAML libraries was unnecessary fragmentation.

## Consequences

### Positive

1. **Code Reduction**: Eliminated ~600 lines of duplicate code across crates
   - Command execution: ~70 lines eliminated
   - Git operations: ~160 lines eliminated
   - State types/cache: ~338 lines eliminated
   - Config parsing: ~80 lines eliminated

2. **Improved Reliability**: Timeout protection now universal
   - Graft previously had NO timeout protection on git operations
   - All git operations now have consistent timeout behavior
   - Reduced risk of hanging operations

3. **Consistency**: Shared behavior across tools
   - Both tools use identical cache directory structure
   - Both tools use same workspace hash algorithm
   - Both tools parse graft.yaml identically

4. **Maintainability**: Single source of truth
   - Bug fixes benefit both tools automatically
   - Changes to git behavior need only one update
   - Testing shared code once verifies both consumers

5. **Clear Boundaries**: Well-defined shared infrastructure
   - `graft-common` has 27 unit tests covering edge cases
   - Clear public API documented with examples
   - Error types properly scoped

### Negative

1. **Additional Crate**: More workspace complexity
   - 7 crates instead of 6
   - Additional dependency to manage
   - Mitigated by clear naming and purpose

2. **Error Conversion Overhead**: Thin wrappers needed
   - Each consumer converts `CommandError` to its own error type
   - Adds boilerplate but maintains proper error domains
   - Acceptable trade-off for isolation

3. **Not Full Trait-Based DI**: Still some coupling
   - Functions accept concrete paths, not trait-based abstractions
   - Consumers can't fully inject custom implementations
   - See "Future Work" below

### Neutral

1. **Breaking Change**: Internal refactor only
   - No user-facing API changes
   - All 423 tests pass unchanged
   - Transparent to end users

## Alternatives Considered

### Alternative 1: Keep Duplication

**Pros**: No refactor needed, no additional crate
**Cons**: Maintenance burden, behavioral drift risk, violation of DRY
**Rejected**: Unacceptable long-term maintenance cost

### Alternative 2: Merge All Crates

**Pros**: Maximum code sharing, single crate
**Cons**: Loss of separation between Graft and Grove, monolithic design
**Rejected**: Too coarse-grained, violates clean architecture

### Alternative 3: Create Separate grove-common and graft-common

**Pros**: Each tool owns its shared code
**Cons**: Misses opportunity for cross-tool sharing, still duplicate git ops
**Rejected**: Doesn't solve the core duplication problem

### Alternative 4: Extract to Separate Repository

**Pros**: True library, could be used by other projects
**Cons**: Over-engineering, adds external dependency management
**Rejected**: Premature optimization, no external users yet

### Alternative 5: Use Third-Party Crates

**Pros**: Leverage ecosystem, avoid maintaining custom code
**Cons**: No existing crate covers our exact needs (timeout-protected git + state caching)
**Rejected**: Custom requirements warrant custom solution

## What Was Not Extracted

### Trait-Based DI Refactor (Deferred)

Graft-engine uses direct function calls instead of trait-based dependency injection:

```rust
// Current: direct calls
pub fn resolve_dependencies(config: &GraftConfig, deps_dir: &Path) -> Result<()> {
    graft_common::git::git_fetch(repo_path)?;  // Direct call
}

// Potential future: trait-based
pub fn resolve_dependencies<G: GitOperations>(
    config: &GraftConfig,
    deps_dir: &Path,
    git: &G,
) -> Result<()> {
    git.fetch(repo_path)?;  // Via trait
}
```

**Rationale for deferral**:
1. Would require touching every function signature in graft-engine
2. Current approach works well, no pressing need
3. Tests use temporary directories (integration testing), not mocking
4. Grove already uses traits (`crates/grove-core/src/traits.rs`)
5. Can be added incrementally if testing needs change

## Future Work

### 1. Trait-Based DI for Graft (If Needed)

If graft-engine testing requires unit-level mocking:
- Define `GitOperations` trait in `graft-common`
- Provide `DefaultGitOperations` implementation
- Add `&impl GitOperations` bounds to engine functions
- Follows existing grove-core pattern

### 2. Expand Shared Utilities

Potential future extractions:
- Submodule operations (currently in graft-engine only)
- Workspace discovery logic (if grove needs it)
- Lock file parsing (if grove needs lock file support)

### 3. Performance Optimizations

Potential improvements:
- Parallel git operations using tokio (already a workspace dependency)
- Cache warming strategies
- More aggressive timeout configuration

## Impact Assessment

### Changed Crates

1. **graft-common** (new): 27 tests, 4 modules (command, git, state, config)
2. **grove-engine**: Reduced by ~120 lines, now delegates to graft-common
3. **grove-cli**: Reduced by ~100 lines, uses shared state types
4. **graft-engine**: Reduced by ~260 lines, now timeout-protected
5. **All crates**: Standardized on `serde_yaml`

### Test Coverage

- Workspace total: 423 tests (up from 402 before unification work)
- graft-common: 27 tests (15 for command/git, 7 for state, 8 for config, -3 overlap = 27)
- All existing tests pass unchanged (demonstrates backward compatibility)

### Verification

```bash
cargo fmt --check          # Pass
cargo clippy -- -D warnings # Pass (except pre-existing grove-cli TUI issues)
cargo test                 # Pass (423 tests)
```

## Implementation Timeline

Implemented across 8 iterations (2026-02-16):
1. Created graft-common with timeout-protected command execution
2. Standardized on serde_yaml
3. Added shared git primitives
4. Extracted state types and cache logic
5. Migrated grove-engine to graft-common
6. Migrated graft-engine to graft-common
7. Migrated state query consumers
8. Deduplicated graft.yaml parsing

Total: ~600 lines of duplicate code eliminated, 27 new tests added.

## Related Decisions

- **ADR 004**: Protocol-Based Dependency Injection (Python equivalent)
- **ADR 005**: Functional Service Layer (Python service design)
- See grove-core traits for Rust equivalent of protocol-based DI

## References

### Planning and Progress
- Planning document: `notes/2026-02-16-workspace-unification/plan.md`
- Progress log: `notes/2026-02-16-workspace-unification/progress.md`

### Implementation
- Shared crate: `crates/graft-common/`
- Consumer migrations:
  - `crates/grove-engine/src/git.rs` (command execution)
  - `crates/graft-engine/src/resolution.rs` (git operations)
  - `crates/grove-cli/src/state/` (state types)
  - `crates/graft-engine/src/state.rs` (cache management)

### Specifications
- Workspace structure: `Cargo.toml` (virtual workspace)
- Rust patterns: `.graft/rust-starter/` (architecture reference)

---

**Supersedes**: No prior ADR
**Related**: ADR 004 (protocol-based DI), ADR 005 (functional service layer)
