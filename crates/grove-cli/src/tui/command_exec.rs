//! Command execution and event handling.

use super::{
    mpsc, supports_unicode, App, CommandState, RepoDetailProvider, RepoRegistry, Sender,
    LINES_TO_DROP, MAX_OUTPUT_LINES,
};
use graft_common::process::{ProcessConfig, ProcessEvent, ProcessHandle};
use graft_common::runs::{run_log_path, write_run_meta, RunMeta};
use std::path::PathBuf;

/// Context for run logging — carries workspace/repo/command identity.
#[derive(Debug, Clone)]
pub struct RunContext {
    pub workspace: String,
    pub repo: String,
    pub command: String,
}

/// Events from async command execution.
#[derive(Debug)]
pub enum CommandEvent {
    Started(u32), // Process PID
    OutputLine(String),
    Completed(i32),
    Failed(String),
}

/// Prepared run-logging state returned by [`prepare_run_logging`].
struct RunLogging {
    log_path: PathBuf,
    log_file: String,
    start_time: String,
    ctx: RunContext,
}

/// Compute the log path, create the parent directory, and capture the start
/// timestamp. Returns `None` when `run_ctx` is `None` (logging disabled).
fn prepare_run_logging(run_ctx: Option<&RunContext>) -> Option<RunLogging> {
    let ctx = run_ctx?;
    let (path, file, ts) = run_log_path(&ctx.workspace, &ctx.repo, &ctx.command);
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    Some(RunLogging {
        log_path: path,
        log_file: file,
        start_time: ts.to_rfc3339(),
        ctx: ctx.clone(),
    })
}

/// Write a `RunMeta` sidecar after a command completes or fails.
fn write_run_completion_meta(
    logging: &RunLogging,
    args: &[String],
    shell_cmd: &str,
    exit_code: Option<i32>,
) {
    let meta = RunMeta {
        command: logging.ctx.command.clone(),
        args: args.to_vec(),
        shell_cmd: shell_cmd.to_string(),
        start_time: logging.start_time.clone(),
        end_time: Some(chrono::Utc::now().to_rfc3339()),
        exit_code,
        log_file: logging.log_file.clone(),
    };
    if let Err(e) = write_run_meta(&logging.ctx.workspace, &logging.ctx.repo, &meta) {
        log::warn!("Failed to write run metadata: {e}");
    }
}

/// Spawn a command defined in graft.yaml in the background and send output via channel.
///
/// Loads `graft.yaml` from `{repo_path}/graft.yaml`, looks up `command_name` in the
/// `commands:` section, and executes the command's `run` shell expression (with any
/// extra `args` appended) via [`ProcessHandle`]. Bridges [`ProcessEvent`] to
/// [`CommandEvent`] for the TUI event loop.
///
/// On success: emits `Started(pid)`, zero or more `OutputLine(line)` events (stdout and
/// stderr interleaved), then `Completed(exit_code)`.
/// On failure (command not found, spawn error): emits `Failed(message)`.
#[allow(clippy::needless_pass_by_value)]
pub fn spawn_command(
    command_name: String,
    args: Vec<String>,
    repo_path: String,
    run_ctx: Option<RunContext>,
    tx: Sender<CommandEvent>,
) {
    // Parse qualified command name (dep:cmd vs local)
    // For dep commands: scripts/templates live in source_dir (.graft/<dep>/),
    // but commands execute in consumer_dir (the repo root).
    let (graft_yaml, lookup_name, source_dir, consumer_dir) =
        if let Some((dep, cmd)) = command_name.split_once(':') {
            let path = PathBuf::from(&repo_path)
                .join(".graft")
                .join(dep)
                .join("graft.yaml");
            let src = PathBuf::from(&repo_path).join(".graft").join(dep);
            let consumer = PathBuf::from(&repo_path);
            (path, cmd.to_string(), src, consumer)
        } else {
            let path = PathBuf::from(&repo_path).join("graft.yaml");
            let dir = PathBuf::from(&repo_path);
            (path, command_name.clone(), dir.clone(), dir)
        };

    // Load commands from graft.yaml.
    let commands = match graft_common::parse_commands(&graft_yaml) {
        Ok(cmds) => cmds,
        Err(e) => {
            let _ = tx.send(CommandEvent::Failed(format!(
                "Failed to load graft.yaml: {e}"
            )));
            return;
        }
    };

    let Some(cmd_def) = commands.get(&lookup_name).cloned() else {
        let _ = tx.send(CommandEvent::Failed(format!(
            "Command '{command_name}' not found in graft.yaml"
        )));
        return;
    };

    // Resolve script paths: rewrite relative paths to absolute in source_dir
    let resolved_run = graft_engine::resolve_script_in_command(&cmd_def.run, &source_dir);

    // Build shell command: `run [args...]`
    let args_clone = args.clone();
    let mut full_parts = vec![resolved_run];
    full_parts.extend(args);
    let shell_cmd = full_parts.join(" ");

    // Working directory: defaults to consumer_dir (repo root), with working_dir
    // relative to consumer_dir
    let working_dir = if let Some(ref sub_dir) = cmd_def.working_dir {
        consumer_dir.join(sub_dir)
    } else {
        consumer_dir.clone()
    };

    // Inject GRAFT_DEP_DIR for dependency commands
    let env = if source_dir == consumer_dir {
        cmd_def.env
    } else {
        let mut env_map = cmd_def.env.unwrap_or_default();
        env_map.insert(
            "GRAFT_DEP_DIR".to_string(),
            source_dir.to_string_lossy().to_string(),
        );
        Some(env_map)
    };

    let logging = prepare_run_logging(run_ctx.as_ref());

    let config = ProcessConfig {
        command: shell_cmd.clone(),
        working_dir,
        env,
        log_path: logging.as_ref().map(|l| l.log_path.clone()),
        timeout: None,
        stdin: None,
    };

    let (_handle, rx) = match ProcessHandle::spawn(&config) {
        Ok(pair) => pair,
        Err(e) => {
            let _ = tx.send(CommandEvent::Failed(format!(
                "Failed to spawn process: {e}"
            )));
            return;
        }
    };

    // Bridge ProcessEvent → CommandEvent. The channel closes after Completed or Failed.
    bridge_events(rx, tx, logging.as_ref(), &args_clone, &shell_cmd);
}

