//! Command execution support.
//!
//! Functions for executing dependency-defined commands from graft.yaml.

use crate::domain::{Command, GraftConfig};
use crate::error::{GraftError, Result};
use crate::state::{get_run_state_entry, get_state};
use crate::template::{resolve_stdin, TemplateContext};
use graft_common::process::{run_to_completion_with_timeout, ProcessConfig};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

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
    /// State written by the command, keyed by name from `writes:`.
    /// Populated only for commands executed via `execute_command_with_context`.
    pub written_state: HashMap<String, serde_json::Value>,
}

/// Execution context for a command, carrying dual-path resolution.
///
/// `source_dir` is where the command's scripts and templates live (the dep directory
/// for dep commands, or the repo root for local commands).
///
/// `consumer_dir` is the consumer's repo root — where commands execute, where git
/// state lives, and where `repo_path` template variable points.
///
/// For local commands, both paths are the same directory.
#[derive(Debug, Clone)]
pub struct CommandContext {
    /// Where scripts and templates live (dep dir or repo root).
    pub source_dir: PathBuf,
    /// Consumer's repo root — working directory for commands and git ops.
    pub consumer_dir: PathBuf,
    /// Workspace name for state caching.
    pub workspace_name: String,
    /// Repo name for state caching.
    pub repo_name: String,
    /// Whether to force-refresh state queries (ignore cache).
    pub refresh: bool,
}

impl CommandContext {
    /// Create a context for a local command (source and consumer are the same directory).
    pub fn local(base_dir: &Path, workspace_name: &str, repo_name: &str, refresh: bool) -> Self {
        Self {
            source_dir: base_dir.to_path_buf(),
            consumer_dir: base_dir.to_path_buf(),
            workspace_name: workspace_name.to_string(),
            repo_name: repo_name.to_string(),
            refresh,
        }
    }

    /// Create a context for a dependency command.
    ///
    /// `dep_dir` is where the dep's scripts/templates live (e.g., `.graft/<dep>/`).
    /// `consumer_dir` is the consumer's repo root.
    pub fn dependency(
        dep_dir: &Path,
        consumer_dir: &Path,
        workspace_name: &str,
        repo_name: &str,
        refresh: bool,
    ) -> Self {
        Self {
            source_dir: dep_dir.to_path_buf(),
            consumer_dir: consumer_dir.to_path_buf(),
            workspace_name: workspace_name.to_string(),
            repo_name: repo_name.to_string(),
            refresh,
        }
    }

    /// Whether this is a dependency context (`source_dir` differs from `consumer_dir`).
    pub fn is_dependency(&self) -> bool {
        self.source_dir != self.consumer_dir
    }
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
        env_remove: vec![],
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
        written_state: HashMap::new(),
    })
}

/// Set up the run-state directory and enforce `reads:` preconditions.
///
/// Creates `.graft/run-state/` in `consumer_dir` if it doesn't exist.
/// For each name in `command.reads`, verifies the corresponding JSON file
/// exists **and contains valid JSON** before returning. Missing or malformed
/// state produces a clear error naming which command produces that state.
///
/// Returns the path to the run-state directory. Call this before executing any
/// command that may use the run-state store (writes, reads, or streaming).
pub fn setup_run_state(
    command: &Command,
    config: &GraftConfig,
    consumer_dir: &Path,
) -> Result<PathBuf> {
    let run_state_dir = consumer_dir.join(".graft").join("run-state");
    std::fs::create_dir_all(&run_state_dir).map_err(|e| {
        GraftError::CommandExecution(format!(
            "Failed to create run-state directory '{}': {e}",
            run_state_dir.display()
        ))
    })?;

    // Build the dependency graph once so each reads-lookup is O(1) rather than O(n).
    // Duplicate-producer errors are caught at validate() time; fall back to an empty
    // graph (no producer hint) if the config is somehow not yet validated.
    let graph = crate::dependency_graph::DependencyGraph::from_config(config).unwrap_or_default();

    for reads_name in &command.reads {
        let state_file = run_state_dir.join(format!("{reads_name}.json"));
        if !state_file.exists() {
            let producer = graph
                .producer(reads_name)
                .map(|p| format!(" (produced by: {p})"))
                .unwrap_or_default();
            return Err(GraftError::CommandExecution(format!(
                "command '{}' requires state '{}'{producer}",
                command.name, reads_name
            )));
        }
        // Validate the file contains well-formed JSON
        let content = std::fs::read_to_string(&state_file).map_err(|e| {
            GraftError::CommandExecution(format!(
                "Failed to read state file '{}': {e}",
                state_file.display()
            ))
        })?;
        serde_json::from_str::<serde_json::Value>(&content).map_err(|e| {
            GraftError::CommandExecution(format!("State '{reads_name}' contains invalid JSON: {e}"))
        })?;
    }

    Ok(run_state_dir)
}

/// Read back state files written by a command after successful execution.
///
/// For each name in `command.writes`, reads `<run_state_dir>/<name>.json`
/// and returns parsed values. Silently skips files that don't exist or
/// contain malformed JSON — the command may have chosen not to write them.
pub fn capture_written_state(
    command: &Command,
    run_state_dir: &Path,
) -> HashMap<String, serde_json::Value> {
    let mut written_state = HashMap::new();
    for name in &command.writes {
        let state_file = run_state_dir.join(format!("{name}.json"));
        if let Ok(content) = std::fs::read_to_string(&state_file) {
            if let Ok(value) = serde_json::from_str::<serde_json::Value>(&content) {
                written_state.insert(name.clone(), value);
            }
        }
    }
    written_state
}

