# Grove Slice 1 - Improvement Plan

**Date:** 2026-02-10
**Status:** Planning
**Scope:** Address critical issues before Slice 2 development

## Executive Summary

Critique identified **32 issues** across 8 categories. Slice 1 has solid architecture but requires targeted improvements in:
1. Documentation accuracy (misleading/wrong information)
2. Test coverage (TUI layer untested)
3. Error handling depth (timeout behavior, generic messages)
4. Production readiness (validation, logging, version info)

**Recommendation:** Execute Phases 3A-3C before starting Slice 2 (12-16 hours total).

---

## Phase 3A: Critical Documentation & Feature Alignment (MUST DO)

**Priority:** CRITICAL
**Effort:** 2-3 hours
**Blocking:** Yes - users following docs will fail

### Issues Addressed
- [CRITICAL] Undocumented environment variable (docs claim feature doesn't exist)
- [HIGH] Dirty status limitation docs are wrong (feature works, docs say it doesn't)
- [MEDIUM] Missing troubleshooting section

### Tasks

**Task 1: Fix Environment Variable Documentation (30 min)**
- **Option A:** Implement the documented feature
  - Add `#[arg(env = "GROVE_WORKSPACE")]` to clap argument
  - Test with `GROVE_WORKSPACE=~/test.yaml grove`
  - Document in user guide usage section
- **Option B:** Remove from documentation
  - Remove lines 85-89 from user-guide.md
  - Document only `--workspace` flag

**Decision:** Recommend Option A (implement) - it's trivial and users expect it

**Files:**
- `src/main.rs:16` - Add env annotation
- `docs/user-guide.md:85-89` - Verify/clarify docs

**Task 2: Fix Dirty Status Documentation (15 min)**
- **Location:** `docs/user-guide.md:179-200`
- **Current (WRONG):**
  ```markdown
  1. **Dirty status always shows clean** `○`
     - Working tree changes not detected yet
  ```
- **Fix:** Remove this limitation entirely (dirty status IS working)
- **Test Evidence:** `git.rs:218-266` proves `check_dirty()` works
- **Verify:** Run integration test `detects_dirty_working_tree` and confirm it passes

**Files:**
- `docs/user-guide.md:179-184` - Delete incorrect limitation

**Task 3: Update ADR with Actual Implementation (30 min)**
- **Location:** `docs/grove/implementation/adr-001-git-status-implementation.md`
- **Add section:** "Implementation Status" documenting:
  - ✅ Dirty detection via `git status --porcelain` (implemented, tested)
  - ✅ Ahead/behind via `git rev-list --count` (implemented, tested)
  - ✅ 5s timeout via `wait-timeout` (implemented, NOT tested)
  - ⚠️ Known issue: Timeout failures are silent (tracked for Phase 3C)

**Task 4: Add Troubleshooting Section to User Guide (1 hour)**
- **Location:** `docs/user-guide.md` (new section after "Controls")
- **Content:**
  ```markdown
  ## Troubleshooting

  ### Grove hangs on startup
  - **Cause:** Repository on slow network filesystem or NFS
  - **Behavior:** Git operations timeout after 5 seconds
  - **Workaround:** Remove slow repos from workspace.yaml temporarily
  - **Future:** Configurable timeout coming in future release

  ### One repository shows [error: ...]
  - **Cause:** Not a valid git repository, or git command failed
  - **Debug:** Check `RUST_LOG=grove=debug grove` for details
  - **Action:** Fix the repository or remove from workspace.yaml

  ### Path expansion rules
  - Supports: `~/path` (home dir), `$VAR/path` (env vars)
  - Does NOT support: `~user/path`, `~+` (PWD), `~-` (OLDPWD)
  - Undefined variables: `$UNDEFINED` → literal string "$UNDEFINED"

  ### Performance with large workspaces
  - Status refresh is serial (one repo at a time)
  - Worst case: 5 seconds per repo (timeout)
  - Recommendation: Split large workspaces or wait for parallel queries
  ```

**Task 5: Add Architecture Documentation (30 min)**
- **Location:** `docs/grove/implementation/architecture-overview.md` (new file)
- **Content:** Three-layer architecture diagram, TUI event loop, git querying strategy

**Verification:**
```bash
# Test env var support works
export GROVE_WORKSPACE=/tmp/test.yaml
grove  # Should load /tmp/test.yaml instead of default

# Verify dirty status limitation is removed from docs
grep -i "dirty status always shows clean" docs/user-guide.md
# Should return no matches

# Verify troubleshooting section exists
grep -A 5 "## Troubleshooting" docs/user-guide.md
```

**Success Criteria:**
- ✅ Environment variable feature works OR documentation is corrected
- ✅ Dirty status limitation removed from docs
- ✅ Troubleshooting section added with 4+ scenarios
- ✅ ADR reflects actual implementation status

---

## Phase 3B: Testing Gaps (SHOULD DO)

**Priority:** HIGH
**Effort:** 6-8 hours
**Blocking:** No (but strongly recommended before Slice 2)

### Issues Addressed
- [CRITICAL] TUI layer untested (232 lines, 0 tests)
- [HIGH] Timeout behavior untested
- [HIGH] Detached HEAD case untested
- [HIGH] Config validation edge cases untested
- [MEDIUM] Rendering edge cases untested

### Tasks

**Task 1: Add TUI Unit Tests (3-4 hours)**
- **Location:** `src/tui.rs` (new test module at bottom)
- **Coverage Target:** Test core TUI logic without full integration

**Tests to Write:**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use grove_core::*;
    use crossterm::event::KeyCode;

    // Test 1: Keybinding handling
    #[test]
    fn handles_quit_keys() {
        let mut app = App::new(MockRegistry::empty());
        app.handle_key(KeyCode::Char('q'));
        assert!(app.should_quit);

        let mut app = App::new(MockRegistry::empty());
        app.handle_key(KeyCode::Esc);
        assert!(app.should_quit);
    }

    // Test 2: Navigation with empty list
    #[test]
    fn navigation_with_empty_list_does_not_panic() {
        let mut app = App::new(MockRegistry::empty());
        app.next();  // Should not crash
        app.previous();  // Should not crash
        assert_eq!(app.list_state.selected(), None);
    }

    // Test 3: Navigation wraps around
    #[test]
    fn navigation_wraps_at_boundaries() {
        let mut app = App::new(MockRegistry::with_repos(3));
        app.list_state.select(Some(0));

        app.previous();  // Should wrap to last item
        assert_eq!(app.list_state.selected(), Some(2));

        app.next();  // Should wrap to first item
        assert_eq!(app.list_state.selected(), Some(0));
    }

    // Test 4: Format repo line with error
    #[test]
    fn formats_error_status_correctly() {
        let path = "~/src/test".to_string();
        let mut status = RepoStatus::new(RepoPath::new("~/src/test").unwrap());
        status.error = Some("Failed to open repo".to_string());

        let line = format_repo_line(path, Some(&status));
        // Verify line contains error styling
        // Note: This tests the data, not the visual appearance
        assert_eq!(line.spans.len(), 3);
        // span[2] should contain "[error: Failed to open repo]"
    }

    // Test 5: Format repo line with all status fields
    #[test]
    fn formats_complete_status_with_ahead_behind() {
        let path = "~/src/test".to_string();
        let mut status = RepoStatus::new(RepoPath::new("~/src/test").unwrap());
        status.branch = Some("main".to_string());
        status.is_dirty = true;
        status.ahead = Some(3);
        status.behind = Some(2);

        let line = format_repo_line(path, Some(&status));
        // Verify spans include: path, branch, dirty indicator, ahead, behind
        assert!(line.spans.len() >= 7);
    }

    // Test 6: Format detached HEAD
    #[test]
    fn formats_detached_head() {
        let path = "~/src/test".to_string();
        let mut status = RepoStatus::new(RepoPath::new("~/src/test").unwrap());
        status.branch = None;  // Detached HEAD

        let line = format_repo_line(path, Some(&status));
        // Should show "[detached]" text
        let text: String = line.spans.iter()
            .map(|s| s.content.to_string())
            .collect();
        assert!(text.contains("[detached]"));
    }

    // Test 7: Format loading state
    #[test]
    fn formats_loading_state() {
        let path = "~/src/test".to_string();
        let line = format_repo_line(path, None);

        let text: String = line.spans.iter()
            .map(|s| s.content.to_string())
            .collect();
        assert!(text.contains("[loading...]"));
    }
}

// Mock registry for testing
struct MockRegistry {
    repos: Vec<RepoPath>,
    statuses: HashMap<RepoPath, RepoStatus>,
}

impl MockRegistry {
    fn empty() -> Self {
        Self { repos: vec![], statuses: HashMap::new() }
    }

    fn with_repos(count: usize) -> Self {
        let repos: Vec<RepoPath> = (0..count)
            .map(|i| RepoPath::new(&format!("~/repo{}", i)).unwrap())
            .collect();
        let statuses = repos.iter().map(|r| {
            (r.clone(), RepoStatus::new(r.clone()))
        }).collect();
        Self { repos, statuses }
    }
}

impl RepoRegistry for MockRegistry {
    fn list_repos(&self) -> Vec<RepoPath> { self.repos.clone() }
    fn get_status(&self, path: &RepoPath) -> Option<&RepoStatus> {
        self.statuses.get(path)
    }
    fn refresh_all(&mut self) -> grove_core::Result<()> { Ok(()) }
}
```

**Files:**
- `src/tui.rs` - Add test module (7 tests minimum)
- May need to add `pub` to some types for testability

**Task 2: Add Git Module Edge Case Tests (2 hours)**
- **Location:** `crates/grove-engine/src/git.rs` (expand existing test module)

**Tests to Write:**

```rust
#[test]
fn handles_detached_head_state() {
    use tempfile::TempDir;
    let temp_dir = TempDir::new().unwrap();
    let repo_path = temp_dir.path();

    // Initialize repo with a commit
    Command::new("git").args(["init"]).current_dir(repo_path).output().unwrap();
    Command::new("git").args(["config", "user.email", "test@example.com"])
        .current_dir(repo_path).output().unwrap();
    Command::new("git").args(["config", "user.name", "Test"])
        .current_dir(repo_path).output().unwrap();
    std::fs::write(repo_path.join("file"), "content").unwrap();
    Command::new("git").args(["add", "."]).current_dir(repo_path).output().unwrap();
    Command::new("git").args(["commit", "-m", "Initial"])
        .current_dir(repo_path).output().unwrap();

    // Detach HEAD by checking out the commit SHA
    let sha = Command::new("git").args(["rev-parse", "HEAD"])
        .current_dir(repo_path).output().unwrap();
    let sha_str = String::from_utf8(sha.stdout).unwrap().trim().to_string();
    Command::new("git").args(["checkout", &sha_str])
        .current_dir(repo_path).output().unwrap();

    // Test that status handles detached HEAD
    let status = GitoxideStatus::new();
    let path = RepoPath::new(repo_path.to_str().unwrap()).unwrap();
    let result = status.get_status(&path).unwrap();

    // Branch should be None or "detached" (depending on implementation)
    assert!(
        result.branch.is_none() || result.branch == Some("detached".to_string()),
        "Detached HEAD should not have a branch name"
    );
}

#[test]
fn handles_repository_with_upstream() {
    // TODO: Create test repo with upstream tracking
    // Set up: git remote add origin <url>
    //         git branch --set-upstream-to=origin/main
    // Test: ahead/behind counts should be Some(0), Some(0) or None
}

#[test]
fn handles_repository_without_upstream() {
    use tempfile::TempDir;
    let temp_dir = TempDir::new().unwrap();
    let repo_path = temp_dir.path();

    // Initialize repo without remote
    Command::new("git").args(["init"]).current_dir(repo_path).output().unwrap();

    // Check that ahead/behind are None (no upstream)
    let (ahead, behind) = GitoxideStatus::check_ahead_behind(repo_path);
    assert_eq!(ahead, None);
    assert_eq!(behind, None);
}
```

**Files:**
- `crates/grove-engine/src/git.rs` - Add 3 tests

**Task 3: Add Config Validation Tests (1 hour)**
- **Location:** `crates/grove-engine/src/config.rs` (expand test module)

**Tests to Write:**

```rust
#[test]
fn rejects_empty_workspace_name() {
    let yaml = r"
name: ''
repositories: []
";
    let loader = YamlConfigLoader::new();
    let result = loader.load_from_str(yaml);
    assert!(result.is_err(), "Should reject empty workspace name");
}

#[test]
fn handles_duplicate_repository_paths() {
    let yaml = r"
name: test
repositories:
  - path: ~/src/repo1
  - path: ~/src/repo1
";
    let loader = YamlConfigLoader::new();
    let result = loader.load_from_str(yaml);
    // Current behavior: Silently accepts duplicates
    // Future: Should warn or error
    assert!(result.is_ok());
    // TODO: Add validation to reject/warn about duplicates
}

#[test]
fn handles_path_with_spaces() {
    let yaml = r#"
name: test
repositories:
  - path: "~/my documents/repo"
"#;
    let loader = YamlConfigLoader::new();
    let result = loader.load_from_str(yaml);
    assert!(result.is_ok());
    let config = result.unwrap();
    assert_eq!(config.repositories[0].path.as_path().to_string_lossy(),
               shellexpand::tilde("~/my documents/repo").as_ref());
}

#[test]
fn handles_undefined_environment_variable() {
    std::env::remove_var("GROVE_TEST_UNDEFINED_VAR");
    let yaml = r"
name: test
repositories:
  - path: $GROVE_TEST_UNDEFINED_VAR/repo
";
    let loader = YamlConfigLoader::new();
    let result = loader.load_from_str(yaml);
    // shellexpand returns literal string for undefined vars
    assert!(result.is_ok());
}
```

**Files:**
- `crates/grove-engine/src/config.rs` - Add 4 tests
- May need to add `load_from_str()` helper for testing

**Verification:**
```bash
# Run all new tests
cargo test

# Verify TUI tests exist
cargo test --test tui -- --nocapture

# Verify git edge case tests exist
cargo test -p grove-engine git::tests::handles_detached_head

# Check test coverage (optional, requires cargo-tarpaulin)
cargo tarpaulin --packages grove-core grove-engine grove
```

**Success Criteria:**
- ✅ TUI has 7+ unit tests covering keybindings, navigation, formatting
- ✅ Git module has 3+ edge case tests (detached HEAD, upstream, etc.)
- ✅ Config has 4+ validation tests
- ✅ All tests pass
- ✅ Test coverage >80% for core business logic

---

## Phase 3C: Error Handling & User Experience (SHOULD DO)

**Priority:** HIGH
**Effort:** 4-6 hours
**Blocking:** No (but improves production readiness significantly)

### Issues Addressed
- [HIGH] Timeout handling is silent and hard-coded
- [HIGH] Error messages too generic
- [MEDIUM] Graceful degradation incomplete (return type)
- [CRITICAL] No logging output control

### Tasks

**Task 1: Redesign Timeout Handling (2-3 hours)**

**Current Problem:**
```rust
Ok(None) => {
    // Timeout occurred, kill the process
    let _ = child.kill();
    let _ = child.wait();
    None  // SILENT FAILURE
}
```

**Solution: Add Timeout Error Type**

**Step 1:** Add `TimeoutError` to core errors
```rust
// crates/grove-core/src/error.rs
#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    // ... existing variants ...

    #[error("git operation timed out after {timeout_ms}ms: {operation}")]
    GitTimeout {
        operation: String,
        timeout_ms: u64,
    },
}
```

**Step 2:** Update git.rs to return timeout errors
```rust
// crates/grove-engine/src/git.rs
fn run_git_with_timeout(mut cmd: Command, operation: &str) -> Result<Output> {
    let mut child = cmd
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| CoreError::GitError {
            details: format!("Failed to spawn git: {e}")
        })?;

    let timeout = Duration::from_millis(GIT_TIMEOUT_MS);

    match child.wait_timeout(timeout) {
        Ok(Some(_status)) => {
            child.wait_with_output().map_err(|e| CoreError::GitError {
                details: format!("Failed to read git output: {e}")
            })
        }
        Ok(None) => {
            // Timeout occurred
            let _ = child.kill();
            let _ = child.wait();
            Err(CoreError::GitTimeout {
                operation: operation.to_string(),
                timeout_ms: GIT_TIMEOUT_MS,
            })
        }
        Err(e) => Err(CoreError::GitError {
            details: format!("Git process error: {e}")
        }),
    }
}

