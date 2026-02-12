---
status: working
date: 2026-02-12
context: Quality review of Grove->Graft command dispatch implementation (Slice 7)
---

# Command Dispatch Implementation - Quality Critique

## Executive Summary

**Overall Grade: B+** (Good implementation with some quality gaps)

The Grove->Graft command dispatch is **functionally complete** and works end-to-end. Users can execute commands from Grove TUI, see streaming output, and get exit codes. However, there are **8 quality issues** (2 critical, 3 high, 3 medium) that should be addressed.

**Critical Issues (Must Fix):** 2
**High Priority (Should Fix):** 3
**Medium Priority (Consider):** 3

---

## What Works Well ✅

1. **End-to-End Integration**
   - Grove successfully spawns `graft run <command>` subprocess
   - Streaming output works (stdout + stderr)
   - Exit codes propagate correctly
   - Error handling covers spawn failures

2. **Security Fix Complete**
   - Domain model uses `shlex.quote()` for argument escaping
   - Working directory validation in `run.py`
   - Environment variable type checking
   - Comprehensive subprocess error handling

3. **User Experience**
   - Command picker UI is intuitive ('x' key)
   - Real-time streaming output
   - Status messages show command state
   - Output scrolling works

4. **Test Coverage**
   - Graft: 421 tests passing (14 new for run command)
   - Grove: 77 tests passing
   - Good coverage of error paths

---

## Critical Issues (Must Fix)

### 🔴 Issue #1: Hardcoded "graft" Command - Path Discovery Failure

**Location**: `grove/src/tui.rs:1474`

**Problem**:
```rust
let result = Command::new("graft")
    .arg("run")
    .arg(&command_name)
```

**Issues**:
1. Assumes `graft` is in PATH
2. Development scenario breaks: `uv run python -m graft run` != `graft`
3. No fallback if graft not installed
4. Error message is unhelpful: "Failed to spawn graft: NotFound"

**Impact**:
- **Blocks development workflow** - Grove can't call graft during development
- **Confusing error for users** - "graft not found" when it's actually installed via uv
- **No clear recovery path** - User doesn't know how to fix it

**Fix**:
```rust
// Option 1: Check for uv-managed graft first
fn find_graft_command() -> Result<String> {
    // Try uv-managed installation first (development + production)
    if let Ok(output) = Command::new("uv")
        .args(&["run", "--quiet", "which", "graft"])
        .output()
    {
        if output.status.success() {
            if let Ok(path) = String::from_utf8(output.stdout) {
                let path = path.trim();
                if !path.is_empty() {
                    return Ok(format!("uv run graft"));
                }
            }
        }
    }

    // Fall back to PATH
    if Command::new("graft").arg("--version").output().is_ok() {
        return Ok("graft".to_string());
    }

    Err(anyhow!("graft not found in PATH or via uv"))
}

// Use in spawn_command:
let graft_cmd = find_graft_command()
    .map_err(|e| CommandEvent::Failed(format!(
        "Cannot find graft command: {e}\n\n\
         Install graft or ensure it's in PATH:\n\
         - Via uv: uv pip install graft\n\
         - System: pip install graft"
    )))?;

let result = if graft_cmd == "uv run graft" {
    Command::new("uv")
        .args(&["run", "graft", "run", &command_name])
        // ...
} else {
    Command::new(&graft_cmd)
        .args(&["run", &command_name])
        // ...
};
```

**Effort**: 45 minutes
**Priority**: Critical - blocks development

---

### 🔴 Issue #2: No Integration Test for End-to-End Command Dispatch

**Location**: Missing test coverage

**Problem**:
- No test that verifies Grove -> Graft -> Command execution works
- No test that `graft run` is called with correct arguments
- No test that output streaming works between processes
- Cannot detect if command dispatch breaks

**Evidence**:
```bash
# Grove tests: 77 tests
# - Unit tests for TUI components
# - Integration tests for workspace discovery
# - NO tests for command execution subprocess

# Graft tests: 421 tests
# - 14 tests for `graft run` CLI
# - NO tests called from Grove
```

**Impact**:
- Regressions in cross-repo integration go undetected
- No confidence that deployment works
- Manual testing required for every change

