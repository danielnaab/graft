//! Integration tests for command dispatch from Grove to Graft.
//!
//! These tests verify the end-to-end flow: Grove spawns graft subprocess,
//! captures output, handles errors, and completes successfully.

use std::fs;
use std::sync::mpsc;
use std::time::Duration;
use tempfile::tempdir;

// Import from grove crate (needs to be pub(crate) in tui.rs)
use grove::tui::{spawn_command, CommandEvent};

#[test]
fn test_spawn_graft_command_successfully() {
    // Setup: Create test repository with graft.yaml
    let temp_dir = tempdir().unwrap();
    let graft_yaml = temp_dir.path().join("graft.yaml");
    fs::write(
        &graft_yaml,
        r#"apiVersion: graft/v1beta1
name: test-repo
description: Test repository for command dispatch

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

    // Spawn in thread to avoid blocking
    std::thread::spawn(move || {
        spawn_command("test-hello".to_string(), Vec::new(), repo_path, tx);
    });

    // Assert: Collect output and verify
    let mut output_lines = Vec::new();
    let mut exit_code = None;
    let timeout = Duration::from_secs(10);
    let start = std::time::Instant::now();

    while start.elapsed() < timeout {
        match rx.recv_timeout(Duration::from_millis(100)) {
            Ok(CommandEvent::Started(_pid)) => {
                // Process started, continue waiting for output
            }
            Ok(CommandEvent::OutputLine(line)) => {
                output_lines.push(line);
            }
            Ok(CommandEvent::Completed(code)) => {
                exit_code = Some(code);
                break;
            }
            Ok(CommandEvent::Failed(msg)) => {
                panic!("Command failed: {}", msg);
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                // Keep waiting
                continue;
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                break;
            }
        }
    }

    // Verify success
    assert_eq!(
        exit_code,
        Some(0),
        "Command should complete successfully. Output: {:?}",
        output_lines
    );
    assert!(
        output_lines
            .iter()
            .any(|line| line.contains("Hello from graft")),
        "Output should contain expected text. Got: {:?}",
        output_lines
    );

    // Verify we got the graft output format (shows command execution)
    let output_text = output_lines.join("\n");
    assert!(
        output_text.contains("Executing:") || output_text.contains("Hello from graft"),
        "Should contain graft execution output"
    );
}

#[test]
fn test_command_not_found_in_graft_yaml() {
    // Setup: Create repo with graft.yaml but request nonexistent command
    let temp_dir = tempdir().unwrap();
    let graft_yaml = temp_dir.path().join("graft.yaml");
    fs::write(
        &graft_yaml,
        r#"apiVersion: graft/v1beta1
name: test-repo
description: Test repository

commands:
  existing-command:
    run: echo "exists"
"#,
    )
    .unwrap();

    // Execute: Request command that doesn't exist
    let (tx, rx) = mpsc::channel();
    let repo_path = temp_dir.path().to_string_lossy().to_string();

    std::thread::spawn(move || {
        spawn_command("nonexistent-command".to_string(), Vec::new(), repo_path, tx);
    });

    // Assert: Should get failure or non-zero exit
    let mut got_failure = false;
    let timeout = Duration::from_secs(10);
    let start = std::time::Instant::now();

    while start.elapsed() < timeout {
        match rx.recv_timeout(Duration::from_millis(100)) {
            Ok(CommandEvent::Started(_)) => {
                // Process started, continue
            }
            Ok(CommandEvent::Failed(msg)) => {
                // Graft should report command not found
                assert!(
                    msg.contains("not found") || msg.contains("Command"),
                    "Error should mention command not found: {}",
                    msg
                );
                got_failure = true;
                break;
            }
            Ok(CommandEvent::Completed(code)) => {
                // Non-zero exit code is also acceptable
                assert_ne!(code, 0, "Command should fail with non-zero exit");
                got_failure = true;
                break;
            }
            Ok(CommandEvent::OutputLine(_)) => {
                // Keep collecting
            }
            Err(mpsc::RecvTimeoutError::Timeout) => continue,
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }

    assert!(got_failure, "Should receive failure indication");
}

#[test]
fn test_command_execution_failure() {
    // Setup: Create command that exits with non-zero code
    let temp_dir = tempdir().unwrap();
    let graft_yaml = temp_dir.path().join("graft.yaml");
    fs::write(
        &graft_yaml,
        r#"apiVersion: graft/v1beta1
name: test-repo
description: Test repository

commands:
  failing-command:
    run: exit 42
    description: Command that fails
"#,
    )
    .unwrap();

    // Execute
    let (tx, rx) = mpsc::channel();
    let repo_path = temp_dir.path().to_string_lossy().to_string();

    std::thread::spawn(move || {
        spawn_command("failing-command".to_string(), Vec::new(), repo_path, tx);
    });

    // Assert: Should complete with exit code 42
    let mut exit_code = None;
    let timeout = Duration::from_secs(10);
    let start = std::time::Instant::now();

    while start.elapsed() < timeout {
        match rx.recv_timeout(Duration::from_millis(100)) {
            Ok(CommandEvent::Started(_)) => {
                // Process started, continue
            }
            Ok(CommandEvent::Completed(code)) => {
                exit_code = Some(code);
                break;
            }
            Ok(CommandEvent::Failed(msg)) => {
                // Also acceptable - might fail before getting exit code
                println!("Got failure: {}", msg);
                break;
            }
            Ok(CommandEvent::OutputLine(_)) => {
                // Keep collecting
            }
            Err(mpsc::RecvTimeoutError::Timeout) => continue,
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }

    // Verify we got either exit code 42 or a failure
    if let Some(code) = exit_code {
        assert_eq!(code, 42, "Should get exit code 42 from failed command");
    }
    // If we got Failed instead, that's also acceptable
}

#[test]
fn test_multiline_output_captured() {
    // Setup: Command that outputs multiple lines
    let temp_dir = tempdir().unwrap();
    let graft_yaml = temp_dir.path().join("graft.yaml");
    fs::write(
        &graft_yaml,
        r#"apiVersion: graft/v1beta1
name: test-repo
description: Test repository

commands:
  multiline:
    run: printf "Line 1\nLine 2\nLine 3\n"
    description: Multi-line output
"#,
    )
    .unwrap();

    // Execute
    let (tx, rx) = mpsc::channel();
    let repo_path = temp_dir.path().to_string_lossy().to_string();

    std::thread::spawn(move || {
        spawn_command("multiline".to_string(), Vec::new(), repo_path, tx);
    });

    // Collect all output
    let mut output_lines = Vec::new();
    let mut completed = false;
    let timeout = Duration::from_secs(10);
    let start = std::time::Instant::now();

    while start.elapsed() < timeout {
        match rx.recv_timeout(Duration::from_millis(100)) {
            Ok(CommandEvent::Started(_)) => {
                // Process started, continue
            }
            Ok(CommandEvent::OutputLine(line)) => {
                output_lines.push(line);
            }
            Ok(CommandEvent::Completed(_)) => {
                completed = true;
                break;
            }
            Ok(CommandEvent::Failed(msg)) => {
                panic!("Command failed: {}", msg);
            }
            Err(mpsc::RecvTimeoutError::Timeout) => continue,
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }

    assert!(completed, "Command should complete");

    // Verify we captured multiple lines
    let output_text = output_lines.join("\n");
    assert!(
        output_text.contains("Line 1") && output_text.contains("Line 2") && output_text.contains("Line 3"),
        "Should capture all three lines. Got: {:?}",
        output_lines
    );
}

#[test]
fn test_command_with_arguments_passed_to_subprocess() {
    // Setup: Create command that echoes its arguments
    let temp_dir = tempdir().unwrap();
    let graft_yaml = temp_dir.path().join("graft.yaml");
    fs::write(
        &graft_yaml,
        r#"apiVersion: graft/v1beta1
name: test-repo
description: Test repository

commands:
  echo-args:
    run: echo
    description: Echo arguments
"#,
    )
    .unwrap();

    // Execute: Spawn command with arguments
    let (tx, rx) = mpsc::channel();
    let repo_path = temp_dir.path().to_string_lossy().to_string();

    std::thread::spawn(move || {
        spawn_command(
            "echo-args".to_string(),
            vec!["arg1".to_string(), "arg2".to_string(), "arg3".to_string()],
            repo_path,
            tx,
        );
    });

    // Collect output
    let mut output_lines = Vec::new();
    let mut exit_code = None;
    let timeout = Duration::from_secs(10);
    let start = std::time::Instant::now();

    while start.elapsed() < timeout {
        match rx.recv_timeout(Duration::from_millis(100)) {
            Ok(CommandEvent::Started(_)) => continue,
            Ok(CommandEvent::OutputLine(line)) => {
                output_lines.push(line);
            }
            Ok(CommandEvent::Completed(code)) => {
                exit_code = Some(code);
                break;
            }
            Ok(CommandEvent::Failed(msg)) => {
                panic!("Command failed: {}", msg);
            }
            Err(mpsc::RecvTimeoutError::Timeout) => continue,
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }

    // Verify arguments were passed
    assert_eq!(
        exit_code,
        Some(0),
        "Command should complete successfully. Output: {:?}",
        output_lines
    );

    let combined = output_lines.join("\n");
    assert!(
        combined.contains("arg1") && combined.contains("arg2") && combined.contains("arg3"),
        "Output should contain all arguments. Got: {}",
        combined
    );
}

#[test]
#[ignore] // Only run if user explicitly removes graft from PATH
fn test_graft_not_in_path_error() {
    // This test verifies the error when graft is not installed
    // Skip if graft IS installed (test would pass incorrectly)
    //
    // To run: temporarily remove graft from PATH, then:
    // cargo test test_graft_not_in_path_error -- --ignored

    let temp_dir = tempdir().unwrap();
    let graft_yaml = temp_dir.path().join("graft.yaml");
    fs::write(
        &graft_yaml,
        "apiVersion: graft/v1beta1\nname: test\ncommands:\n  test:\n    run: echo hi\n",
    )
    .unwrap();

    let (tx, rx) = mpsc::channel();
    spawn_command(
        "test".to_string(),
        Vec::new(),
        temp_dir.path().to_string_lossy().to_string(),
        tx,
    );

    // Should get helpful "graft not found" error
    loop {
        match rx.recv_timeout(Duration::from_secs(2)) {
            Ok(CommandEvent::Started(_)) => {
                // Process started, continue
            }
            Ok(CommandEvent::Failed(msg)) => {
                assert!(
                    msg.contains("graft") && msg.contains("not found"),
                    "Error should mention graft not found: {}",
                    msg
                );
                break;
            }
            other => panic!(
                "Expected Failed event with graft not found, got: {:?}",
                other
            ),
        }
    }
}