/// Spawn a pre-assembled shell command in the background.
///
/// Unlike `spawn_command`, this does not re-read graft.yaml or look up a command name.
/// The caller has already assembled the full shell command (from the form overlay).
/// `working_dir_override` and `env` are forwarded from the original `CommandDef`.
#[allow(clippy::needless_pass_by_value)]
pub fn spawn_command_assembled(
    shell_cmd: String,
    repo_path: String,
    working_dir_override: Option<String>,
    env: Option<std::collections::HashMap<String, String>>,
    run_ctx: Option<RunContext>,
    tx: Sender<CommandEvent>,
) {
    let working_dir = if let Some(ref sub_dir) = working_dir_override {
        PathBuf::from(&repo_path).join(sub_dir)
    } else {
        PathBuf::from(&repo_path)
    };

    let logging = prepare_run_logging(run_ctx.as_ref());

    let config = ProcessConfig {
        command: shell_cmd.clone(),
        working_dir,
        env,
        log_path: logging.as_ref().map(|l| l.log_path.clone()),
        timeout: None,
        stdin: None,
    };

    let (_handle, rx) = match ProcessHandle::spawn(&config) {
        Ok(pair) => pair,
        Err(e) => {
            let _ = tx.send(CommandEvent::Failed(format!(
                "Failed to spawn process: {e}"
            )));
            return;
        }
    };

    bridge_events(rx, tx, logging.as_ref(), &[], &shell_cmd);
}

/// Bridge `ProcessEvent` to `CommandEvent`, writing run metadata on completion.
#[allow(clippy::needless_pass_by_value)]
fn bridge_events(
    rx: mpsc::Receiver<ProcessEvent>,
    tx: Sender<CommandEvent>,
    logging: Option<&RunLogging>,
    args: &[String],
    shell_cmd: &str,
) {
    for event in rx {
        match event {
            ProcessEvent::Started { pid } => {
                let _ = tx.send(CommandEvent::Started(pid));
            }
            ProcessEvent::OutputLine { line, .. } => {
                if tx.send(CommandEvent::OutputLine(line)).is_err() {
                    break;
                }
            }
            ProcessEvent::Completed { exit_code } => {
                if let Some(l) = logging {
                    write_run_completion_meta(l, args, shell_cmd, Some(exit_code));
                }
                let _ = tx.send(CommandEvent::Completed(exit_code));
            }
            ProcessEvent::Failed { error } => {
                if let Some(l) = logging {
                    write_run_completion_meta(l, args, shell_cmd, None);
                }
                let _ = tx.send(CommandEvent::Failed(error));
            }
        }
    }
}