**Fix**:
```rust
// tests/integration/test_command_dispatch.rs
#[test]
fn test_grove_spawns_graft_successfully() {
    // Setup: Create test repo with graft.yaml
    let temp_dir = tempdir().unwrap();
    let graft_yaml = temp_dir.path().join("graft.yaml");
    fs::write(&graft_yaml, r#"
commands:
  test:
    run: echo "Hello from graft"
    description: Test command
"#).unwrap();

    // Execute: Spawn command like Grove does
    let (tx, rx) = mpsc::channel();
    spawn_command("test".to_string(), temp_dir.path().to_string_lossy().to_string(), tx);

    // Assert: Verify output received
    let mut output_lines = Vec::new();
    while let Ok(event) = rx.recv_timeout(Duration::from_secs(2)) {
        match event {
            CommandEvent::OutputLine(line) => output_lines.push(line),
            CommandEvent::Completed(code) => {
                assert_eq!(code, 0);
                break;
            }
            CommandEvent::Failed(msg) => panic!("Command failed: {}", msg),
        }
    }

    assert!(output_lines.iter().any(|line| line.contains("Hello from graft")));
}

#[test]
fn test_command_not_found_error_helpful() {
    let temp_dir = tempdir().unwrap();
    let graft_yaml = temp_dir.path().join("graft.yaml");
    fs::write(&graft_yaml, r#"
commands:
  nonexistent:
    run: this-command-does-not-exist
"#).unwrap();

    let (tx, rx) = mpsc::channel();
    spawn_command("nonexistent".to_string(), temp_dir.path().to_string_lossy().to_string(), tx);

    // Should get helpful error message
    match rx.recv_timeout(Duration::from_secs(2)) {
        Ok(CommandEvent::Failed(msg)) => {
            assert!(msg.contains("not found") || msg.contains("exit code"));
        }
        _ => panic!("Expected failure event"),
    }
}
```

**Effort**: 1 hour
**Priority**: Critical - no safety net

---

## High Priority Issues (Should Fix)

### 🟡 Issue #3: Output Truncation Loses Information

**Location**: `grove/src/tui.rs:MAX_OUTPUT_BYTES`

**Problem**:
```rust
const MAX_OUTPUT_BYTES: usize = 1_048_576; // 1MB

// In handle_command_events():
if self.output_bytes + line.len() > MAX_OUTPUT_BYTES {
    if !self.output_truncated {
        self.output_lines.push("... [output truncated at 1MB]".to_string());
        self.output_truncated = true;
    }
    continue; // SILENTLY DROPS REST OF OUTPUT
}
```

**Issues**:
1. User sees "[output truncated]" but can't access rest
2. Exit code may be at end of output (lost)
3. Error messages may be at end (lost)
4. No way to recover truncated data

**Example Failure**:
```bash
# Long test output (2MB)
$ grove -> execute "test"
[900KB of passing tests]
... [output truncated at 1MB]
[100KB more tests]
ERROR: 5 tests failed  # <- USER NEVER SEES THIS
```

**Fix Options**:

**Option A: Ring buffer (keep last N lines)**
```rust
const MAX_OUTPUT_LINES: usize = 10_000;

if self.output_lines.len() >= MAX_OUTPUT_LINES {
    // Keep last 9000 lines, drop oldest 1000
    self.output_lines.drain(0..1000);
    if !self.output_truncated {
        self.output_lines.insert(0, "... [earlier output truncated]".to_string());
        self.output_truncated = true;
    }
}
self.output_lines.push(line);
```
**Pro**: Always see recent output (including errors/exit)
**Con**: Lose beginning of output

**Option B: Save to temp file**
```rust
if self.output_bytes > MAX_OUTPUT_BYTES {
    if self.output_file.is_none() {
        let path = format!("/tmp/grove-output-{}.log", self.command_name.as_ref().unwrap());
        self.output_file = Some(path.clone());
        self.status_message = Some(StatusMessage::warning(
            format!("Output large, saved to {}", path)
        ));
    }
    // Write to file
    writeln!(self.output_file.as_ref().unwrap(), "{}", line)?;
}
```
**Pro**: No data loss
**Con**: More complex

**Recommendation**: Option A (ring buffer) - simpler, keeps critical end

**Effort**: 30 minutes
**Priority**: High - data loss is bad UX

---

### 🟡 Issue #4: No Command Cancellation

**Location**: `grove/src/tui.rs` - Missing feature

**Problem**:
- Long-running commands cannot be stopped
- Pressing 'q' closes output pane but command continues
- No way to send SIGTERM/SIGKILL
- Zombie processes accumulate

**Evidence from spec**:
```gherkin
# From command-execution.md (Open Questions):
"Should there be a stop confirmation when closing output pane with running command?"
```

**Current behavior**:
```rust
// User presses 'q' while command running
ActivePane::CommandOutput => {
    if matches!(key.code, KeyCode::Char('q')) {
        // Close pane, but command keeps running in background!
        self.active_pane = ActivePane::RepoList;
        self.command_event_rx = None; // Drop receiver, orphan thread
    }
}
```

