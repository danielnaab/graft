# ADR 001: Git Status Implementation Strategy

## Status

Accepted

## Context

Grove needs to query git repository status (branch, dirty state, ahead/behind counts) for multiple repositories efficiently. The implementation must:

- Provide accurate git status information
- Handle timeouts gracefully (repos on slow filesystems/NFS)
- Avoid blocking the TUI on hung git operations
- Support graceful degradation when repos fail
- Be maintainable and understandable

### Technology Options Considered

1. **Pure Gitoxide (gix)**: Use gitoxide APIs for all status queries
2. **Pure Git CLI**: Shell out to git commands for everything
3. **Hybrid Approach**: Use gitoxide for discovery/branch + git CLI for status details

### Evaluation

**Pure Gitoxide (gix 0.66)**

Pros:
- Pure Rust, type-safe
- No subprocess overhead
- Better performance potential

Cons:
- Pre-1.0 API instability (0.66.x series)
- Complex worktree status API (`gix::status::index_as_worktree` patterns changed between versions)
- Ahead/behind counting requires manual graph walking
- Timeout handling requires wrapping blocking operations in threads
- Limited documentation and examples for advanced use cases

**Pure Git CLI**

Pros:
- Well-understood, stable interface
- Timeout handling via `wait-timeout` crate
- Comprehensive documentation

Cons:
- Subprocess overhead for every operation
- String parsing fragility
- Repository discovery logic would need manual implementation

**Hybrid Approach (Selected)**

Pros:
- Leverages gitoxide's strength: repository discovery and reference parsing
- Leverages git CLI's strength: robust, well-tested status operations
- Simple timeout implementation via `wait-timeout`
- Easy to understand and debug
- Can migrate to pure gitoxide incrementally as API stabilizes

Cons:
- Mixed abstractions (some gitoxide, some subprocess)
- Subprocess overhead for status queries

## Decision

Use a **hybrid implementation**:

- **Gitoxide (`gix`)** for:
  - Repository discovery (`gix::discover`)
  - Branch name extraction (`head.referent_name()`)

- **Git CLI subprocess** for:
  - Dirty check: `git status --porcelain`
  - Upstream detection: `git rev-parse --abbrev-ref @{upstream}`
  - Ahead count: `git rev-list --count {upstream}..HEAD`
  - Behind count: `git rev-list --count HEAD..{upstream}`

All git subprocess calls use a 5-second timeout via the `wait-timeout` crate, implemented in `run_git_with_timeout()`.

### Implementation Details

**File:** `crates/grove-engine/src/git.rs`

```rust
// Timeout helper
const GIT_TIMEOUT_MS: u64 = 5000;

fn run_git_with_timeout(mut cmd: Command) -> Option<Output> {
    // Spawns, waits with timeout, kills if timeout occurs
}

// Branch detection via gitoxide
let repo = gix::discover(repo_path)?;
let branch = repo.head()?.referent_name()?.shorten().to_string();

// Status checks via subprocess with timeout
fn check_dirty(repo_path: &Path) -> bool {
    run_git_with_timeout(Command::new("git").args(["status", "--porcelain"]))
        .is_some_and(|output| !output.stdout.is_empty())
}
```

## Implementation Status

**As of 2026-02-10** (Slice 1 completion):

### Implemented & Tested ✅
- **Branch detection** via gitoxide (`gix::discover` + `head().referent_name()`)
- **Dirty status** via `git status --porcelain` subprocess
- **Ahead count** via `git rev-list --count {upstream}..HEAD`
- **Behind count** via `git rev-list --count HEAD..{upstream}`
- **Timeout protection** via `wait-timeout` crate (5s timeout, configurable in Phase 3C)

### Test Coverage
- ✅ `git::tests::detects_clean_working_tree` - Proves dirty detection works on clean repos
- ✅ `git::tests::detects_dirty_working_tree` - Proves dirty detection works on modified repos
- ✅ `git::tests::fails_on_non_git_directory` - Proves graceful error handling
- ✅ 6 integration tests covering end-to-end workspace scenarios

### Known Issues (Tracked for Phase 3)
- ⚠️ Timeout failures are **silent** (return `None` instead of distinct error type)
  - Tracked in Phase 3C improvement plan
  - Will add `GitTimeout` error variant and TUI indicator
- ⚠️ Detached HEAD state not explicitly tested
  - Tracked in Phase 3B testing gaps
  - Implementation returns `None` for branch (acceptable, but should show "[detached]")
- ⚠️ No logging for git operations
  - Tracked in Phase 3C
  - Will add debug/trace logs for observability

### Performance Characteristics
- **Serial execution**: Repos queried one at a time (~50-100ms per repo nominal, 5s worst case)
- **Timeout overhead**: ~5s per hung repo (acceptable for MVP, configurable in future)
- **Scaling limit**: ~20-50 repos before UX degrades (deferred to parallel query optimization)

## Consequences

### Positive

- **Pragmatic**: Ships working status detection immediately
- **Resilient**: 5s timeout prevents TUI hangs on network filesystems
- **Maintainable**: Git CLI behavior is well-documented and stable
- **Incremental**: Can replace subprocess calls with gitoxide as API matures

### Negative

- **Performance**: Subprocess overhead (~10-50ms per repo) limits scaling to thousands of repos
- **Mixed abstraction**: Two different git libraries in the same module

### Migration Path

When gitoxide 1.0 stabilizes:

1. Replace `check_dirty()` with `gix::status::index_as_worktree()`
2. Replace ahead/behind with `gix::revision::walk()` graph algorithms
3. Keep timeout mechanism (wrap gitoxide calls in threads if blocking)

Performance threshold for migration: If profiling shows subprocess overhead >100ms for typical workspaces (10-50 repos), prioritize migration.

## References

- [Gitoxide 0.66 API docs](https://docs.rs/gix/0.66.0/gix/)
- [wait-timeout crate](https://docs.rs/wait-timeout/0.2.1/wait_timeout/)
- Git porcelain format: `man git-status`
- [Grove Phase 2 Review](../../notes/2026-02-10-grove-slice-1-review-phase-2.md)

## Date

2026-02-10