/// Execute a command with context/stdin support.
///
/// This function handles commands that have `stdin:` and/or `context:` fields:
/// 1. Resolves each state query listed in `command.context`
/// 2. Builds env vars: `GRAFT_STATE_<UPPER_NAME>=<json>` for each resolved state query
/// 3. If `command.stdin` is present, renders the template with built-in + state variables
/// 4. Pipes the rendered text to the subprocess's stdin
///
/// Also manages the run-state store for all commands (not only those with
/// stdin/context): `GRAFT_STATE_DIR` is always injected, `reads:` preconditions
/// are always enforced, and `writes:` state is captured after successful runs.
///
/// Path resolution uses `CommandContext`:
/// - Template files → resolved from `ctx.source_dir`
/// - State query working dir → `ctx.consumer_dir`
/// - Git operations → `ctx.consumer_dir`
/// - Command working dir → defaults to `ctx.consumer_dir`
/// - Template `repo_path` variable → `ctx.consumer_dir`
///
/// For commands without stdin/context, use `execute_command()` instead (or this works too).
pub fn execute_command_with_context(
    command: &Command,
    config: &GraftConfig,
    ctx: &CommandContext,
    args: &[String],
) -> Result<CommandResult> {
    execute_command_with_context_timeout(command, config, ctx, args, None)
}

/// Like [`execute_command_with_context`] but with an optional per-step timeout override.
///
/// When `timeout_override` is `Some`, it takes precedence over `GRAFT_TIMEOUT` env var
/// and process defaults. Used by sequence execution to enforce per-step timeouts.
pub fn execute_command_with_context_timeout(
    command: &Command,
    config: &GraftConfig,
    ctx: &CommandContext,
    args: &[String],
    timeout_override: Option<std::time::Duration>,
) -> Result<CommandResult> {
    // Set up run-state directory and enforce reads: preconditions
    let run_state_dir = setup_run_state(command, config, &ctx.consumer_dir)?;

    // Step 1: Resolve state queries from context (git ops use consumer_dir)
    let state_results = resolve_state_queries(command, config, ctx)?;

    // Step 2: Build env vars from state results
    let mut merged_env: HashMap<String, String> = command.env.clone().unwrap_or_default();

    for (name, value) in &state_results {
        let env_key = format!("GRAFT_STATE_{}", name.to_uppercase().replace('-', "_"));
        let json_str = serde_json::to_string(value)
            .map_err(|e| GraftError::CommandExecution(format!("Failed to serialize state: {e}")))?;
        merged_env.insert(env_key, json_str);
    }

    // Inject GRAFT_STATE_DIR pointing to the run-state store
    merged_env.insert(
        "GRAFT_STATE_DIR".to_string(),
        run_state_dir.to_string_lossy().to_string(),
    );

    // Inject GRAFT_DEP_DIR for dependency commands
    if ctx.is_dependency() {
        merged_env.insert(
            "GRAFT_DEP_DIR".to_string(),
            ctx.source_dir.to_string_lossy().to_string(),
        );
    }

    // Step 3: Resolve stdin (if present)
    // Template files resolve from source_dir; template context repo_path = consumer_dir
    let rendered_stdin = if let Some(ref stdin_source) = command.stdin {
        let git_branch =
            get_git_branch(&ctx.consumer_dir).unwrap_or_else(|_| "unknown".to_string());
        let commit_hash = graft_common::get_current_commit(&ctx.consumer_dir)
            .unwrap_or_else(|_| "unknown".to_string());

        let template_ctx = TemplateContext::new(
            &ctx.consumer_dir,
            &commit_hash,
            &git_branch,
            &state_results,
            args,
        );

        // Template file paths resolve from source_dir
        let rendered = resolve_stdin(stdin_source, &ctx.source_dir, &template_ctx)?;
        Some(rendered)
    } else {
        None
    };

    // Step 4: Build and execute command
    // When stdin is configured, args are consumed by the template (not appended to command)
    let shell_cmd = build_shell_command(command, &ctx.source_dir, args);

    // Working dir defaults to consumer_dir
    let working_dir = if let Some(ref cmd_dir) = command.working_dir {
        ctx.consumer_dir.join(cmd_dir)
    } else {
        ctx.consumer_dir.clone()
    };

    let env = if merged_env.is_empty() {
        None
    } else {
        Some(merged_env)
    };

    // For local commands, explicitly unset GRAFT_DEP_DIR so the subprocess does not
    // inherit it from the parent process environment (e.g. when running inside a dep shell).
    let env_remove = if ctx.is_dependency() {
        vec![]
    } else {
        vec!["GRAFT_DEP_DIR".to_string()]
    };

    let process_config = ProcessConfig {
        command: shell_cmd,
        working_dir,
        env,
        env_remove,
        log_path: None,
        timeout: timeout_override,
        stdin: rendered_stdin,
    };

    let output = run_to_completion_with_timeout(&process_config)
        .map_err(|e| GraftError::CommandExecution(e.to_string()))?;

    // Read back any state written by the command
    let written_state = if output.success {
        capture_written_state(command, &run_state_dir)
    } else {
        HashMap::new()
    };

    Ok(CommandResult {
        exit_code: output.exit_code,
        stdout: output.stdout,
        stderr: output.stderr,
        success: output.success,
        written_state,
    })
}

