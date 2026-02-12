---
status: working
date: 2026-02-12
context: Improvement plan based on command dispatch critique
---

# Command Dispatch Improvements - Implementation Plan

## Overview

Based on the quality critique, this plan addresses 8 issues in prioritized phases.

**Total effort**: 5h 40min
**Recommended Phase 1**: 3h 30min (Critical + High priority)

---

## Phase 1: Critical Fixes (1h 45min)

### Task 1.1: Fix Hardcoded Graft Command Path ⚠️ CRITICAL

**Problem**: Grove hardcodes `Command::new("graft")`, breaking `uv run` workflow

**File**: `grove/src/tui.rs`

**Implementation**:

```rust
// Add helper function at module level
fn find_graft_command() -> Result<String> {
    // Try uv-managed installation (development + production)
    if let Ok(output) = std::process::Command::new("uv")
        .args(&["run", "--quiet", "python", "-m", "graft", "--version"])
        .output()
    {
        if output.status.success() {
            return Ok("uv run python -m graft".to_string());
        }
    }

    // Fall back to PATH
    if std::process::Command::new("graft")
        .arg("--version")
        .output()
        .is_ok()
    {
        return Ok("graft".to_string());
    }

    anyhow::bail!(
        "graft command not found\n\n\
         Install graft to enable command execution:\n\
         - Via uv: uv pip install graft\n\
         - System: pip install graft\n\n\
         Or ensure graft is in your PATH"
    )
}

// Update spawn_command function
fn spawn_command(command_name: String, repo_path: String, tx: Sender<CommandEvent>) {
    use std::io::{BufRead, BufReader};
    use std::process::{Command, Stdio};

    // Find graft command
    let graft_cmd = match find_graft_command() {
        Ok(cmd) => cmd,
        Err(e) => {
            let _ = tx.send(CommandEvent::Failed(e.to_string()));
            return;
        }
    };

    // Spawn appropriate command
    let result = if graft_cmd.starts_with("uv run") {
        // Split "uv run python -m graft" into parts
        Command::new("uv")
            .args(&["run", "python", "-m", "graft", "run", &command_name])
            .current_dir(&repo_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
    } else {
        Command::new(&graft_cmd)
            .args(&["run", &command_name])
            .current_dir(&repo_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
    };

    let mut child = match result {
        Ok(child) => child,
        Err(e) => {
            let _ = tx.send(CommandEvent::Failed(format!(
                "Failed to spawn {}: {}\n\n\
                 Ensure graft is installed and in PATH",
                graft_cmd, e
            )));
            return;
        }
    };

    // ... rest of function unchanged
}
```

**Testing**:
```bash
# Verify both modes work
cd /home/coder/src/graft

# Mode 1: uv run (development)
uv run python -m graft --version  # Should work
cargo run -- --workspace test-workspace.yaml
# Press 'x', execute command

# Mode 2: System graft (after install)
pip install -e .
graft --version  # Should work
cargo run -- --workspace test-workspace.yaml
# Press 'x', execute command
```

**Acceptance Criteria**:
- [ ] `uv run python -m graft` mode works
- [ ] System `graft` mode works
- [ ] Helpful error if neither available
- [ ] Error message explains how to fix

**Effort**: 45 minutes

---

### Task 1.2: Add Integration Test for Command Dispatch ⚠️ CRITICAL

**Problem**: No test verifies Grove -> Graft -> execution works

**File**: `grove/tests/integration/test_command_dispatch.rs` (NEW)

**Implementation**:

```rust
//! Integration tests for command dispatch from Grove to Graft.

use grove_core::{CommandEvent};
use std::fs;
use std::sync::mpsc;
use std::time::Duration;
use tempfile::tempdir;

// Re-export spawn_command from tui module for testing
// (Need to make it pub(crate) first)
use grove::tui::spawn_command;

#[test]
fn test_spawn_graft_command_successfully() {
    // Setup: Create test repository with graft.yaml
    let temp_dir = tempdir().unwrap();
    let graft_yaml = temp_dir.path().join("graft.yaml");
    fs::write(
        &graft_yaml,
        r#"
commands:
  test-hello:
    run: echo "Hello from graft command"
    description: Test command that echoes
"#,
    )
    .unwrap();

    // Execute: Spawn command like Grove TUI does
    let (tx, rx) = mpsc::channel();
    let repo_path = temp_dir.path().to_string_lossy().to_string();

    spawn_command("test-hello".to_string(), repo_path, tx);

    // Assert: Collect output and verify
    let mut output_lines = Vec::new();
    let mut exit_code = None;

    while let Ok(event) = rx.recv_timeout(Duration::from_secs(5)) {
        match event {
            CommandEvent::OutputLine(line) => {
                output_lines.push(line);
            }
            CommandEvent::Completed(code) => {
                exit_code = Some(code);
                break;
            }
            CommandEvent::Failed(msg) => {
                panic!("Command failed: {}", msg);
            }
        }
    }

    // Verify success
    assert_eq!(exit_code, Some(0), "Command should complete successfully");
    assert!(
        output_lines
            .iter()
            .any(|line| line.contains("Hello from graft")),
        "Output should contain expected text. Got: {:?}",
        output_lines
    );
}

#[test]
fn test_graft_command_not_found_helpful_error() {
    // Setup: Create repo with command that doesn't exist
    let temp_dir = tempdir().unwrap();
    let graft_yaml = temp_dir.path().join("graft.yaml");
    fs::write(
        &graft_yaml,
        r#"
commands:
  nonexistent:
    run: this-command-absolutely-does-not-exist-12345
    description: Command that will fail
"#,
    )
    .unwrap();

    // Execute
    let (tx, rx) = mpsc::channel();
    let repo_path = temp_dir.path().to_string_lossy().to_string();

    spawn_command("nonexistent".to_string(), repo_path, tx);

    // Assert: Should get helpful error or failure exit code
    let mut got_failure = false;

    while let Ok(event) = rx.recv_timeout(Duration::from_secs(5)) {
        match event {
            CommandEvent::Failed(msg) => {
                // Helpful error from graft
                assert!(
                    msg.contains("not found") || msg.contains("Failed"),
                    "Error should be helpful: {}",
                    msg
                );
                got_failure = true;
                break;
            }
            CommandEvent::Completed(code) => {
                // Non-zero exit code is also acceptable
                assert_ne!(code, 0, "Command should fail");
                got_failure = true;
                break;
            }
            CommandEvent::OutputLine(_) => {
                // Keep collecting
            }
        }
    }

    assert!(got_failure, "Should receive failure indication");
}

#[test]
fn test_graft_not_in_path_error() {
    // This test verifies the error when graft is not installed
    // Skip if graft IS installed (test would pass incorrectly)
    if std::process::Command::new("graft")
        .arg("--version")
        .output()
        .is_ok()
    {
        eprintln!("Skipping test: graft is installed");
        return;
    }

    let temp_dir = tempdir().unwrap();
    let graft_yaml = temp_dir.path().join("graft.yaml");
    fs::write(&graft_yaml, "commands:\n  test:\n    run: echo hi\n").unwrap();

    let (tx, rx) = mpsc::channel();
    spawn_command("test".to_string(), temp_dir.path().to_string_lossy().to_string(), tx);

    // Should get helpful "graft not found" error
    match rx.recv_timeout(Duration::from_secs(2)) {
        Ok(CommandEvent::Failed(msg)) => {
            assert!(
                msg.contains("graft") && msg.contains("not found"),
                "Error should mention graft not found: {}",
                msg
            );
        }
        other => panic!("Expected Failed event with graft not found, got: {:?}", other),
    }
}
```

**Changes needed**:
1. Make `spawn_command` visible to tests: `pub(crate) fn spawn_command`
2. Add to `grove/tests/integration/mod.rs`
3. Update `grove/src/lib.rs` to expose for tests

**Testing**:
```bash
cargo test --test test_command_dispatch
```

**Acceptance Criteria**:
- [ ] Test for successful command execution passes
- [ ] Test for command not found passes
- [ ] Test for graft not in PATH passes
- [ ] All 3 tests run in < 10 seconds

**Effort**: 1 hour

---

## Phase 2: High Priority Fixes (1h 45min)

### Task 2.1: Implement Output Ring Buffer ⚠️ HIGH

**Problem**: Output truncation at 1MB loses end of output (where errors often are)

**File**: `grove/src/tui.rs`

**Implementation**:

```rust
// Update constants
const MAX_OUTPUT_LINES: usize = 10_000; // ~1MB assuming 100 chars/line

// Update App struct
pub struct App<R, D> {
    // Replace output_bytes tracking
    output_lines: Vec<String>,
    output_scroll: usize,
    output_truncated_start: bool, // New: track if we dropped lines from start
    // Remove: output_bytes, output_truncated
}

// Update handle_command_events
fn handle_command_events(&mut self) {
    if let Some(rx) = &self.command_event_rx {
        while let Ok(event) = rx.try_recv() {
            match event {
                CommandEvent::OutputLine(line) => {
                    // Add line to buffer
                    self.output_lines.push(line);

                    // If buffer too large, drop oldest lines (ring buffer)
                    if self.output_lines.len() > MAX_OUTPUT_LINES {
                        let drop_count = 1000; // Drop 1000 lines at a time
                        self.output_lines.drain(0..drop_count);

                        // Add truncation marker if first time
                        if !self.output_truncated_start {
                            self.output_lines.insert(
                                0,
                                "... [earlier output truncated - showing last 10,000 lines]".to_string()
                            );
                            self.output_truncated_start = true;

                            // Show warning in status bar
                            self.status_message = Some(StatusMessage::warning(
                                "Output large - showing last 10,000 lines"
                            ));
                        }

                        // Adjust scroll to stay in bounds
                        if self.output_scroll > drop_count {
                            self.output_scroll -= drop_count;
                        } else {
                            self.output_scroll = 0;
                        }
                    }
                }
                // ... rest unchanged
            }
        }
    }
}
```

**Testing**:
```bash
# Create command that outputs >10,000 lines
echo 'commands:
  spam:
    run: seq 1 15000
' > test-graft.yaml

# Run and verify:
# - Shows "earlier output truncated" message
# - Shows lines 5000-15000
# - Can scroll through all visible lines
# - Exit code still visible at end
```

**Acceptance Criteria**:
- [ ] Keeps last 10,000 lines
- [ ] Shows truncation warning
- [ ] Exit code always visible
- [ ] Scroll position adjusted correctly
- [ ] Status bar shows warning

**Effort**: 30 minutes

---

### Task 2.2: Implement Command Cancellation ⚠️ HIGH

**Problem**: No way to stop long-running commands

**Files**: `grove/src/tui.rs`, `grove/Cargo.toml`

**Implementation**:

```rust
// Add to Cargo.toml
[target.'cfg(unix)'.dependencies]
nix = { version = "0.27", features = ["signal"] }

// Update CommandEvent enum
#[derive(Debug)]
enum CommandEvent {
    Started(u32), // NEW: PID of spawned process
    OutputLine(String),
    Completed(i32),
    Failed(String),
}

// Update App struct
pub struct App<R, D> {
    running_command_pid: Option<u32>, // NEW
    show_stop_confirmation: bool,     // NEW
    // ... rest unchanged
}

// Update spawn_command to send PID
fn spawn_command(command_name: String, repo_path: String, tx: Sender<CommandEvent>) {
    // ... spawn logic ...

    let mut child = match result {
        Ok(child) => child,
        Err(e) => {
            let _ = tx.send(CommandEvent::Failed(format!("Failed to spawn: {}", e)));
            return;
        }
    };

    // Send PID first
    let _ = tx.send(CommandEvent::Started(child.id()));

    // ... rest of capture logic ...
}

// Update handle_command_events
fn handle_command_events(&mut self) {
    if let Some(rx) = &self.command_event_rx {
        while let Ok(event) = rx.try_recv() {
            match event {
                CommandEvent::Started(pid) => {
                    self.running_command_pid = Some(pid);
                }
                // ... rest unchanged
            }
        }
    }
}

// Update key handling for output pane
fn handle_key(&mut self, key: KeyEvent) {
    match self.active_pane {
        ActivePane::CommandOutput => {
            if self.show_stop_confirmation {
                // Handle stop confirmation dialog
                match key.code {
                    KeyCode::Char('y') | KeyCode::Char('Y') => {
                        // Send SIGTERM
                        if let Some(pid) = self.running_command_pid {
                            #[cfg(unix)]
                            {
                                use nix::sys::signal::{kill, Signal};
                                use nix::unistd::Pid;
                                if let Err(e) = kill(Pid::from_raw(pid as i32), Signal::SIGTERM) {
                                    self.status_message = Some(StatusMessage::error(
                                        format!("Failed to stop command: {}", e)
                                    ));
                                } else {
                                    self.status_message = Some(StatusMessage::info("Stopping command..."));
                                }
                            }
                            #[cfg(not(unix))]
                            {
                                self.status_message = Some(StatusMessage::warning(
                                    "Command cancellation not supported on Windows"
                                ));
                            }

                            self.running_command_pid = None;
                        }
                        self.show_stop_confirmation = false;
                        self.active_pane = ActivePane::RepoList;
                        self.command_event_rx = None;
                    }
                    KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                        // Cancel stop
                        self.show_stop_confirmation = false;
                    }
                    _ => {}
                }
            } else {
                // Normal output pane handling
                match key.code {
                    KeyCode::Char('q') => {
                        if self.command_state == CommandState::Running {
                            // Show confirmation
                            self.show_stop_confirmation = true;
                        } else {
                            // Command finished, just close
                            self.active_pane = ActivePane::RepoList;
                            self.command_event_rx = None;
                            self.running_command_pid = None;
                        }
                    }
                    // ... rest of output pane keys
                }
            }
        }
        // ... other panes
    }
}

// Add render_stop_confirmation method
fn render_stop_confirmation(&self, frame: &mut ratatui::Frame, area: Rect) {
    // Center a confirmation dialog
    let dialog_width = 50;
    let dialog_height = 5;
    let x = (area.width.saturating_sub(dialog_width)) / 2;
    let y = (area.height.saturating_sub(dialog_height)) / 2;

    let dialog_area = Rect::new(x, y, dialog_width, dialog_height);

    // Clear background
    frame.render_widget(Clear, dialog_area);

    // Render dialog
    let text = vec![
        Line::from(""),
        Line::from(Span::styled(
            "Stop running command?",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "y = Yes, stop   n = No, continue   Esc = Cancel",
            Style::default().fg(Color::Gray),
        )),
    ];

    let dialog = Paragraph::new(text)
        .block(Block::default().borders(Borders::ALL).border_style(
            Style::default().fg(Color::Yellow),
        ))
        .alignment(Alignment::Center);

    frame.render_widget(dialog, dialog_area);
}

// Call from render() when needed
if self.show_stop_confirmation {
    self.render_stop_confirmation(frame, frame.area());
}
```