**Fix**:
```rust
pub struct App<R, D> {
    // Add field to track child process
    running_command_pid: Option<u32>,
}

// In spawn_command, store PID:
fn spawn_command(...) {
    let mut child = Command::new("graft")...spawn()?;
    let pid = child.id();
    tx.send(CommandEvent::Started(pid))?;
    // ... rest of function
}

// In TUI, handle stop request:
ActivePane::CommandOutput => {
    if matches!(key.code, KeyCode::Char('q')) {
        if self.command_state == CommandState::Running {
            self.show_stop_confirmation = true; // New field
        } else {
            self.active_pane = ActivePane::RepoList;
        }
    }
}

// Render confirmation dialog
if self.show_stop_confirmation {
    // "Stop running command? (y/n)"
    // If 'y': send SIGTERM to PID
    if let Some(pid) = self.running_command_pid {
        #[cfg(unix)]
        {
            use nix::sys::signal::{kill, Signal};
            use nix::unistd::Pid;
            let _ = kill(Pid::from_raw(pid as i32), Signal::SIGTERM);
        }
    }
}
```

**Effort**: 1 hour
**Priority**: High - poor UX without cancellation

---

### 🟡 Issue #5: Error Messages Don't Follow Spec Format

**Location**: Multiple places

**Problem - Inconsistent with spec**:
```gherkin
# Spec says:
"✓ Command completed successfully"
"✗ Command failed with exit code N"

# Actual implementation:
Grove: Uses status messages (auto-dismiss after 3s)
Graft: Prints to terminal directly
```

**Evidence**:
```rust
// grove/src/tui.rs:409
CommandEvent::Completed(exit_code) => {
    if exit_code == 0 {
        self.status_message = Some(StatusMessage::success("Command completed"));
        // Missing: "✓" prefix, "successfully" word
    } else {
        self.status_message = Some(StatusMessage::error(
            format!("Command failed (exit {})", exit_code)
        ));
        // Missing: "✗" prefix, "with exit code" wording
    }
}
```

**Fix**:
```rust
CommandEvent::Completed(exit_code) => {
    self.command_state = CommandState::Completed(exit_code);

    if exit_code == 0 {
        // Add to output pane (not just status bar)
        self.output_lines.push("".to_string());
        self.output_lines.push("✓ Command completed successfully".to_string());
        self.status_message = Some(StatusMessage::success("Command completed successfully"));
    } else {
        // Add to output pane
        self.output_lines.push("".to_string());
        self.output_lines.push(format!("✗ Command failed with exit code {}", exit_code));
        self.status_message = Some(StatusMessage::error(
            format!("Command failed with exit code {}", exit_code)
        ));
    }
}
```

**Effort**: 15 minutes
**Priority**: High - spec compliance

---

## Medium Priority Issues

### 🟠 Issue #6: No Spec for Grove Domain Types

**Location**: `grove/crates/grove-core/src/domain.rs`

**Problem**:
- Added `Command`, `CommandState`, `GraftYaml` types
- No specification for these domain models
- No validation rules documented
- Inconsistent with graft specification patterns

**Evidence**:
```rust
// grove/crates/grove-core/src/domain.rs:28
pub struct Command {
    pub run: String,
    pub description: Option<String>,
    pub working_dir: Option<String>,
    pub env: Option<HashMap<String, String>>,
}
```

**Missing**:
- Spec defining Command structure
- Validation rules (max length, required fields)
- Relationship to `graft/domain/command.py`
- Decision record for duplication

**Fix**: Create `docs/specifications/grove/domain-models.md`:
```markdown
## Command

Represents an executable command from a repository's graft.yaml.

### Structure
- `run: String` - Shell command to execute (required, max 10KB)
- `description: Option<String>` - Human-readable description (max 500 chars)
- `working_dir: Option<String>` - Relative path from repo root
- `env: Option<HashMap<String, String>>` - Environment variables

### Validation
- `run` must not be empty
- `working_dir` must be relative (no absolute paths)
- `env` values must be valid UTF-8 strings

### Relationship to Graft
Grove's Command is a read-only subset of graft's Command model.
Grove parses graft.yaml but delegates execution to `graft run`.
```

**Effort**: 30 minutes
**Priority**: Medium - documentation debt

---

### 🟠 Issue #7: Scratch Documents in Root Directory

**Location**: Root directory

**Problem**:
```bash
$ ls *.md
FINAL-SUMMARY.md
IMPLEMENTATION-COMPLETE.md
STATUS-BAR-CRITIQUE.md
STATUS-BAR-IMPLEMENTATION.md
STATUS-BAR-IMPROVEMENTS-PLAN.md
STATUS-BAR-SUMMARY.md
grove-status-bar-design.md
grove-status-bar-test.md
grove-tui-test-guide.md
implementation-status.md
pr-description.md
template-status.md
```