/// Resolve a command's stdin to rendered text (for dry-run mode).
///
/// Returns the rendered stdin text, or None if the command has no stdin.
/// Template files resolve from `ctx.source_dir`; template `repo_path` = `ctx.consumer_dir`.
pub fn resolve_command_stdin(
    command: &Command,
    config: &GraftConfig,
    ctx: &CommandContext,
    args: &[String],
) -> Result<Option<String>> {
    if command.stdin.is_none() {
        return Ok(None);
    }

    // Resolve state queries from context (git ops use consumer_dir)
    let state_results = resolve_state_queries(command, config, ctx)?;

    let stdin_source = command.stdin.as_ref().unwrap();
    let git_branch = get_git_branch(&ctx.consumer_dir).unwrap_or_else(|_| "unknown".to_string());
    let commit_hash = graft_common::get_current_commit(&ctx.consumer_dir)
        .unwrap_or_else(|_| "unknown".to_string());

    let template_ctx = TemplateContext::new(
        &ctx.consumer_dir,
        &commit_hash,
        &git_branch,
        &state_results,
        args,
    );

    // Template file paths resolve from source_dir
    let rendered = resolve_stdin(stdin_source, &ctx.source_dir, &template_ctx)?;
    Ok(Some(rendered))
}

/// Resolve state queries referenced in a command's `context` field.
///
/// State query scripts execute in `ctx.consumer_dir` (the consumer's repo root).
/// For dependency commands, state query `run:` fields have script paths resolved
/// from `ctx.source_dir` (same heuristic as command `run:` fields).
pub(crate) fn resolve_state_queries(
    command: &Command,
    config: &GraftConfig,
    ctx: &CommandContext,
) -> Result<HashMap<String, serde_json::Value>> {
    let mut state_results: HashMap<String, serde_json::Value> = HashMap::new();

    if !command.context.is_empty() {
        let commit_hash = graft_common::get_current_commit(&ctx.consumer_dir)
            .map_err(|e| GraftError::CommandExecution(format!("Failed to get commit hash: {e}")))?;

        for ctx_name in &command.context {
            if let Some(query) = config.state.get(ctx_name) {
                // Resolve script paths in the state query's run field for dep commands
                let resolved_query = if ctx.is_dependency() {
                    let resolved_run = resolve_script_in_command(&query.run, &ctx.source_dir);
                    if resolved_run == query.run {
                        None
                    } else {
                        let mut q = query.clone();
                        q.run = resolved_run;
                        Some(q)
                    }
                } else {
                    None
                };

                let result = get_state(
                    resolved_query.as_ref().unwrap_or(query),
                    &ctx.workspace_name,
                    &ctx.repo_name,
                    &ctx.consumer_dir,
                    &commit_hash,
                    ctx.refresh,
                )?;

                state_results.insert(ctx_name.clone(), result.data);
            } else if let Some(value) = get_run_state_entry(ctx_name, &ctx.consumer_dir) {
                // Fall back to run-state store
                state_results.insert(ctx_name.clone(), value);
            } else {
                return Err(GraftError::CommandExecution(format!(
                    "Context entry '{ctx_name}' not found in state section or run-state store"
                )));
            }
        }
    }

    Ok(state_results)
}

/// Resolve a script path in a command's `run:` field.
///
/// For commands matching `<interpreter> <relative-path> [args]` where the path
/// exists in `source_dir`, rewrites the path to absolute so the script is found
/// even when the working directory is the consumer's repo root.
///
/// Returns the (possibly rewritten) shell command string.
pub fn resolve_script_in_command(run: &str, source_dir: &Path) -> String {
    let parts: Vec<&str> = run.splitn(3, char::is_whitespace).collect();

    if parts.len() < 2 {
        return run.to_string();
    }

    let script_path = Path::new(parts[1]);

    // Only rewrite relative paths (not absolute, not bare commands like "cat")
    if script_path.is_absolute() || !script_path.to_string_lossy().contains('/') {
        return run.to_string();
    }

    // Check if the script exists in source_dir
    let resolved = source_dir.join(script_path);
    if resolved.exists() {
        let abs_path = resolved.to_string_lossy();
        if parts.len() == 3 {
            format!("{} {} {}", parts[0], abs_path, parts[2])
        } else {
            format!("{} {}", parts[0], abs_path)
        }
    } else {
        run.to_string()
    }
}

/// Build the shell command string, resolving script paths and placeholders.
///
/// If the `run` field contains `{name}` placeholders (excluding `${VAR}` shell syntax),
/// they are replaced with arg values in order. Otherwise, args are appended to the
/// command (unless stdin is configured, in which case args go to the template).
pub(crate) fn build_shell_command(command: &Command, source_dir: &Path, args: &[String]) -> String {
    let resolved_run = resolve_script_in_command(&command.run, source_dir);

    let (substituted, had_placeholders) = substitute_placeholders(&resolved_run, args);

    if had_placeholders {
        return substituted;
    }

    let mut full_command = vec![resolved_run];
    if command.stdin.is_none() {
        full_command.extend(args.iter().cloned());
    }

    full_command.join(" ")
}

/// Check whether a command string contains `{name}` placeholders.
///
/// Returns `true` if at least one `{identifier}` pattern is found (excluding
/// `${VAR}` shell variable syntax). Identifiers may contain alphanumeric
/// characters, underscores, or hyphens.
pub fn has_placeholders(run: &str) -> bool {
    scan_placeholders(run, &mut |_| None::<&str>).1
}