**Testing**:
```bash
# Create long-running command
echo 'commands:
  slow:
    run: sleep 30
' > test-graft.yaml

# Run Grove, execute "slow", press 'q'
# Should see: "Stop running command? (y/n)"
# Press 'y', verify command stops
# Check with `ps aux | grep sleep` - should be gone
```

**Acceptance Criteria**:
- [ ] Pressing 'q' during running command shows confirmation
- [ ] Pressing 'y' sends SIGTERM and closes pane
- [ ] Pressing 'n' or Esc cancels and continues command
- [ ] Pressing 'q' after completion closes immediately
- [ ] Status message shows "Stopping command..."
- [ ] Works on Unix (gracefully fails on Windows with message)

**Effort**: 1 hour

---

### Task 2.3: Fix Error Message Format to Match Spec ⚠️ HIGH

**Problem**: Messages don't match spec format

**File**: `grove/src/tui.rs`

**Implementation**:

```rust
// Update handle_command_events
fn handle_command_events(&mut self) {
    if let Some(rx) = &self.command_event_rx {
        while let Ok(event) = rx.try_recv() {
            match event {
                CommandEvent::Completed(exit_code) => {
                    self.command_state = CommandState::Completed(exit_code);

                    // Add completion message to output (not just status bar)
                    self.output_lines.push("".to_string());

                    if exit_code == 0 {
                        // Match spec: "✓ Command completed successfully"
                        let unicode = supports_unicode();
                        let symbol = if unicode { "✓" } else { "*" };
                        self.output_lines.push(format!("{} Command completed successfully", symbol));

                        self.status_message = Some(StatusMessage::success(
                            "Command completed successfully"
                        ));
                    } else {
                        // Match spec: "✗ Command failed with exit code N"
                        let unicode = supports_unicode();
                        let symbol = if unicode { "✗" } else { "X" };
                        self.output_lines.push(format!(
                            "{} Command failed with exit code {}",
                            symbol,
                            exit_code
                        ));

                        self.status_message = Some(StatusMessage::error(format!(
                            "Command failed with exit code {}",
                            exit_code
                        )));
                    }

                    // Scroll to bottom to show completion message
                    if self.output_lines.len() > 0 {
                        let visible_height = 20; // Approximate, will be corrected on render
                        if self.output_lines.len() > visible_height {
                            self.output_scroll = self.output_lines.len() - visible_height;
                        }
                    }
                }
                // ... rest unchanged
            }
        }
    }
}
```

**Testing**:
```bash
# Test success case
echo 'commands:
  success:
    run: echo "works"
' > test-graft.yaml
# Output should end with: "✓ Command completed successfully"

# Test failure case
echo 'commands:
  fail:
    run: exit 42
' > test-graft.yaml
# Output should end with: "✗ Command failed with exit code 42"
```

