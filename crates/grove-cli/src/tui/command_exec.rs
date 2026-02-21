//! Command execution and event handling.

use super::{
    mpsc, supports_unicode, App, CommandState, RepoDetailProvider, RepoRegistry, Sender,
    LINES_TO_DROP, MAX_OUTPUT_LINES,
};
use graft_common::process::{ProcessConfig, ProcessEvent, ProcessHandle};
use std::path::PathBuf;

/// Events from async command execution.
#[derive(Debug)]
pub enum CommandEvent {
    Started(u32), // Process PID
    OutputLine(String),
    Completed(i32),
    Failed(String),
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
    tx: Sender<CommandEvent>,
) {
    // Parse qualified command name (dep:cmd vs local)
    let (graft_yaml, lookup_name, base_dir) = if let Some((dep, cmd)) = command_name.split_once(':')
    {
        // Dependency command: load from .graft/<dep>/graft.yaml
        let path = PathBuf::from(&repo_path)
            .join(".graft")
            .join(dep)
            .join("graft.yaml");
        let dir = PathBuf::from(&repo_path).join(".graft").join(dep);
        (path, cmd.to_string(), dir)
    } else {
        // Local command
        let path = PathBuf::from(&repo_path).join("graft.yaml");
        let dir = PathBuf::from(&repo_path);
        (path, command_name.clone(), dir)
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

    // Build shell command: `run [args...]`
    let mut full_parts = vec![cmd_def.run.clone()];
    full_parts.extend(args);
    let shell_cmd = full_parts.join(" ");

    // Determine working directory (relative to base_dir if specified).
    let working_dir = if let Some(ref sub_dir) = cmd_def.working_dir {
        base_dir.join(sub_dir)
    } else {
        base_dir
    };

    let config = ProcessConfig {
        command: shell_cmd,
        working_dir,
        env: cmd_def.env,
        log_path: None,
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
                let _ = tx.send(CommandEvent::Completed(exit_code));
            }
            ProcessEvent::Failed { error } => {
                let _ = tx.send(CommandEvent::Failed(error));
            }
        }
    }
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
    tx: Sender<CommandEvent>,
) {
    let working_dir = if let Some(ref sub_dir) = working_dir_override {
        PathBuf::from(&repo_path).join(sub_dir)
    } else {
        PathBuf::from(&repo_path)
    };

    let config = ProcessConfig {
        command: shell_cmd,
        working_dir,
        env,
        log_path: None,
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
                let _ = tx.send(CommandEvent::Completed(exit_code));
            }
            ProcessEvent::Failed { error } => {
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
    /// For dependency commands (`dep:cmd`), the base directory is resolved to
    /// `.graft/<dep>/` so that `working_dir` is relative to the dep directory.
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

        // Compute the effective base directory: for dep:cmd, use .graft/<dep>/
        let base_dir = if let Some((dep, _)) = command_name.split_once(':') {
            PathBuf::from(repo_path)
                .join(".graft")
                .join(dep)
                .display()
                .to_string()
        } else {
            repo_path.clone()
        };

        let (tx, rx) = mpsc::channel();
        self.command_event_rx = Some(rx);

        self.command_name = Some(command_name);
        self.command_state = CommandState::Running;
        self.output_lines.clear();
        self.output_scroll = 0;
        self.output_truncated_start = false;
        self.running_command_pid = None;

        std::thread::spawn(move || {
            spawn_command_assembled(shell_cmd, base_dir, working_dir, env, tx);
        });
    }

    /// Execute command with provided arguments.
    pub(super) fn execute_command_with_args(&mut self, command_name: String, args: Vec<String>) {
        let Some(repo_path) = &self.selected_repo_for_commands else {
            return;
        };

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
            spawn_command(command_name, args, repo_path_clone, tx);
        });
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
        spawn_command(command_name.to_string(), args, repo_path.to_string(), tx);
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
    fn spawn_command_dep_uses_dep_working_dir() {
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

        // The working directory should be .graft/mylib/
        let expected_dir = repo.path().join(".graft").join("mylib");
        assert!(events.iter().any(|e| matches!(
            e,
            CommandEvent::OutputLine(s) if s.trim() == expected_dir.to_str().unwrap()
        )));
    }

    #[test]
    fn spawn_command_dep_with_working_dir_override() {
        let repo = setup_repo("commands: {}");
        let dep_dir = repo.path().join(".graft").join("mylib");
        let sub_dir = dep_dir.join("subdir");
        fs::create_dir_all(&sub_dir).unwrap();
        fs::write(
            dep_dir.join("graft.yaml"),
            r#"
commands:
  pwd:
    run: "pwd"
    working_dir: "subdir"
"#,
        )
        .unwrap();

        let events = collect_events("mylib:pwd", vec![], repo.path().to_str().unwrap());

        // working_dir should resolve relative to .graft/mylib/, not repo root
        assert!(events.iter().any(|e| matches!(
            e,
            CommandEvent::OutputLine(s) if s.trim() == sub_dir.to_str().unwrap()
        )));
    }

    #[test]
    fn spawn_command_fails_for_missing_dep_command() {
        let repo = setup_repo("commands: {}");
        // No dep directory at all

        let events = collect_events("nodep:test", vec![], repo.path().to_str().unwrap());

        assert!(events.iter().any(|e| matches!(e, CommandEvent::Failed(_))));
    }

    #[test]
    fn spawn_command_assembled_uses_base_dir() {
        let repo = setup_repo("commands: {}");
        setup_dep(repo.path(), "lib", "commands: {}");

        let dep_dir = repo.path().join(".graft").join("lib");
        let (tx, rx) = mpsc::channel();

        // Simulate what execute_command_assembled does for dep commands:
        // pass the dep directory as repo_path (base_dir)
        spawn_command_assembled(
            "pwd".to_string(),
            dep_dir.display().to_string(),
            None,
            None,
            tx,
        );

        let events: Vec<_> = rx.into_iter().collect();
        assert!(events.iter().any(|e| matches!(
            e,
            CommandEvent::OutputLine(s) if s.trim() == dep_dir.to_str().unwrap()
        )));
    }
}
