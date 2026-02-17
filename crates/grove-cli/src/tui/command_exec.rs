//! Command execution and event handling.

use super::{
    mpsc, supports_unicode, App, CommandState, RepoDetailProvider, RepoRegistry, Sender,
    LINES_TO_DROP, MAX_OUTPUT_LINES,
};

/// Events from async command execution.
#[derive(Debug)]
pub enum CommandEvent {
    Started(u32), // Process PID
    OutputLine(String),
    Completed(i32),
    Failed(String),
}

/// Find the graft command, checking uv-managed installation first.
fn find_graft_command() -> anyhow::Result<String> {
    let uv_check = std::process::Command::new("uv")
        .args(["run", "--quiet", "python", "-m", "graft", "--help"])
        .current_dir("/tmp")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();

    if let Ok(status) = uv_check {
        if status.success() {
            return Ok("uv run python -m graft".to_string());
        }
    }

    let system_check = std::process::Command::new("graft")
        .arg("--help")
        .current_dir("/tmp")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();

    if let Ok(status) = system_check {
        if status.success() {
            return Ok("graft".to_string());
        }
    }

    Err(anyhow::anyhow!(
        "graft command not found\n\n\
         Grove requires graft to execute commands.\n\n\
         Install graft:\n\
         - Via uv: uv pip install graft\n\
         - Via pip: pip install graft\n\n\
         Or ensure graft is in your PATH."
    ))
}

/// Spawn a graft command in the background and send output via channel.
#[allow(clippy::needless_pass_by_value)]
pub fn spawn_command(
    command_name: String,
    args: Vec<String>,
    repo_path: String,
    tx: Sender<CommandEvent>,
) {
    use std::io::{BufRead, BufReader};
    use std::process::{Command, Stdio};

    let graft_cmd = match find_graft_command() {
        Ok(cmd) => cmd,
        Err(e) => {
            let _ = tx.send(CommandEvent::Failed(e.to_string()));
            return;
        }
    };

    let result = if graft_cmd.starts_with("uv run") {
        let mut cmd = Command::new("uv");
        cmd.args(["run", "python", "-m", "graft", "run", &command_name]);

        if !args.is_empty() {
            cmd.args(&args);
        }

        cmd.current_dir(&repo_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
    } else {
        let mut cmd = Command::new(&graft_cmd);
        cmd.args(["run", &command_name]);

        if !args.is_empty() {
            cmd.args(&args);
        }

        cmd.current_dir(&repo_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
    };

    let mut child = match result {
        Ok(child) => child,
        Err(e) => {
            let _ = tx.send(CommandEvent::Failed(format!(
                "Failed to spawn {graft_cmd}: {e}\n\n\
                 Ensure graft is properly installed and in PATH."
            )));
            return;
        }
    };

    let _ = tx.send(CommandEvent::Started(child.id()));

    let stdout = child.stdout.take();
    let tx_stdout = tx.clone();
    let stdout_thread = stdout.map(|out| {
        std::thread::spawn(move || {
            let reader = BufReader::new(out);
            for line in reader.lines() {
                match line {
                    Ok(line) => {
                        if tx_stdout.send(CommandEvent::OutputLine(line)).is_err() {
                            break;
                        }
                    }
                    Err(e) => {
                        let _ = tx_stdout.send(CommandEvent::Failed(format!("Read error: {e}")));
                        break;
                    }
                }
            }
        })
    });

    let stderr = child.stderr.take();
    let tx_stderr = tx.clone();
    let stderr_thread = stderr.map(|err| {
        std::thread::spawn(move || {
            let reader = BufReader::new(err);
            for line in reader.lines() {
                match line {
                    Ok(line) => {
                        if tx_stderr.send(CommandEvent::OutputLine(line)).is_err() {
                            break;
                        }
                    }
                    Err(e) => {
                        let _ = tx_stderr.send(CommandEvent::Failed(format!("Read error: {e}")));
                        break;
                    }
                }
            }
        })
    });

    match child.wait() {
        Ok(status) => {
            if let Some(thread) = stdout_thread {
                let _ = thread.join();
            }
            if let Some(thread) = stderr_thread {
                let _ = thread.join();
            }

            let exit_code = status.code().unwrap_or(-1);
            let _ = tx.send(CommandEvent::Completed(exit_code));
        }
        Err(e) => {
            let _ = tx.send(CommandEvent::Failed(format!("Wait failed: {e}")));
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