**Acceptance Criteria**:
- [ ] Success shows: "✓ Command completed successfully"
- [ ] Failure shows: "✗ Command failed with exit code N"
- [ ] Message added to output pane (not just status bar)
- [ ] Auto-scroll to show completion message
- [ ] Unicode fallback works (*, X on ASCII terminals)

**Effort**: 15 minutes

---

## Phase 3: Medium Priority (2h 10min)

### Task 3.1: Document Grove Domain Types 📋 MEDIUM

**Problem**: No spec for Command, CommandState, GraftYaml types

**File**: `docs/specifications/grove/domain-models.md` (NEW)

**Content**: See critique for full content outline

**Effort**: 30 minutes

---

### Task 3.2: Organize Scratch Documents 📋 MEDIUM

**Problem**: Temporary docs in root directory

**Implementation**: Move files to proper locations per meta-knowledge-base patterns

**Effort**: 10 minutes

---

### Task 3.3: Add TUI State Tests 📋 MEDIUM

**Problem**: Command execution UI state not tested

**File**: `grove/src/tui_tests.rs`

**Implementation**: Add 5-6 tests covering command picker, output pane, scrolling

**Effort**: 1 hour

---

## Implementation Order

### Week 1: Critical Fixes
**Day 1-2**: Task 1.1 - Graft command discovery (45 min)
**Day 2-3**: Task 1.2 - Integration tests (1 hour)

**Checkpoint**: Verify development workflow works

### Week 1-2: High Priority
**Day 3**: Task 2.1 - Output ring buffer (30 min)
**Day 4**: Task 2.2 - Command cancellation (1 hour)
**Day 4**: Task 2.3 - Error message format (15 min)

**Checkpoint**: Verify UX improvements, run all tests

### Week 2: Medium Priority (Optional)
**Day 5**: Task 3.1 - Document domain types (30 min)
**Day 5**: Task 3.2 - Organize docs (10 min)
**Day 6**: Task 3.3 - TUI state tests (1 hour)

**Checkpoint**: Full test suite, documentation complete

---

## Verification Strategy

After each phase:

```bash
# 1. All tests pass
cargo test --all
uv run pytest
# Target: 498+ tests (Grove 77+, Graft 421)

# 2. Integration test works
cargo test --test test_command_dispatch
# Target: 3 tests passing

# 3. Manual smoke test
cd /tmp/test-workspace
echo 'commands:
  test:
    run: echo "Hello from command"
' > graft.yaml

# Via uv (development)
cd /home/coder/src/graft
cargo run -- --workspace /tmp/test-workspace/workspace.yaml
# Press 'x', execute "test", verify output

# Via installed graft (production)
pip install -e .
cargo run -- --workspace /tmp/test-workspace/workspace.yaml
# Press 'x', execute "test", verify output

# 4. Command cancellation
echo 'commands:
  slow:
    run: sleep 30
' > graft.yaml
cargo run -- --workspace /tmp/test-workspace/workspace.yaml
# Press 'x', execute "slow", press 'q', press 'y'
# Verify: command stops, no zombie process

# 5. Output ring buffer
echo 'commands:
  spam:
    run: seq 1 15000
' > graft.yaml
cargo run -- --workspace /tmp/test-workspace/workspace.yaml
# Press 'x', execute "spam"
# Verify: Shows truncation warning, can scroll, see end
```

---

## Success Criteria

### After Phase 1 (Critical)
- ✅ Development workflow works (`uv run python -m graft`)
- ✅ Production workflow works (system `graft`)
- ✅ Integration tests pass (3 tests)
- ✅ Helpful error if graft not found

### After Phase 2 (High Priority)
- ✅ Output ring buffer prevents data loss
- ✅ Command cancellation works (SIGTERM)
- ✅ Error messages match spec format
- ✅ All 498+ tests passing

### After Phase 3 (Medium Priority)
- ✅ Grove domain types documented
- ✅ Scratch documents organized
- ✅ TUI state tests added (5-6 tests)
- ✅ 100% compliance with meta-knowledge-base patterns

---

## Grade Progression

- **Current**: B+ (functionally complete, quality gaps)
- **After Phase 1**: A- (development unblocked, tests added)
- **After Phase 2**: A (production-ready UX)
- **After Phase 3**: A+ (exemplary quality)

---

## Notes

- Phase 1 is **blocking** for development - prioritize first
- Phase 2 improves UX significantly - recommended
- Phase 3 is polish - can defer if time constrained

**Estimated total**: 5h 40min for all phases
**Recommended minimum**: 3h 30min (Phases 1 + 2)
