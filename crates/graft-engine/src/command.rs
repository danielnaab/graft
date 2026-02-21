//! Command execution support.
//!
//! Functions for executing dependency-defined commands from graft.yaml.

use crate::domain::{Command, GraftConfig};
use crate::error::{GraftError, Result};
use crate::state::get_state;
use crate::template::{resolve_stdin, TemplateContext};
use graft_common::process::{run_to_completion_with_timeout, ProcessConfig};
use std::collections::HashMap;
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
        stdin: None,
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

/// Execute a command with context/stdin support.
///
/// This function handles commands that have `stdin:` and/or `context:` fields:
/// 1. Resolves each state query listed in `command.context`
/// 2. Builds env vars: `GRAFT_STATE_<UPPER_NAME>=<json>` for each resolved state query
/// 3. If `command.stdin` is present, renders the template with built-in + state variables
/// 4. Pipes the rendered text to the subprocess's stdin
///
/// For commands without stdin/context, use `execute_command()` instead (or this works too).
#[allow(clippy::too_many_arguments)]
pub fn execute_command_with_context(
    command: &Command,
    config: &GraftConfig,
    base_dir: &Path,
    args: &[String],
    workspace_name: &str,
    repo_name: &str,
    refresh: bool,
) -> Result<CommandResult> {
    // Step 1: Resolve state queries from context
    let mut state_results: HashMap<String, serde_json::Value> = HashMap::new();

    if !command.context.is_empty() {
        let commit_hash = graft_common::get_current_commit(base_dir)
            .map_err(|e| GraftError::CommandExecution(format!("Failed to get commit hash: {e}")))?;

        for ctx_name in &command.context {
            let query = config.state.get(ctx_name).ok_or_else(|| {
                GraftError::CommandExecution(format!(
                    "Context entry '{ctx_name}' not found in state section"
                ))
            })?;

            let result = get_state(
                query,
                workspace_name,
                repo_name,
                base_dir,
                &commit_hash,
                refresh,
            )?;

            state_results.insert(ctx_name.clone(), result.data);
        }
    }

    // Step 2: Build env vars from state results
    let mut merged_env: HashMap<String, String> = command.env.clone().unwrap_or_default();

    for (name, value) in &state_results {
        let env_key = format!("GRAFT_STATE_{}", name.to_uppercase().replace('-', "_"));
        let json_str = serde_json::to_string(value)
            .map_err(|e| GraftError::CommandExecution(format!("Failed to serialize state: {e}")))?;
        merged_env.insert(env_key, json_str);
    }

    // Step 3: Resolve stdin (if present)
    let rendered_stdin = if let Some(ref stdin_source) = command.stdin {
        let git_branch = get_git_branch(base_dir).unwrap_or_else(|_| "unknown".to_string());
        let commit_hash =
            graft_common::get_current_commit(base_dir).unwrap_or_else(|_| "unknown".to_string());

        let template_ctx =
            TemplateContext::new(base_dir, &commit_hash, &git_branch, &state_results);

        let rendered = resolve_stdin(stdin_source, base_dir, &template_ctx)?;
        Some(rendered)
    } else {
        None
    };

    // Step 4: Build and execute command
    let mut full_command = vec![command.run.clone()];
    full_command.extend(args.iter().cloned());

    let working_dir = if let Some(ref cmd_dir) = command.working_dir {
        base_dir.join(cmd_dir)
    } else {
        base_dir.to_path_buf()
    };

    let shell_cmd = full_command.join(" ");

    let env = if merged_env.is_empty() {
        None
    } else {
        Some(merged_env)
    };

    let process_config = ProcessConfig {
        command: shell_cmd,
        working_dir,
        env,
        log_path: None,
        timeout: None,
        stdin: rendered_stdin,
    };

    let output = run_to_completion_with_timeout(&process_config)
        .map_err(|e| GraftError::CommandExecution(e.to_string()))?;

    Ok(CommandResult {
        exit_code: output.exit_code,
        stdout: output.stdout,
        stderr: output.stderr,
        success: output.success,
    })
}

/// Resolve a command's stdin to rendered text (for dry-run mode).
///
/// Returns the rendered stdin text, or None if the command has no stdin.
pub fn resolve_command_stdin(
    command: &Command,
    config: &GraftConfig,
    base_dir: &Path,
    workspace_name: &str,
    repo_name: &str,
    refresh: bool,
) -> Result<Option<String>> {
    if command.stdin.is_none() {
        return Ok(None);
    }

    // Resolve state queries from context
    let mut state_results: HashMap<String, serde_json::Value> = HashMap::new();

    if !command.context.is_empty() {
        let commit_hash = graft_common::get_current_commit(base_dir)
            .map_err(|e| GraftError::CommandExecution(format!("Failed to get commit hash: {e}")))?;

        for ctx_name in &command.context {
            let query = config.state.get(ctx_name).ok_or_else(|| {
                GraftError::CommandExecution(format!(
                    "Context entry '{ctx_name}' not found in state section"
                ))
            })?;

            let result = get_state(
                query,
                workspace_name,
                repo_name,
                base_dir,
                &commit_hash,
                refresh,
            )?;

            state_results.insert(ctx_name.clone(), result.data);
        }
    }

    let stdin_source = command.stdin.as_ref().unwrap();
    let git_branch = get_git_branch(base_dir).unwrap_or_else(|_| "unknown".to_string());
    let commit_hash =
        graft_common::get_current_commit(base_dir).unwrap_or_else(|_| "unknown".to_string());

    let template_ctx = TemplateContext::new(base_dir, &commit_hash, &git_branch, &state_results);

    let rendered = resolve_stdin(stdin_source, base_dir, &template_ctx)?;
    Ok(Some(rendered))
}

/// Get current git branch name.
fn get_git_branch(repo_path: &Path) -> Result<String> {
    let config = ProcessConfig {
        command: "git rev-parse --abbrev-ref HEAD".to_string(),
        working_dir: repo_path.to_path_buf(),
        env: None,
        log_path: None,
        timeout: None,
        stdin: None,
    };

    let output = run_to_completion_with_timeout(&config)
        .map_err(|e| GraftError::CommandExecution(format!("Failed to get git branch: {e}")))?;

    if output.success {
        Ok(output.stdout.trim().to_string())
    } else {
        Err(GraftError::CommandExecution(
            "Failed to get git branch".to_string(),
        ))
    }
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
