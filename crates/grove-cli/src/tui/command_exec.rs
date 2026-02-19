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
    // Load commands from graft.yaml.
    let graft_yaml = PathBuf::from(&repo_path).join("graft.yaml");
    let commands = match graft_common::parse_commands(&graft_yaml) {
        Ok(cmds) => cmds,
        Err(e) => {
            let _ = tx.send(CommandEvent::Failed(format!(
                "Failed to load graft.yaml: {e}"
            )));
            return;
        }
    };

    let Some(cmd_def) = commands.get(&command_name).cloned() else {
        let _ = tx.send(CommandEvent::Failed(format!(
            "Command '{command_name}' not found in graft.yaml"
        )));
        return;
    };

    // Build shell command: `run [args...]`
    let mut full_parts = vec![cmd_def.run.clone()];
    full_parts.extend(args);
    let shell_cmd = full_parts.join(" ");

    // Determine working directory (relative to repo_path if specified).
    let working_dir = if let Some(ref sub_dir) = cmd_def.working_dir {
        PathBuf::from(&repo_path).join(sub_dir)
    } else {
        PathBuf::from(&repo_path)
    };

    let config = ProcessConfig {
        command: shell_cmd,
        working_dir,
        env: cmd_def.env,
        log_path: None,
        timeout: None,
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