impl<R: RepoRegistry, D: RepoDetailProvider> App<R, D> {
    /// Handle incoming command output events.
    pub(super) fn handle_command_events(&mut self) {
        let mut should_close = false;

        if let Some(rx) = &self.command_event_rx {
            while let Ok(event) = rx.try_recv() {
                match event {
                    CommandEvent::Started(pid) => {
                        self.running_command_pid = Some(pid);
                    }
                    CommandEvent::OutputLine(line) => {
                        self.output_lines.push(line);

                        if self.output_lines.len() > MAX_OUTPUT_LINES {
                            self.output_lines.drain(0..LINES_TO_DROP);

                            if !self.output_truncated_start {
                                self.output_lines.insert(
                                    0,
                                    format!(
                                        "... [earlier output truncated - showing last {MAX_OUTPUT_LINES} lines]"
                                    ),
                                );
                                self.output_truncated_start = true;
                            }

                            if self.output_scroll > LINES_TO_DROP {
                                self.output_scroll -= LINES_TO_DROP;
                            } else {
                                self.output_scroll = 0;
                            }
                        }
                    }
                    CommandEvent::Completed(exit_code) => {
                        self.command_state = CommandState::Completed { exit_code };

                        self.output_lines.push(String::new());

                        let unicode = supports_unicode();
                        if exit_code == 0 {
                            let symbol = if unicode { "✓" } else { "*" };
                            self.output_lines
                                .push(format!("{symbol} Command completed successfully"));
                        } else {
                            let symbol = if unicode { "✗" } else { "X" };
                            self.output_lines.push(format!(
                                "{symbol} Command failed with exit code {exit_code}"
                            ));
                        }

                        should_close = true;
                    }
                    CommandEvent::Failed(error) => {
                        self.command_state = CommandState::Failed { error };
                        should_close = true;
                    }
                }
            }
        }

        if should_close {
            self.command_event_rx = None;
        }
    }

    /// Execute a command with a pre-assembled shell command string (from form overlay).
    ///
    /// Unlike `execute_command_with_args`, this passes the already-assembled shell
    /// command directly to `spawn_command_assembled`, skipping arg splitting.
    /// The `working_dir` and `env` from the original command definition are forwarded.
    ///
    /// For both local and dependency commands, the base directory is the consumer's
    /// repo root (commands always execute in the consumer's context).
    pub(super) fn execute_command_assembled(
        &mut self,
        command_name: String,
        shell_cmd: String,
        working_dir: Option<String>,
        env: Option<std::collections::HashMap<String, String>>,
    ) {
        let Some(repo_path) = &self.selected_repo_for_commands else {
            return;
        };

        // Always use the consumer's repo root as base directory
        let base_dir = repo_path.clone();

        let run_ctx = self.build_run_context(repo_path, &command_name);

        let (tx, rx) = mpsc::channel();
        self.command_event_rx = Some(rx);

        self.command_name = Some(command_name);
        self.command_state = CommandState::Running;
        self.output_lines.clear();
        self.output_scroll = 0;
        self.output_truncated_start = false;
        self.running_command_pid = None;

        std::thread::spawn(move || {
            spawn_command_assembled(shell_cmd, base_dir, working_dir, env, run_ctx, tx);
        });
    }

    /// Execute command with provided arguments.
    pub(super) fn execute_command_with_args(&mut self, command_name: String, args: Vec<String>) {
        let Some(repo_path) = &self.selected_repo_for_commands else {
            return;
        };

        let run_ctx = self.build_run_context(repo_path, &command_name);

        let (tx, rx) = mpsc::channel();
        self.command_event_rx = Some(rx);

        self.command_name = Some(command_name.clone());
        self.command_state = CommandState::Running;
        self.output_lines.clear();
        self.output_scroll = 0;
        self.output_truncated_start = false;
        self.running_command_pid = None;

        let repo_path_clone = repo_path.clone();
        std::thread::spawn(move || {
            spawn_command(command_name, args, repo_path_clone, run_ctx, tx);
        });
    }