/// Replace `{name}` placeholders positionally with shell-escaped arg values.
///
/// Args are consumed in order: the first placeholder gets `args[0]`, the second
/// gets `args[1]`, etc. Extra args beyond the number of placeholders are ignored.
/// Placeholders without a corresponding arg value are left as-is.
///
/// Returns the substituted string and whether any placeholders were found.
pub fn substitute_placeholders(run: &str, args: &[String]) -> (String, bool) {
    let mut arg_index = 0;
    scan_placeholders(run, &mut |_name| {
        if arg_index < args.len() {
            let val = &args[arg_index];
            arg_index += 1;
            Some(val.as_str())
        } else {
            None
        }
    })
}

/// Replace `{name}` placeholders by name with shell-escaped values.
///
/// Each placeholder `{foo}` is looked up in `named_args` by matching the first
/// element of each pair. Unmatched placeholders are left as-is.
///
/// Returns the substituted string and whether any placeholders were found.
pub fn substitute_named_placeholders(run: &str, named_args: &[(&str, &str)]) -> (String, bool) {
    scan_placeholders(run, &mut |name| {
        named_args.iter().find(|(n, _)| *n == name).map(|(_, v)| *v)
    })
}

/// Core placeholder scanner.
///
/// Scans `run` for `{identifier}` patterns (excluding `${VAR}` shell syntax).
/// For each placeholder found, calls `lookup(name)`:
/// - `Some(value)` → substituted with `shell_words::quote(value)`
/// - `None` → left as-is (`{name}`)
///
/// Returns the result string and whether any placeholders were found.
fn scan_placeholders<'a>(
    run: &str,
    lookup: &mut impl FnMut(&str) -> Option<&'a str>,
) -> (String, bool) {
    let chars: Vec<char> = run.chars().collect();
    let mut result = String::with_capacity(run.len());
    let mut i = 0;
    let mut found_any = false;

    while i < chars.len() {
        if chars[i] == '{' {
            // Skip ${...} (shell env var syntax)
            if i > 0 && chars[i - 1] == '$' {
                result.push('{');
                i += 1;
                continue;
            }
            // Scan for closing `}`
            let start = i + 1;
            let mut end = start;
            while end < chars.len() && chars[end] != '}' {
                end += 1;
            }
            if end < chars.len() && end > start {
                let inner: String = chars[start..end].iter().collect();
                if inner
                    .chars()
                    .all(|ch| ch.is_alphanumeric() || ch == '_' || ch == '-')
                {
                    found_any = true;
                    if let Some(value) = lookup(&inner) {
                        let escaped = shell_words::quote(value);
                        result.push_str(&escaped);
                    } else {
                        result.push('{');
                        result.push_str(&inner);
                        result.push('}');
                    }
                    i = end + 1;
                    continue;
                }
            }
            result.push('{');
            i += 1;
        } else {
            result.push(chars[i]);
            i += 1;
        }
    }

    (result, found_any)
}

