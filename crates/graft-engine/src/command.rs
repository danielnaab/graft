//! Command execution support.
//!
//! Functions for executing dependency-defined commands from graft.yaml.

use graft_core::domain::{Command, GraftConfig};
use graft_core::error::{GraftError, Result};
use std::path::Path;
use std::process::{Command as ProcessCommand, Stdio};

/// Result of executing a command.
#[derive(Debug, Clone)]
pub struct CommandResult {
    /// Exit code from the command
    pub exit_code: i32,
    /// Standard output
    pub stdout: String,
    /// Standard error
    pub stderr: String,
    /// Whether the command succeeded (exit code 0)
    pub success: bool,
}

/// Execute a dependency-defined command.
///
/// # Arguments
///
/// * `command` - Command definition from graft.yaml
/// * `base_dir` - Base directory for executing the command (typically dependency directory)
/// * `args` - Additional command-line arguments to pass
///
/// # Returns
///
/// `CommandResult` with exit code and output
pub fn execute_command(
    command: &Command,
    base_dir: &Path,
    args: &[String],
) -> Result<CommandResult> {
    // Build full command with args
    let mut full_command = vec![command.run.clone()];
    full_command.extend(args.iter().cloned());

    // Determine working directory
    let working_dir = if let Some(ref cmd_dir) = command.working_dir {
        base_dir.join(cmd_dir)
    } else {
        base_dir.to_path_buf()
    };

    // Execute command via shell to support pipes, redirects, etc.
    let shell_cmd = full_command.join(" ");

    // Set environment variables if specified (must be done before spawn)
    let mut cmd = ProcessCommand::new("sh");
    cmd.arg("-c")
        .arg(&shell_cmd)
        .current_dir(&working_dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    if let Some(env_vars) = &command.env {
        for (key, value) in env_vars {
            cmd.env(key, value);
        }
    }

    let process = cmd
        .spawn()
        .map_err(|e| GraftError::CommandExecution(format!("Failed to spawn command: {e}")))?;

    let output = process
        .wait_with_output()
        .map_err(|e| GraftError::CommandExecution(format!("Failed to wait for command: {e}")))?;

    let exit_code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let success = output.status.success();

    Ok(CommandResult {
        exit_code,
        stdout,
        stderr,
        success,
    })
}

/// Execute a command by name from the command registry.
///
/// # Arguments
///
/// * `config` - Graft configuration containing command definitions
/// * `command_name` - Name of command to execute
/// * `base_dir` - Base directory for executing the command
/// * `args` - Additional command-line arguments
///
/// # Returns
///
/// `CommandResult` with exit code and output
pub fn execute_command_by_name(
    config: &GraftConfig,
    command_name: &str,
    base_dir: &Path,
    args: &[String],
) -> Result<CommandResult> {
    let command = config.commands.get(command_name).ok_or_else(|| {
        GraftError::CommandExecution(format!("Command not found: {command_name}"))
    })?;

    execute_command(command, base_dir, args)
}

#[cfg(test)]
mod tests {
    use super::*;
    use graft_core::domain::Command;
    use std::path::PathBuf;

    #[test]
    fn execute_simple_command_success() {
        let command = Command::new("echo", "echo 'hello world'").unwrap();
        let result = execute_command(&command, &PathBuf::from("."), &[]).unwrap();

        assert!(result.success);
        assert_eq!(result.exit_code, 0);
        assert!(result.stdout.contains("hello world"));
    }

    #[test]
    fn execute_simple_command_failure() {
        let command = Command::new("fail", "exit 1").unwrap();
        let result = execute_command(&command, &PathBuf::from("."), &[]).unwrap();

        assert!(!result.success);
        assert_eq!(result.exit_code, 1);
    }

    #[test]
    fn execute_command_captures_stderr() {
        let command = Command::new("err", "echo 'error message' >&2").unwrap();
        let result = execute_command(&command, &PathBuf::from("."), &[]).unwrap();

        assert!(result.success);
        assert!(result.stderr.contains("error message"));
    }

    #[test]
    fn execute_nonexistent_command_by_name() {
        let config = GraftConfig::new("graft/v0").unwrap();
        let result = execute_command_by_name(&config, "nonexistent", &PathBuf::from("."), &[]);

        assert!(result.is_err());
        if let Err(GraftError::CommandExecution(msg)) = result {
            assert!(msg.contains("Command not found"));
        } else {
            panic!("Expected CommandExecution error");
        }
    }
}
