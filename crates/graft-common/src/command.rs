//! Command execution with timeout protection.

use std::process::{Command, Output};
use std::time::Duration;
use thiserror::Error;
use wait_timeout::ChildExt;

/// Default timeout for subprocess commands (5 seconds)
const DEFAULT_COMMAND_TIMEOUT_MS: u64 = 5000;

/// Errors from command execution.
#[derive(Error, Debug)]
pub enum CommandError {
    #[error("Failed to spawn {operation}: {details}")]
    SpawnError { operation: String, details: String },

    #[error("Command {operation} timed out after {timeout_ms}ms")]
    Timeout { operation: String, timeout_ms: u64 },

    #[error("Failed to read output for {operation}: {details}")]
    OutputError { operation: String, details: String },

    #[error("Process error for {operation}: {details}")]
    ProcessError { operation: String, details: String },
}

/// Get the command timeout from environment variable or use default.
///
/// Checks `GRAFT_COMMAND_TIMEOUT_MS` for graft operations,
/// `GROVE_GIT_TIMEOUT_MS` for grove operations (for backwards compatibility),
/// or uses the default timeout.
fn get_command_timeout(env_var: Option<&str>) -> u64 {
    // Try the provided environment variable first
    if let Some(var) = env_var {
        if let Ok(s) = std::env::var(var) {
            if let Ok(ms) = s.parse::<u64>() {
                return ms;
            }
        }
    }

    // Fall back to default
    DEFAULT_COMMAND_TIMEOUT_MS
}

/// Run a command with a timeout.
///
/// # Arguments
/// * `cmd` - The command to run (must have stdout/stderr configured)
/// * `operation` - Human-readable operation name for error messages
/// * `timeout_env_var` - Optional environment variable name to read timeout from
///
/// # Returns
/// The command output on success, or an error if the command fails or times out.
///
/// # Examples
/// ```no_run
/// use std::process::Command;
/// use graft_common::run_command_with_timeout;
///
/// let mut cmd = Command::new("git");
/// cmd.args(["status", "--porcelain"]);
/// let output = run_command_with_timeout(cmd, "git status", Some("GROVE_GIT_TIMEOUT_MS"))?;
/// # Ok::<(), graft_common::CommandError>(())
/// ```
pub fn run_command_with_timeout(
    mut cmd: Command,
    operation: &str,
    timeout_env_var: Option<&str>,
) -> Result<Output, CommandError> {
    // Spawn with piped stdout/stderr
    let mut child = cmd
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| CommandError::SpawnError {
            operation: operation.to_string(),
            details: e.to_string(),
        })?;

    let timeout_ms = get_command_timeout(timeout_env_var);
    let timeout = Duration::from_millis(timeout_ms);

    match child.wait_timeout(timeout) {
        Ok(Some(_status)) => {
            // Process completed within timeout - get output
            child
                .wait_with_output()
                .map_err(|e| CommandError::OutputError {
                    operation: operation.to_string(),
                    details: e.to_string(),
                })
        }
        Ok(None) => {
            // Timeout occurred, kill the process
            let _ = child.kill();
            let _ = child.wait();
            Err(CommandError::Timeout {
                operation: operation.to_string(),
                timeout_ms,
            })
        }
        Err(e) => Err(CommandError::ProcessError {
            operation: operation.to_string(),
            details: e.to_string(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

    #[test]
    fn successful_command_execution() {
        let mut cmd = Command::new("echo");
        cmd.arg("hello");
        let result = run_command_with_timeout(cmd, "echo test", None);
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.status.success());
        assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "hello");
    }

    #[test]
    fn command_with_nonzero_exit() {
        let mut cmd = Command::new("sh");
        cmd.args(["-c", "exit 1"]);
        let result = run_command_with_timeout(cmd, "exit 1 test", None);
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(!output.status.success());
    }

    #[test]
    fn command_timeout() {
        // This command sleeps for 10 seconds, which should exceed the default timeout
        let mut cmd = Command::new("sleep");
        cmd.arg("10");

        // Set a very short timeout via environment variable
        std::env::set_var("TEST_TIMEOUT_MS", "100");

        let result = run_command_with_timeout(cmd, "sleep test", Some("TEST_TIMEOUT_MS"));

        std::env::remove_var("TEST_TIMEOUT_MS");

        assert!(result.is_err());
        match result.unwrap_err() {
            CommandError::Timeout {
                operation,
                timeout_ms,
            } => {
                assert_eq!(operation, "sleep test");
                assert_eq!(timeout_ms, 100);
            }
            other => panic!("Expected Timeout error, got: {:?}", other),
        }
    }

    #[test]
    fn spawn_failure() {
        let cmd = Command::new("nonexistent_command_12345");
        let result = run_command_with_timeout(cmd, "nonexistent", None);
        assert!(result.is_err());
        match result.unwrap_err() {
            CommandError::SpawnError { operation, .. } => {
                assert_eq!(operation, "nonexistent");
            }
            other => panic!("Expected SpawnError, got: {:?}", other),
        }
    }

    #[test]
    fn uses_default_timeout_when_no_env_var() {
        // Quick command should complete within default timeout
        let mut cmd = Command::new("echo");
        cmd.arg("test");
        let result = run_command_with_timeout(cmd, "echo test", None);
        assert!(result.is_ok());
    }

    #[test]
    fn respects_custom_timeout_env_var() {
        std::env::set_var("CUSTOM_TIMEOUT", "200");

        let mut cmd = Command::new("sleep");
        cmd.arg("1"); // Sleep for 1 second

        let result = run_command_with_timeout(cmd, "sleep 1", Some("CUSTOM_TIMEOUT"));

        std::env::remove_var("CUSTOM_TIMEOUT");

        assert!(result.is_err());
        match result.unwrap_err() {
            CommandError::Timeout { timeout_ms, .. } => {
                assert_eq!(timeout_ms, 200);
            }
            other => panic!("Expected Timeout, got: {:?}", other),
        }
    }
}