    /// Build a `RunContext` for run logging from the current workspace and repo path.
    fn build_run_context(&self, repo_path: &str, command_name: &str) -> Option<RunContext> {
        Some(RunContext {
            workspace: self.workspace_name.clone(),
            repo: graft_common::repo_name_from_path(repo_path).to_string(),
            command: command_name.to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    /// Helper: create a graft.yaml with the given YAML content inside a temp directory.
    /// Returns the TempDir (caller must hold it to keep the directory alive).
    fn setup_repo(yaml: &str) -> TempDir {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("graft.yaml"), yaml).unwrap();
        dir
    }

    /// Helper: create a dependency graft.yaml at .graft/<dep>/graft.yaml.
    fn setup_dep(repo_dir: &std::path::Path, dep_name: &str, yaml: &str) {
        let dep_dir = repo_dir.join(".graft").join(dep_name);
        fs::create_dir_all(&dep_dir).unwrap();
        fs::write(dep_dir.join("graft.yaml"), yaml).unwrap();
    }

    /// Collect all events from spawn_command into a Vec.
    fn collect_events(command_name: &str, args: Vec<String>, repo_path: &str) -> Vec<CommandEvent> {
        let (tx, rx) = mpsc::channel();
        spawn_command(
            command_name.to_string(),
            args,
            repo_path.to_string(),
            None,
            tx,
        );
        rx.into_iter().collect()
    }

    #[test]
    fn spawn_command_runs_local_command() {
        let repo = setup_repo(
            r#"
commands:
  greet:
    run: "echo hello"
"#,
        );

        let events = collect_events("greet", vec![], repo.path().to_str().unwrap());

        // Should have Started, at least one OutputLine, and Completed
        assert!(events.iter().any(|e| matches!(e, CommandEvent::Started(_))));
        assert!(events
            .iter()
            .any(|e| matches!(e, CommandEvent::OutputLine(s) if s.contains("hello"))));
        assert!(events
            .iter()
            .any(|e| matches!(e, CommandEvent::Completed(0))));
    }

    #[test]
    fn spawn_command_runs_dep_command() {
        let repo = setup_repo("commands: {}");
        setup_dep(
            repo.path(),
            "tools",
            r#"
commands:
  check:
    run: "echo dep-tools-check"
"#,
        );

        let events = collect_events("tools:check", vec![], repo.path().to_str().unwrap());

        assert!(events
            .iter()
            .any(|e| matches!(e, CommandEvent::OutputLine(s) if s.contains("dep-tools-check"))));
        assert!(events
            .iter()
            .any(|e| matches!(e, CommandEvent::Completed(0))));
    }

    #[test]
    fn spawn_command_dep_uses_consumer_working_dir() {
        let repo = setup_repo("commands: {}");
        setup_dep(
            repo.path(),
            "mylib",
            r#"
commands:
  pwd:
    run: "pwd"
"#,
        );

        let events = collect_events("mylib:pwd", vec![], repo.path().to_str().unwrap());

        // The working directory should be the consumer's repo root (not .graft/mylib/)
        let expected_dir = repo.path();
        assert!(events.iter().any(|e| matches!(
            e,
            CommandEvent::OutputLine(s) if s.trim() == expected_dir.to_str().unwrap()
        )));
    }

    #[test]
    fn spawn_command_dep_with_working_dir_override() {
        let repo = setup_repo("commands: {}");
        // Create subdir under the repo root (not the dep dir)
        let sub_dir = repo.path().join("subdir");
        fs::create_dir_all(&sub_dir).unwrap();
        setup_dep(
            repo.path(),
            "mylib",
            r#"
commands:
  pwd:
    run: "pwd"
    working_dir: "subdir"
"#,
        );

        let events = collect_events("mylib:pwd", vec![], repo.path().to_str().unwrap());

        // working_dir should resolve relative to consumer dir (repo root)
        assert!(events.iter().any(|e| matches!(
            e,
            CommandEvent::OutputLine(s) if s.trim() == sub_dir.to_str().unwrap()
        )));
    }

    #[test]
    fn spawn_command_dep_sets_graft_dep_dir() {
        let repo = setup_repo("commands: {}");
        setup_dep(
            repo.path(),
            "mylib",
            r#"
commands:
  env:
    run: "printenv GRAFT_DEP_DIR"
"#,
        );

        let events = collect_events("mylib:env", vec![], repo.path().to_str().unwrap());

        let expected_dep_dir = repo.path().join(".graft").join("mylib");
        assert!(events.iter().any(|e| matches!(
            e,
            CommandEvent::OutputLine(s) if s.trim() == expected_dep_dir.to_str().unwrap()
        )));
    }

    #[test]
    fn spawn_command_local_no_graft_dep_dir() {
        let repo = setup_repo(
            r#"
commands:
  env:
    run: "printenv GRAFT_DEP_DIR || echo NOT_SET"
"#,
        );

        let events = collect_events("env", vec![], repo.path().to_str().unwrap());

        // For local commands, GRAFT_DEP_DIR should not be set
        assert!(events
            .iter()
            .any(|e| matches!(e, CommandEvent::OutputLine(s) if s.contains("NOT_SET"))));
    }

    #[test]
    fn spawn_command_fails_for_missing_dep_command() {
        let repo = setup_repo("commands: {}");
        // No dep directory at all

        let events = collect_events("nodep:test", vec![], repo.path().to_str().unwrap());

        assert!(events.iter().any(|e| matches!(e, CommandEvent::Failed(_))));
    }

    #[test]
    fn spawn_command_assembled_uses_consumer_dir() {
        let repo = setup_repo("commands: {}");
        setup_dep(repo.path(), "lib", "commands: {}");

        let (tx, rx) = mpsc::channel();

        // execute_command_assembled now always uses the consumer's repo root
        spawn_command_assembled(
            "pwd".to_string(),
            repo.path().display().to_string(),
            None,
            None,
            None,
            tx,
        );

        let events: Vec<_> = rx.into_iter().collect();
        assert!(events.iter().any(|e| matches!(
            e,
            CommandEvent::OutputLine(s) if s.trim() == repo.path().to_str().unwrap()
        )));
    }
}