// Update call sites:
fn check_dirty(repo_path: &Path) -> Result<bool> {
    let mut cmd = Command::new("git");
    cmd.args(["status", "--porcelain"]).current_dir(repo_path);

    run_git_with_timeout(cmd, "git status")?
        .map(|output| !output.stdout.is_empty())
        .ok_or_else(|| CoreError::GitError {
            details: "No output from git status".to_string()
        })
}
```

**Step 3:** Update TUI to show timeout indicator
```rust
// src/tui.rs
Some(status) => {
    if let Some(error_msg) = &status.error {
        let style = if error_msg.contains("timed out") {
            Style::default().fg(Color::Yellow)  // Yellow for timeout
        } else {
            Style::default().fg(Color::Red)  // Red for other errors
        };

        Line::from(vec![
            Span::styled(path, Style::default().fg(Color::White)),
            Span::raw(" "),
            Span::styled(format!("[error: {error_msg}]"), style),
        ])
    }
    // ... rest of formatting
}
```

**Step 4:** Make timeout configurable via environment variable
```rust
// crates/grove-engine/src/git.rs
fn get_timeout() -> Duration {
    let timeout_ms = std::env::var("GROVE_GIT_TIMEOUT_MS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(5000);  // Default 5s

    Duration::from_millis(timeout_ms)
}
```

**Files:**
- `crates/grove-core/src/error.rs` - Add GitTimeout variant
- `crates/grove-engine/src/git.rs` - Update timeout handling
- `src/tui.rs` - Show timeout vs error differently
- `docs/user-guide.md` - Document `GROVE_GIT_TIMEOUT_MS` env var

**Task 2: Improve Error Messages with Suggestions (1-2 hours)**

**Current:**
```
Error: Failed to load workspace from '~/.config/grove/workspace.yaml'
```

**Improved:**
```
Error: Workspace config not found: ~/.config/grove/workspace.yaml

Suggestions:
  • Create the config file: mkdir -p ~/.config/grove
  • Use a different path: grove --workspace /path/to/config.yaml
  • See example config: grove --help

Documentation: https://github.com/.../docs/user-guide.md#configuration
```

**Implementation:**
```rust
// src/main.rs
let config = loader
    .load_workspace(&config_path)
    .or_else(|e| {
        if e.to_string().contains("No such file") {
            eprintln!("Error: Workspace config not found: {}\n", config_path);
            eprintln!("Suggestions:");
            eprintln!("  • Create the config file: mkdir -p ~/.config/grove");
            eprintln!("  • Use a different path: grove --workspace /path/to/config.yaml");
            eprintln!("  • See example config: grove --help\n");
        }
        Err(e)
    })
    .with_context(|| format!("Failed to load workspace from '{config_path}'"))?;
```

Or use a dedicated error formatting function.

**Files:**
- `src/main.rs` - Add helpful error messages for common cases
- Consider using `miette` crate for fancy error reporting

**Task 3: Improve refresh_all() Return Type (1 hour)**

**Current:**
```rust
fn refresh_all(&mut self) -> Result<()> {
    // ... processes all repos ...
    Ok(())  // Always succeeds!
}
```

**Improved:**
```rust
pub struct RefreshStats {
    pub successful: usize,
    pub failed: usize,
}

fn refresh_all(&mut self) -> Result<RefreshStats> {
    let mut stats = RefreshStats { successful: 0, failed: 0 };

    for repo_decl in &self.config.repositories {
        let status = match self.git_status.get_status(&repo_decl.path) {
            Ok(status) => {
                stats.successful += 1;
                status
            }
            Err(e) => {
                stats.failed += 1;
                log::warn!("Failed to get status for {}: {}", repo_decl.path, e);
                RepoStatus::with_error(repo_decl.path.clone(), e.to_string())
            }
        };
        self.status_cache.insert(repo_decl.path.clone(), status);
    }

    Ok(stats)
}
```

**Usage in main.rs:**
```rust
let stats = registry.refresh_all()
    .context("Failed to refresh repository status")?;

if stats.failed > 0 {
    log::warn!("Loaded {}/{} repos ({} had errors)",
               stats.successful,
               stats.successful + stats.failed,
               stats.failed);
}
```

**Files:**
- `crates/grove-engine/src/registry.rs` - Change return type
- `crates/grove-core/src/traits.rs` - Update trait signature
- `src/main.rs` - Handle stats
- Update all tests that call `refresh_all()`

**Task 4: Add Comprehensive Logging (30 min)**

**Add logs at key points:**
```rust
// src/main.rs
log::info!("Grove {} starting", env!("CARGO_PKG_VERSION"));
log::debug!("Loading workspace config from: {}", config_path);
log::info!("Loaded workspace '{}' with {} repositories",
           config.name, config.repositories.len());
log::debug!("Repositories: {:?}", config.repositories);
log::info!("Querying status for {} repositories...", config.repositories.len());

// After refresh
log::info!("Status refresh complete: {}/{} successful",
           stats.successful, stats.successful + stats.failed);

// In registry.rs
log::debug!("Querying status for: {}", repo_decl.path);
log::trace!("Status result: {:?}", status);

// In git.rs
log::trace!("Running git command: {:?}", cmd);
log::debug!("Git operation {} took {}ms", operation, elapsed_ms);
```

**Files:**
- `src/main.rs` - Add startup and shutdown logs
- `crates/grove-engine/src/registry.rs` - Add per-repo logs
- `crates/grove-engine/src/git.rs` - Add operation logs (trace level)
- `docs/user-guide.md` - Document `RUST_LOG` usage

**Verification:**
```bash
# Test timeout error shows correctly
GROVE_GIT_TIMEOUT_MS=1 grove  # Should show timeout errors

# Test error messages are helpful
rm ~/.config/grove/workspace.yaml
grove  # Should show helpful suggestions

# Test logging works
RUST_LOG=grove=debug grove
RUST_LOG=grove=trace grove

# Test refresh stats
grove  # Should log "Loaded X/Y repos (Z had errors)" if any fail
```

**Success Criteria:**
- ✅ Timeout errors are distinct from other git errors
- ✅ Timeout is configurable via `GROVE_GIT_TIMEOUT_MS` env var
- ✅ TUI shows timeout indicator differently than other errors
- ✅ Error messages include suggestions for common issues
- ✅ `refresh_all()` returns success/failure counts
- ✅ Logging provides useful debugging information at debug/trace levels

---

## Phase 3D: Production Hardening (OPTIONAL - Can Defer)

**Priority:** MEDIUM
**Effort:** 3-4 hours
**Blocking:** No (nice to have, not critical for Slice 2)

### Issues Addressed
- [CRITICAL] No version output with build info
- [HIGH] No configuration validation on startup
- [MEDIUM] Path validation gaps
- [MEDIUM] No graceful handling of missing default config

### Tasks

**Task 1: Add Version Info with Build Metadata (1 hour)**

**Use `vergen` or custom build.rs:**

```rust
// build.rs
use std::process::Command;

fn main() {
    // Get git commit hash
    if let Ok(output) = Command::new("git").args(["rev-parse", "--short", "HEAD"]).output() {
        let git_hash = String::from_utf8(output.stdout).unwrap();
        println!("cargo:rustc-env=GIT_HASH={}", git_hash.trim());
    } else {
        println!("cargo:rustc-env=GIT_HASH=unknown");
    }

    // Build date
    println!("cargo:rustc-env=BUILD_DATE={}", chrono::Utc::now().format("%Y-%m-%d"));
}

// src/main.rs
const VERSION: &str = concat!(
    env!("CARGO_PKG_VERSION"),
    " (", env!("GIT_HASH"), ", built ", env!("BUILD_DATE"), ")"
);

#[derive(Parser, Debug)]
#[command(version = VERSION)]
struct Cli {
    // ...
}
```

**Or use `vergen` crate (recommended):**
```toml
[build-dependencies]
vergen = { version = "8", features = ["git", "build"] }
```

**Files:**
- `Cargo.toml` - Add build-dependencies
- `build.rs` - New file
- `src/main.rs` - Use extended version string

**Task 2: Add Config Validation (1 hour)**

```rust
// crates/grove-core/src/domain.rs
impl WorkspaceConfig {
    pub fn validate(&self) -> Result<()> {
        // Check 1: Non-empty repository list
        if self.repositories.is_empty() {
            return Err(CoreError::InvalidConfig {
                details: "Workspace has no repositories configured".to_string(),
            });
        }

        // Check 2: Duplicate paths
        let mut seen = std::collections::HashSet::new();
        for repo in &self.repositories {
            if !seen.insert(repo.path.as_path()) {
                log::warn!("Duplicate repository path: {}", repo.path);
                // Don't error, just warn - user might want same repo with different tags
            }
        }

        // Check 3: Validate paths are reasonable (optional, strict mode)
        for repo in &self.repositories {
            let path = repo.path.as_path();
            if path.as_os_str().is_empty() {
                return Err(CoreError::InvalidConfig {
                    details: format!("Repository path is empty: {:?}", repo),
                });
            }
        }

        Ok(())
    }
}

// src/main.rs
let config = loader.load_workspace(&config_path)?;
config.validate()?;  // Validate after loading
```

**Files:**
- `crates/grove-core/src/domain.rs` - Add `validate()` method
- `src/main.rs` - Call validation
- Add tests for validation logic

**Task 3: Improve Path Security (1 hour)**

```rust
// crates/grove-core/src/domain.rs
impl RepoPath {
    pub fn new(path: &str) -> Result<Self> {
        if path.trim().is_empty() {
            return Err(CoreError::EmptyRepoPath);
        }

        // Expand tilde and environment variables
        let expanded = shellexpand::full(path).map_err(|e| CoreError::InvalidRepoPath {
            path: format!("{path}: {e}"),
        })?;

        let mut path_buf = PathBuf::from(expanded.as_ref());

        // Security: Normalize path components (remove . and ..)
        // Don't use canonicalize() as it requires path to exist
        path_buf = normalize_path(&path_buf);

        // Optional: Warn if path is outside home directory (security policy)
        if let Some(home) = std::env::var_os("HOME") {
            let home = PathBuf::from(home);
            if !path_buf.starts_with(&home) {
                log::warn!("Repository path is outside home directory: {:?}", path_buf);
                // Don't error - might be intentional
            }
        }

        Ok(Self(path_buf))
    }
}

// Helper to normalize path without requiring it to exist
fn normalize_path(path: &Path) -> PathBuf {
    use std::path::Component;

    let mut components = Vec::new();
    for component in path.components() {
        match component {
            Component::CurDir => {},  // Skip .
            Component::ParentDir => {  // Process ..
                if !components.is_empty() {
                    components.pop();
                }
            }
            comp => components.push(comp),
        }
    }

    components.iter().collect()
}
```

**Files:**
- `crates/grove-core/src/domain.rs` - Add path normalization
- Add tests for path traversal attempts
- Document path security policy in ADR

**Task 4: Improve First-Run Experience (30 min)**

**Better default config error:**
```rust
// src/main.rs
let config = loader
    .load_workspace(&config_path)
    .or_else(|e| {
        if config_path.contains(".config/grove/workspace.yaml") {
            // User is using default path
            let is_not_found = e.to_string().contains("No such file")
                            || e.to_string().contains("not found");

            if is_not_found {
                eprintln!("Grove workspace config not found.\n");
                eprintln!("Expected location: {}\n", config_path);
                eprintln!("To get started:");
                eprintln!("  1. Create config directory:");
                eprintln!("     mkdir -p ~/.config/grove");
                eprintln!();
                eprintln!("  2. Create workspace config:");
                eprintln!("     cat > ~/.config/grove/workspace.yaml <<EOF");
                eprintln!("name: my-workspace");
                eprintln!("repositories:");
                eprintln!("  - path: ~/src/project1");
                eprintln!("    tags: [rust]");
                eprintln!("EOF");
                eprintln!();
                eprintln!("  3. Run grove:");
                eprintln!("     grove");
                eprintln!();
                eprintln!("Documentation: {}/README.md", env!("CARGO_PKG_REPOSITORY"));
            }
        }
        Err(e)
    })?;
```

**Or add `grove init` command:**
```rust
#[derive(Parser, Debug)]
enum Commands {
    /// Initialize a new workspace configuration
    Init {
        /// Path to create config file
        #[arg(default_value = "~/.config/grove/workspace.yaml")]
        path: String,
    },
    /// Run the TUI (default command)
    Run,
}

// Then implement init logic
fn run_init(path: &str) -> Result<()> {
    let expanded = shellexpand::full(path)?;
    let path = PathBuf::from(expanded.as_ref());

    if path.exists() {
        bail!("Config already exists: {}", path.display());
    }

    // Create parent directory
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Write template config
    let template = r#"name: my-workspace
repositories:
  - path: ~/src/example-repo
    tags: [example]
"#;
    std::fs::write(&path, template)?;

    println!("Created workspace config: {}", path.display());
    println!("\nEdit the file to add your repositories, then run:");
    println!("  grove");

    Ok(())
}
```

**Files:**
- `src/main.rs` - Improve default config error OR add `init` command
- `docs/user-guide.md` - Document init command if added

**Verification:**
```bash
# Test version info
grove --version
# Should show: grove 0.1.0 (abc1234, built 2026-02-10)

# Test config validation
echo "name: test\nrepositories: []" > /tmp/empty.yaml
grove --workspace /tmp/empty.yaml
# Should error: "Workspace has no repositories configured"

# Test path normalization
echo "name: test\nrepositories:\n  - path: ~/src/../.." > /tmp/path.yaml
grove --workspace /tmp/path.yaml
# Should normalize path (or warn about it)

# Test first-run UX
rm ~/.config/grove/workspace.yaml
grove
# Should show helpful instructions

# Or test init
grove init
# Should create template config
```

**Success Criteria:**
- ✅ Version output includes git hash and build date
- ✅ Config validation rejects empty repository lists
- ✅ Path normalization prevents traversal issues (or logs warnings)
- ✅ First-run experience is helpful with clear instructions
- ✅ Optional: `grove init` command creates template config

---

## Phase 4: Future Optimizations (DEFER TO LATER SLICES)

**Priority:** LOW
**Effort:** 8-12 hours
**Blocking:** No - defer to Slice 2+

### Deferred Tasks
These are valid improvements but not critical for Slice 1 completion:

1. **Parallel Git Queries** (4-5 hours)
   - Use `rayon` to query repos in parallel
   - Add progress bar during startup
   - Estimated speedup: 5-10x for large workspaces
   - **Defer to:** Performance optimization slice

2. **Persistent Status Caching** (3-4 hours)
   - Cache status to `~/.cache/grove/status.json`
   - TTL-based invalidation (5 min default)
   - Speeds up repeated runs
   - **Defer to:** Performance optimization slice

3. **Interactive Filtering/Search** (4-5 hours)
   - Add `/` key to search repos
   - Filter by tags
   - Fuzzy matching
   - **Defer to:** Slice 2 (detail pane) or later UX slice

4. **Repository Detail Pane** (6-8 hours)
   - Split screen layout
   - Show commit log for selected repo
   - Show changed files
   - **Defer to:** Slice 2 (already planned)

5. **Background Status Refresh** (3-4 hours)
   - Refresh status in background thread
   - Update TUI live
   - Watch for file changes
   - **Defer to:** Advanced TUI features slice

---

## Implementation Strategy

### Recommended Sequence

**Week 1: Critical Fixes**
- Day 1: Phase 3A - Documentation (3 hours)
- Day 2-3: Phase 3B - Testing (8 hours spread over 2 days)

**Week 2: Quality & Production**
- Day 4-5: Phase 3C - Error Handling & UX (6 hours spread over 2 days)
- Day 6: Phase 3D - Production Hardening (optional, 4 hours)

**Total Effort:** 17-21 hours (with optional hardening)

### Minimal Viable Improvements (Time Constrained)

If limited time, prioritize in this order:
1. ✅ Fix env var documentation (30 min) - MUST DO
2. ✅ Fix dirty status docs (15 min) - MUST DO
3. ✅ Add TUI tests (3 hours) - SHOULD DO
4. ✅ Add timeout error handling (2 hours) - SHOULD DO
5. ✅ Add troubleshooting docs (1 hour) - SHOULD DO
6. Everything else is NICE TO HAVE

**Minimum viable time:** 6.75 hours

---

## Success Metrics

### Before Starting Slice 2

**Required (Phase 3A-3B):**
- [ ] Environment variable support works OR documentation corrected
- [ ] Dirty status limitation removed from docs
- [ ] TUI has 7+ unit tests
- [ ] Git edge cases have 3+ tests
- [ ] Config validation has 4+ tests
- [ ] All tests pass (target: 30+ total tests)

**Strongly Recommended (Phase 3C):**
- [ ] Timeout errors are distinct and configurable
- [ ] Error messages include helpful suggestions
- [ ] Logging provides useful debug information
- [ ] `refresh_all()` returns success/failure stats

**Nice to Have (Phase 3D):**
- [ ] Version shows git hash and build date
- [ ] Config validation catches common mistakes
- [ ] Path normalization prevents security issues
- [ ] First-run UX is friendly

### Quality Gates

**Gate 1: Documentation Accuracy**
```bash
# All documented features must work
export GROVE_WORKSPACE=/tmp/test.yaml
grove  # Must load /tmp/test.yaml

# No false limitations in docs
grep -i "dirty status" docs/user-guide.md
# Should NOT say "not working" or "not implemented"
```

**Gate 2: Test Coverage**
```bash
cargo test | grep "test result"
# Should show 30+ tests passing

cargo tarpaulin --packages grove-core grove-engine grove
# Should show >80% coverage for core/engine
```

**Gate 3: User Experience**
```bash
# Error messages are helpful
rm ~/.config/grove/workspace.yaml
grove 2>&1 | grep -i "suggestion"
# Should include suggestions

# Logging works
RUST_LOG=grove=debug grove 2>&1 | grep -i "loaded workspace"
# Should show debug logs

# Timeout configurable
GROVE_GIT_TIMEOUT_MS=10000 grove
# Should use 10s timeout
```

---

## Risks & Mitigations

### Risk 1: Test Writing Takes Longer Than Estimated
- **Mitigation:** Start with TUI tests (highest value), defer config tests if needed
- **Fallback:** Merge partial test coverage, track remaining tests in TODO

### Risk 2: Timeout Redesign Breaks Existing Behavior
- **Mitigation:** Keep existing tests passing, add new tests for timeout case
- **Rollback Plan:** Timeout errors can be made non-breaking by catching and logging

### Risk 3: Documentation Changes Reveal More Issues
- **Mitigation:** Limit scope to fixing known issues, track new findings separately
- **Process:** Create separate tickets for new issues discovered during documentation update

---

## Conclusion

**Phase 3A-3C are strongly recommended before Slice 2** to ensure:
1. Documentation matches reality (user trust)
2. Core functionality is well-tested (maintainability)
3. Error handling is production-ready (user experience)

**Phase 3D can be deferred** without significant impact, though it improves production readiness.

**Estimated total effort: 12-16 hours** (core improvements)
**Minimum viable: 6.75 hours** (critical fixes only)

---

**Next Steps:**
1. Review this plan with stakeholder
2. Approve prioritization (3A → 3B → 3C → 3D?)
3. Execute phases in sequence
4. Update roadmap after completion
5. Begin Slice 2 planning with production-ready foundation
