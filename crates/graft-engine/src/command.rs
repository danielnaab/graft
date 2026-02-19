//! Command execution support.
//!
//! Functions for executing dependency-defined commands from graft.yaml.

use crate::domain::{Command, GraftConfig};
use crate::error::{GraftError, Result};
use graft_common::process::{run_to_completion_with_timeout, ProcessConfig};
use std::path::Path;

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

    let config = ProcessConfig {
        command: shell_cmd,
        working_dir,
        env: command.env.clone(),
        log_path: None,
        timeout: None,
    };

    let output = run_to_completion_with_timeout(&config)
        .map_err(|e| GraftError::CommandExecution(e.to_string()))?;

    Ok(CommandResult {
        exit_code: output.exit_code,
        stdout: output.stdout,
        stderr: output.stderr,
        success: output.success,
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
    use crate::domain::Command;
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