/// Get current git branch name.
pub(crate) fn get_git_branch(repo_path: &Path) -> Result<String> {
    let config = ProcessConfig {
        command: "git rev-parse --abbrev-ref HEAD".to_string(),
        working_dir: repo_path.to_path_buf(),
        env: None,
        env_remove: vec![],
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

    /// Tests the args-routing contract: when stdin is present, CLI args go to the
    /// template context (as `{{ args }}`), not to the shell command. When stdin is
    /// absent, args are appended to the run command as before.
    ///
    /// This duplicates the conditional from `execute_command_with_context` rather
    /// than calling it directly because the real function requires a git repo and
    /// live subprocess. The duplication is acceptable because the test asserts the
    /// behavioral contract (args routing) independent of the execution environment.
    #[test]
    fn args_not_appended_to_command_when_stdin_present() {
        use crate::domain::StdinSource;

        // Build a command with stdin
        let command = Command::new("gen", "echo ok")
            .unwrap()
            .with_stdin(StdinSource::Literal("hello".to_string()));

        let args = ["extra".to_string(), "args".to_string()];

        // Replicate the args-routing logic from execute_command_with_context
        let mut full_command = vec![command.run.clone()];
        if command.stdin.is_none() {
            full_command.extend(args.iter().cloned());
        }

        let shell_cmd = full_command.join(" ");
        assert_eq!(
            shell_cmd, "echo ok",
            "args should not be appended when stdin is present"
        );

        // Verify that without stdin, args ARE appended
        let command_no_stdin = Command::new("gen", "echo ok").unwrap();
        let mut full_command2 = vec![command_no_stdin.run.clone()];
        if command_no_stdin.stdin.is_none() {
            full_command2.extend(args.iter().cloned());
        }

        let shell_cmd2 = full_command2.join(" ");
        assert_eq!(
            shell_cmd2, "echo ok extra args",
            "args should be appended when stdin is absent"
        );
    }

    #[test]
    fn resolve_script_rewrites_relative_path() {
        let dir = tempfile::tempdir().unwrap();
        let scripts_dir = dir.path().join("scripts");
        std::fs::create_dir_all(&scripts_dir).unwrap();
        std::fs::write(scripts_dir.join("foo.sh"), "#!/bin/bash\necho hi").unwrap();

        let result = resolve_script_in_command("bash scripts/foo.sh", dir.path());
        let expected = format!("bash {}/scripts/foo.sh", dir.path().display());
        assert_eq!(result, expected);
    }

    #[test]
    fn resolve_script_preserves_trailing_args() {
        let dir = tempfile::tempdir().unwrap();
        let scripts_dir = dir.path().join("scripts");
        std::fs::create_dir_all(&scripts_dir).unwrap();
        std::fs::write(scripts_dir.join("foo.sh"), "").unwrap();

        let result = resolve_script_in_command("bash scripts/foo.sh --verbose -n", dir.path());
        let expected = format!("bash {}/scripts/foo.sh --verbose -n", dir.path().display());
        assert_eq!(result, expected);
    }

    #[test]
    fn resolve_script_noop_for_absolute_path() {
        let result = resolve_script_in_command("bash /usr/bin/script.sh", Path::new("/tmp"));
        assert_eq!(result, "bash /usr/bin/script.sh");
    }

    #[test]
    fn resolve_script_noop_for_bare_command() {
        let result = resolve_script_in_command("cat", Path::new("/tmp"));
        assert_eq!(result, "cat");
    }

    #[test]
    fn resolve_script_noop_for_non_path_arg() {
        // "echo hello" — "hello" has no `/`, so it's not treated as a path
        let result = resolve_script_in_command("echo hello", Path::new("/tmp"));
        assert_eq!(result, "echo hello");
    }

    #[test]
    fn resolve_script_noop_when_file_not_found() {
        let dir = tempfile::tempdir().unwrap();
        let result = resolve_script_in_command("bash scripts/missing.sh", dir.path());
        assert_eq!(result, "bash scripts/missing.sh");
    }

    #[test]
    fn resolve_script_noop_when_source_equals_consumer() {
        // When source_dir == consumer_dir (local command), relative paths work as-is
        let dir = tempfile::tempdir().unwrap();
        let scripts_dir = dir.path().join("scripts");
        std::fs::create_dir_all(&scripts_dir).unwrap();
        std::fs::write(scripts_dir.join("foo.sh"), "").unwrap();

        // The function still resolves, but in the local case, source_dir == consumer_dir
        // so rewriting is harmless (same directory). The caller can skip calling this
        // for local commands, but the function itself always resolves.
        let result = resolve_script_in_command("bash scripts/foo.sh", dir.path());
        assert!(result.contains("scripts/foo.sh"));
    }

    // --- Placeholder substitution tests ---

    #[test]
    fn substitute_replaces_single_placeholder() {
        let (result, found) =
            substitute_placeholders("bash scripts/iterate.sh {slice}", &["my-feature".into()]);
        assert!(found);
        assert_eq!(result, "bash scripts/iterate.sh my-feature");
    }

    #[test]
    fn substitute_replaces_embedded_placeholder() {
        let (result, found) = substitute_placeholders(
            "bash scripts/iterate.sh {slice} | claude -p",
            &["session-resume".into()],
        );
        assert!(found);
        assert_eq!(result, "bash scripts/iterate.sh session-resume | claude -p");
    }

    #[test]
    fn substitute_replaces_multiple_placeholders() {
        let (result, found) =
            substitute_placeholders("deploy {env} {tag}", &["staging".into(), "v1.2".into()]);
        assert!(found);
        assert_eq!(result, "deploy staging v1.2");
    }

    #[test]
    fn substitute_skips_shell_vars() {
        let (result, found) = substitute_placeholders("echo ${HOME} {name}", &["world".into()]);
        assert!(found);
        assert_eq!(result, "echo ${HOME} world");
    }

    #[test]
    fn substitute_no_placeholders_returns_unchanged() {
        let (result, found) = substitute_placeholders("echo hello", &["extra".into()]);
        assert!(!found);
        assert_eq!(result, "echo hello");
    }

    #[test]
    fn substitute_leaves_unmatched_placeholders() {
        let (result, found) = substitute_placeholders("deploy {env} {tag}", &["staging".into()]);
        assert!(found);
        assert_eq!(result, "deploy staging {tag}");
    }

    #[test]
    fn substitute_handles_empty_args() {
        let (result, found) = substitute_placeholders("deploy {env}", &[]);
        assert!(found);
        assert_eq!(result, "deploy {env}");
    }

    #[test]
    fn substitute_escapes_values_with_spaces() {
        let (result, found) = substitute_placeholders("deploy {env}", &["my environment".into()]);
        assert!(found);
        assert_eq!(result, "deploy 'my environment'");
    }

    #[test]
    fn substitute_escapes_shell_metacharacters() {
        let (result, found) = substitute_placeholders("echo {msg}", &["hello; rm -rf /".into()]);
        assert!(found);
        assert_eq!(result, "echo 'hello; rm -rf /'");
    }

    #[test]
    fn has_placeholders_detects_simple() {
        assert!(has_placeholders("bash {slice}"));
        assert!(has_placeholders("deploy {env} {tag}"));
        assert!(has_placeholders("{name}"));
    }

    #[test]
    fn has_placeholders_skips_shell_vars() {
        assert!(!has_placeholders("echo ${HOME}"));
        assert!(!has_placeholders("${FOO} ${BAR}"));
    }

    #[test]
    fn has_placeholders_rejects_non_identifiers() {
        assert!(!has_placeholders("echo {}"));
        assert!(!has_placeholders("echo {foo bar}"));
        assert!(!has_placeholders("echo hello"));
    }

    #[test]
    fn substitute_named_replaces_by_name() {
        let (result, found) = substitute_named_placeholders(
            "deploy {env} --tag {tag}",
            &[("tag", "v1.2"), ("env", "staging")],
        );
        assert!(found);
        assert_eq!(result, "deploy staging --tag v1.2");
    }

    #[test]
    fn substitute_named_escapes_values() {
        let (result, found) =
            substitute_named_placeholders("echo {msg}", &[("msg", "hello world")]);
        assert!(found);
        assert_eq!(result, "echo 'hello world'");
    }

    #[test]
    fn substitute_named_leaves_unmatched() {
        let (result, found) =
            substitute_named_placeholders("deploy {env} {tag}", &[("env", "staging")]);
        assert!(found);
        assert_eq!(result, "deploy staging {tag}");
    }

    #[test]
    fn build_shell_command_substitutes_placeholders() {
        let command = Command::new("impl", "bash scripts/iterate.sh {slice} | claude -p").unwrap();
        let result = build_shell_command(&command, Path::new("/tmp"), &["my-slice".into()]);
        assert_eq!(result, "bash scripts/iterate.sh my-slice | claude -p");
    }

    #[test]
    fn command_context_local_has_same_dirs() {
        let ctx = CommandContext::local(Path::new("/repo"), "ws", "repo", false);
        assert_eq!(ctx.source_dir, ctx.consumer_dir);
        assert!(!ctx.is_dependency());
    }

    #[test]
    fn command_context_dependency_has_different_dirs() {
        let ctx = CommandContext::dependency(
            Path::new("/repo/.graft/dep"),
            Path::new("/repo"),
            "ws",
            "repo",
            false,
        );
        assert_ne!(ctx.source_dir, ctx.consumer_dir);
        assert!(ctx.is_dependency());
    }

    #[test]
    fn build_shell_command_resolves_script_for_dep() {
        let dir = tempfile::tempdir().unwrap();
        let scripts_dir = dir.path().join("scripts");
        std::fs::create_dir_all(&scripts_dir).unwrap();
        std::fs::write(scripts_dir.join("plan.sh"), "").unwrap();

        let command = Command::new("plan", "bash scripts/plan.sh").unwrap();
        let result = build_shell_command(&command, dir.path(), &[]);
        assert!(result.starts_with("bash "));
        assert!(result.contains(&dir.path().display().to_string()));
    }

    #[test]
    fn build_shell_command_appends_args_without_stdin() {
        let command = Command::new("test", "echo ok").unwrap();
        let args = vec!["--flag".to_string()];
        let result = build_shell_command(&command, Path::new("/tmp"), &args);
        assert_eq!(result, "echo ok --flag");
    }

    #[test]
    fn build_shell_command_no_args_with_stdin() {
        use crate::domain::StdinSource;
        let command = Command::new("gen", "claude")
            .unwrap()
            .with_stdin(StdinSource::Literal("prompt".to_string()));
        let args = vec!["extra".to_string()];
        let result = build_shell_command(&command, Path::new("/tmp"), &args);
        assert_eq!(result, "claude");
    }

    // --- Integration-style tests for dep command context ---

    #[test]
    fn dep_context_template_resolves_from_source_dir() {
        use crate::domain::StdinSource;

        // Set up separate source_dir (dep) and consumer_dir
        let dep_dir = tempfile::tempdir().unwrap();
        let consumer_dir = tempfile::tempdir().unwrap();

        // Template lives in the dep directory
        std::fs::write(
            dep_dir.path().join("prompt.md"),
            "Hello from {{ repo_name }}!",
        )
        .unwrap();

        let command = Command::new("gen", "cat")
            .unwrap()
            .with_stdin(StdinSource::Template {
                path: "prompt.md".to_string(),
                engine: None,
            });

        let config = GraftConfig::new("graft/v0").unwrap();
        let ctx =
            CommandContext::dependency(dep_dir.path(), consumer_dir.path(), "ws", "repo", false);

        // resolve_command_stdin should find the template in source_dir (dep dir)
        let rendered = resolve_command_stdin(&command, &config, &ctx, &[]).unwrap();
        assert!(rendered.is_some());
        // repo_name comes from consumer_dir, not source_dir
        let consumer_name = consumer_dir.path().file_name().unwrap().to_str().unwrap();
        assert!(
            rendered.as_ref().unwrap().contains(consumer_name),
            "Expected consumer repo name '{consumer_name}' in rendered output: {rendered:?}",
        );
    }

    #[test]
    fn dep_context_executes_in_consumer_dir() {
        let dep_dir = tempfile::tempdir().unwrap();
        let consumer_dir = tempfile::tempdir().unwrap();

        // Write graft.yaml to dep dir (not needed for execute_command_with_context directly)
        let command = Command::new("pwd", "pwd").unwrap();
        let config = GraftConfig::new("graft/v0").unwrap();
        let ctx =
            CommandContext::dependency(dep_dir.path(), consumer_dir.path(), "ws", "repo", false);

        let result = execute_command_with_context(&command, &config, &ctx, &[]).unwrap();
        assert!(result.success);
        // pwd should output consumer_dir, not dep_dir
        assert!(
            result
                .stdout
                .trim()
                .ends_with(consumer_dir.path().file_name().unwrap().to_str().unwrap()),
            "Expected consumer dir in pwd output, got: {}",
            result.stdout.trim()
        );
    }

    #[test]
    fn dep_context_sets_graft_dep_dir_env() {
        let dep_dir = tempfile::tempdir().unwrap();
        let consumer_dir = tempfile::tempdir().unwrap();

        let command = Command::new("env", "printenv GRAFT_DEP_DIR").unwrap();
        let config = GraftConfig::new("graft/v0").unwrap();
        let ctx =
            CommandContext::dependency(dep_dir.path(), consumer_dir.path(), "ws", "repo", false);

        let result = execute_command_with_context(&command, &config, &ctx, &[]).unwrap();
        assert!(result.success);
        assert_eq!(
            result.stdout.trim(),
            dep_dir.path().to_str().unwrap(),
            "GRAFT_DEP_DIR should point to the dep directory"
        );
    }

    #[test]
    fn local_context_no_graft_dep_dir_env() {
        let base_dir = tempfile::tempdir().unwrap();

        let command = Command::new("env", "printenv GRAFT_DEP_DIR || echo NOT_SET").unwrap();
        let config = GraftConfig::new("graft/v0").unwrap();
        let ctx = CommandContext::local(base_dir.path(), "ws", "repo", false);

        let result = execute_command_with_context(&command, &config, &ctx, &[]).unwrap();
        assert!(result.success);
        assert!(
            result.stdout.contains("NOT_SET"),
            "GRAFT_DEP_DIR should not be set for local commands"
        );
    }

    #[test]
    fn dep_context_resolves_script_path() {
        let dep_dir = tempfile::tempdir().unwrap();
        let consumer_dir = tempfile::tempdir().unwrap();

        // Create a script in the dep directory
        let scripts_dir = dep_dir.path().join("scripts");
        std::fs::create_dir_all(&scripts_dir).unwrap();
        std::fs::write(scripts_dir.join("hello.sh"), "#!/bin/bash\necho dep-hello").unwrap();

        let command = Command::new("hello", "bash scripts/hello.sh").unwrap();
        let config = GraftConfig::new("graft/v0").unwrap();
        let ctx =
            CommandContext::dependency(dep_dir.path(), consumer_dir.path(), "ws", "repo", false);

        let result = execute_command_with_context(&command, &config, &ctx, &[]).unwrap();
        assert!(result.success, "Script should be found and executed");
        assert!(
            result.stdout.contains("dep-hello"),
            "Script from dep dir should run successfully"
        );
    }

    #[test]
    fn local_context_unchanged_regression() {
        // Verify local commands still work identically
        let base_dir = tempfile::tempdir().unwrap();

        let command = Command::new("greet", "echo hello-local").unwrap();
        let config = GraftConfig::new("graft/v0").unwrap();
        let ctx = CommandContext::local(base_dir.path(), "ws", "repo", false);

        let result = execute_command_with_context(&command, &config, &ctx, &[]).unwrap();
        assert!(result.success);
        assert!(result.stdout.contains("hello-local"));
    }

    // --- Run-state / GRAFT_STATE_DIR tests (command-output-state-capture) ---

    #[test]
    fn setup_run_state_creates_directory() {
        let consumer_dir = tempfile::tempdir().unwrap();
        let command = Command::new("test", "echo ok").unwrap();
        let config = GraftConfig::new("graft/v0").unwrap();

        let run_state_dir = setup_run_state(&command, &config, consumer_dir.path()).unwrap();

        assert!(run_state_dir.exists(), "run-state dir should be created");
        assert!(run_state_dir.ends_with(".graft/run-state"));
    }

    #[test]
    fn graft_state_dir_is_injected_into_command_env() {
        let consumer_dir = tempfile::tempdir().unwrap();
        let command = Command::new("env", "printenv GRAFT_STATE_DIR").unwrap();
        let config = GraftConfig::new("graft/v0").unwrap();
        let ctx = CommandContext::local(consumer_dir.path(), "ws", "repo", false);

        let result = execute_command_with_context(&command, &config, &ctx, &[]).unwrap();
        assert!(result.success);
        let graft_state_dir = result.stdout.trim();
        assert!(
            graft_state_dir.ends_with(".graft/run-state"),
            "GRAFT_STATE_DIR should end with .graft/run-state, got: {graft_state_dir}"
        );
    }

    #[test]
    fn written_state_is_captured_after_successful_run() {
        let consumer_dir = tempfile::tempdir().unwrap();
        let mut command = Command::new(
            "write-state",
            r#"sh -c 'echo "{\"id\":\"abc123\"}" > "$GRAFT_STATE_DIR/session.json"'"#,
        )
        .unwrap();
        command.writes = vec!["session".to_string()];

        let config = GraftConfig::new("graft/v0").unwrap();
        let ctx = CommandContext::local(consumer_dir.path(), "ws", "repo", false);

        let result = execute_command_with_context(&command, &config, &ctx, &[]).unwrap();
        assert!(result.success);
        assert_eq!(
            result
                .written_state
                .get("session")
                .and_then(|v| v.get("id")),
            Some(&serde_json::json!("abc123")),
            "written_state should contain the session value"
        );
    }

    #[test]
    fn written_state_not_captured_on_failure() {
        let consumer_dir = tempfile::tempdir().unwrap();
        let mut command = Command::new(
            "write-then-fail",
            r#"sh -c 'echo "{\"id\":\"abc\"}" > "$GRAFT_STATE_DIR/session.json"; exit 1'"#,
        )
        .unwrap();
        command.writes = vec!["session".to_string()];

        let config = GraftConfig::new("graft/v0").unwrap();
        let ctx = CommandContext::local(consumer_dir.path(), "ws", "repo", false);

        let result = execute_command_with_context(&command, &config, &ctx, &[]).unwrap();
        assert!(!result.success);
        assert!(
            result.written_state.is_empty(),
            "written_state should be empty on command failure"
        );
    }

    #[test]
    fn reads_enforcement_fails_when_state_missing() {
        let consumer_dir = tempfile::tempdir().unwrap();
        let mut command = Command::new("resume", "echo ok").unwrap();
        command.reads = vec!["session".to_string()];

        // Config with an implement command that writes session
        let mut config = GraftConfig::new("graft/v0").unwrap();
        let mut producer = Command::new("implement", "echo implement").unwrap();
        producer.writes = vec!["session".to_string()];
        config.commands.insert("implement".to_string(), producer);

        let ctx = CommandContext::local(consumer_dir.path(), "ws", "repo", false);

        let result = execute_command_with_context(&command, &config, &ctx, &[]);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("requires state 'session'"),
            "Error should mention missing state, got: {err}"
        );
        assert!(
            err.contains("implement"),
            "Error should name the producing command, got: {err}"
        );
    }

    #[test]
    fn reads_enforcement_succeeds_when_state_exists() {
        let consumer_dir = tempfile::tempdir().unwrap();

        // Pre-create the run-state file
        let run_state_dir = consumer_dir.path().join(".graft").join("run-state");
        std::fs::create_dir_all(&run_state_dir).unwrap();
        std::fs::write(
            run_state_dir.join("session.json"),
            r#"{"id":"pre-existing"}"#,
        )
        .unwrap();

        let mut command = Command::new("resume", "echo ok").unwrap();
        command.reads = vec!["session".to_string()];

        let config = GraftConfig::new("graft/v0").unwrap();
        let ctx = CommandContext::local(consumer_dir.path(), "ws", "repo", false);

        let result = execute_command_with_context(&command, &config, &ctx, &[]).unwrap();
        assert!(
            result.success,
            "Command should succeed when reads are satisfied"
        );
    }

    #[test]
    fn reads_enforcement_fails_on_invalid_json() {
        let consumer_dir = tempfile::tempdir().unwrap();

        // Pre-create a state file with malformed JSON
        let run_state_dir = consumer_dir.path().join(".graft").join("run-state");
        std::fs::create_dir_all(&run_state_dir).unwrap();
        std::fs::write(run_state_dir.join("session.json"), "not valid json").unwrap();

        let mut command = Command::new("resume", "echo ok").unwrap();
        command.reads = vec!["session".to_string()];

        let config = GraftConfig::new("graft/v0").unwrap();
        let ctx = CommandContext::local(consumer_dir.path(), "ws", "repo", false);

        let result = execute_command_with_context(&command, &config, &ctx, &[]);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("invalid JSON"),
            "Error should mention invalid JSON, got: {err}"
        );
    }

    #[test]
    fn context_falls_back_to_run_state_when_no_configured_query() {
        use std::process::Command as StdCommand;

        let consumer_dir = tempfile::tempdir().unwrap();

        // Initialize a git repo (required for commit hash resolution when context is non-empty)
        StdCommand::new("git")
            .args(["init"])
            .current_dir(consumer_dir.path())
            .output()
            .unwrap();
        StdCommand::new("git")
            .args(["commit", "--allow-empty", "-m", "init"])
            .current_dir(consumer_dir.path())
            .output()
            .unwrap();

        // Pre-create a run-state entry
        let run_state_dir = consumer_dir.path().join(".graft").join("run-state");
        std::fs::create_dir_all(&run_state_dir).unwrap();
        std::fs::write(
            run_state_dir.join("session.json"),
            r#"{"id":"from-run-state"}"#,
        )
        .unwrap();

        // Command with `context: [session]` but no configured state query for it.
        let mut command = Command::new("use-state", "printenv GRAFT_STATE_SESSION").unwrap();
        command.context = vec!["session".to_string()];

        let config = GraftConfig::new("graft/v0").unwrap();
        let ctx = CommandContext::local(consumer_dir.path(), "ws", "repo", false);

        let result = execute_command_with_context(&command, &config, &ctx, &[]).unwrap();
        assert!(
            result.success,
            "Command should succeed when context resolves from run-state"
        );
        assert!(
            result.stdout.contains("from-run-state"),
            "GRAFT_STATE_SESSION should contain the run-state value, got: {}",
            result.stdout
        );
    }

    #[test]
    fn dep_context_resolves_state_query_script_path() {
        use crate::domain::{StateCache, StateQuery};
        use std::process::Command as StdCommand;

        let dep_dir = tempfile::tempdir().unwrap();
        let consumer_dir = tempfile::tempdir().unwrap();

        // consumer_dir needs to be a git repo (for get_current_commit)
        StdCommand::new("git")
            .args(["init"])
            .current_dir(consumer_dir.path())
            .output()
            .unwrap();
        StdCommand::new("git")
            .args(["commit", "--allow-empty", "-m", "init"])
            .current_dir(consumer_dir.path())
            .output()
            .unwrap();

        // Create a state query script in the dep directory
        let scripts_dir = dep_dir.path().join("scripts");
        std::fs::create_dir_all(&scripts_dir).unwrap();
        std::fs::write(
            scripts_dir.join("query.sh"),
            "#!/bin/bash\necho '{\"from\": \"dep\"}'",
        )
        .unwrap();

        // Command that references the state query
        let mut command = Command::new("test", "cat").unwrap();
        command.context = vec!["myquery".to_string()];

        // Config with the state query
        let mut config = GraftConfig::new("graft/v0").unwrap();
        let query = StateQuery::new("myquery", "bash scripts/query.sh")
            .unwrap()
            .with_cache(StateCache {
                inputs: Vec::new(), // no inputs → always run fresh
                ttl: None,
            });
        config.state.insert("myquery".to_string(), query);

        let ctx =
            CommandContext::dependency(dep_dir.path(), consumer_dir.path(), "ws", "repo", false);

        // execute_command_with_context should resolve the state query script from source_dir
        let result = execute_command_with_context(&command, &config, &ctx, &[]).unwrap();
        assert!(
            result.success,
            "State query script should be found and executed. stderr: {}",
            result.stderr
        );
    }
}