**Issues**:
- Violates meta-knowledge-base patterns (ephemeral docs in root)
- Should be in `notes/archive/2026-02-12-*.md`
- `grove-tui-test-guide.md` should be in `docs/guides/`
- Obsolete files should be removed

**Fix**: (Attempted earlier, interrupted)
```bash
# Move to notes/archive
git mv FINAL-SUMMARY.md notes/archive/2026-02-12-grove-slice-7-summary.md
# ... (9 files total)

# Move testing guide to durable docs
git mv grove-tui-test-guide.md docs/guides/grove-tui-testing.md

# Remove obsolete
git rm pr-description.md template-status.md
```

**Effort**: 10 minutes
**Priority**: Medium - organization quality

---

### 🟠 Issue #8: Missing Grove Tests for TUI State

**Location**: `grove/src/tui_tests.rs`

**Problem**:
```rust
// Current tests (14 tests):
- test_app_creation
- test_repo_selection
- test_detail_pane_loading
- test_help_overlay
// ... only UI state tests

// MISSING:
- test_command_picker_state
- test_command_execution_state
- test_output_pane_scrolling
- test_command_cancellation
```

**Impact**:
- Command execution UI not tested
- State machine transitions not verified
- Edge cases not covered

**Fix**:
```rust
#[test]
fn test_command_picker_opens_and_closes() {
    let mut app = create_test_app();
    app.repos.insert(0, create_test_repo());

    // Open picker
    app.handle_key(KeyCode::Char('x'));
    assert_eq!(app.active_pane, ActivePane::CommandPicker);

    // Close picker
    app.handle_key(KeyCode::Char('q'));
    assert_eq!(app.active_pane, ActivePane::RepoList);
}

#[test]
fn test_output_scroll_clamping() {
    let mut app = create_test_app();
    app.output_lines = vec!["line1".into(), "line2".into()];
    app.output_scroll = 0;

    // Can't scroll past end
    app.handle_key(KeyCode::Char('j')); // down
    app.handle_key(KeyCode::Char('j'));
    app.handle_key(KeyCode::Char('j')); // Should clamp
    assert!(app.output_scroll <= 2);

    // Can't scroll before start
    app.handle_key(KeyCode::Char('k')); // up
    app.handle_key(KeyCode::Char('k'));
    app.handle_key(KeyCode::Char('k'));
    app.handle_key(KeyCode::Char('k')); // Should clamp
    assert_eq!(app.output_scroll, 0);
}
```

**Effort**: 1 hour
**Priority**: Medium - safety net for future changes

---

## Summary of Fixes

### Must Fix (Critical)
1. ⚠️ **Hardcoded graft command** - 45 min - Blocks development
2. ⚠️ **No integration tests** - 1 hour - No safety net

### Should Fix (High)
3. ⚠️ **Output truncation loses data** - 30 min - Poor UX
4. ⚠️ **No command cancellation** - 1 hour - Poor UX
5. ⚠️ **Error messages don't match spec** - 15 min - Spec compliance

### Consider (Medium)
6. 📋 **No spec for Grove domain types** - 30 min - Documentation debt
7. 📋 **Scratch docs in root** - 10 min - Organization
8. 📋 **Missing TUI state tests** - 1 hour - Testing gaps

---

## Total Estimated Effort

**Critical fixes**: 1h 45min
**High priority**: 1h 45min
**Medium priority**: 2h 10min

**Total**: 5h 40min for all issues

**Recommended first phase**: Critical + High (3h 30min)

---

## Verification Checklist

After fixes:
- [ ] Integration test passes (Grove -> Graft -> Command)
- [ ] `uv run` development workflow works
- [ ] Command cancellation works (SIGTERM sent)
- [ ] Output ring buffer prevents data loss
- [ ] Error messages match specification format
- [ ] All tests pass (Graft: 421, Grove: 77+)
- [ ] Scratch documents organized
- [ ] Grove domain types documented

---

## Positive Aspects ✅

1. **Security is solid** - shlex escaping, validation, error handling
2. **Core functionality works** - end-to-end integration successful
3. **Good test coverage** - 498 total tests passing
4. **Spec-driven development** - command-execution.md guides implementation
5. **Clean architecture** - protocol-based, trait boundaries
6. **Responsive UI** - async execution, non-blocking

---

## Conclusion

The command dispatch implementation is **functionally complete and secure**, but has **quality gaps** that affect:
- **Development workflow** (Issue #1 - critical)
- **Test confidence** (Issue #2 - critical)
- **User experience** (Issues #3, #4, #5)

**Recommendation**: Fix critical issues first (1h 45min), then high priority (1h 45min). This gets to production-ready quality in ~3.5 hours total.

**Status**: B+ → A- with critical fixes, A with all fixes
