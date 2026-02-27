//! Command execution and event handling.

use graft_common::process::{ProcessConfig, ProcessEvent, ProcessHandle};
use graft_common::runs::{run_log_path, write_run_meta, RunMeta};
use std::path::PathBuf;
use std::sync::mpsc::{self, Sender};

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
    /// Path to the run log file (sent once when the process spawns successfully).
    LogPath(PathBuf),
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
#[allow(clippy::too_many_lines)]
pub fn spawn_command(
    command_name: String,
    args: Vec<String>,
    repo_path: String,
    run_ctx: Option<RunContext>,
    tx: Sender<CommandEvent>,
) {
    // Detect sequence dispatch: names prefixed with "» " are sequences.
    // Strip the prefix and run via `graft run <seq_name>` in the consumer directory.
    if let Some(seq_name) = command_name.strip_prefix("» ") {
        let consumer_dir = PathBuf::from(&repo_path);

        let logging = prepare_run_logging(run_ctx.as_ref());

        let run_state_dir = consumer_dir.join(".graft").join("run-state");
        let _ = std::fs::create_dir_all(&run_state_dir);

        let mut full_parts = vec!["graft".to_string(), "run".to_string(), seq_name.to_string()];
        full_parts.extend(args);
        let shell_cmd = full_parts.join(" ");

        let config = ProcessConfig {
            command: shell_cmd.clone(),
            working_dir: consumer_dir.clone(),
            env: {
                let mut env_map = std::collections::HashMap::new();
                env_map.insert(
                    "GRAFT_STATE_DIR".to_string(),
                    run_state_dir.to_string_lossy().to_string(),
                );
                Some(env_map)
            },
            env_remove: vec![],
            log_path: logging.as_ref().map(|l| l.log_path.clone()),
            timeout: None,
            stdin: None,
        };

        let (_handle, rx) = match ProcessHandle::spawn(&config) {
            Ok(pair) => pair,
            Err(e) => {
                let _ = tx.send(CommandEvent::Failed(format!(
                    "Failed to spawn sequence process: {e}"
                )));
                return;
            }
        };

        if let Some(ref l) = logging {
            let _ = tx.send(CommandEvent::LogPath(l.log_path.clone()));
        }

        bridge_events(rx, tx, logging.as_ref(), &[], &shell_cmd);
        return;
    }

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

    // Build shell command: substitute named {placeholder} tokens with shell-quoted arg
    // values; fall back to appending shell-quoted args when no placeholders are present.
    let args_clone = args.clone();
    let (substituted, had_placeholders) =
        graft_engine::substitute_placeholders(&resolved_run, &args);
    let shell_cmd = if had_placeholders {
        substituted
    } else if args.is_empty() {
        resolved_run
    } else {
        let quoted_args = args
            .iter()
            .map(|a| shell_words::quote(a).into_owned())
            .collect::<Vec<_>>();
        format!("{resolved_run} {}", quoted_args.join(" "))
    };

    // Working directory: defaults to consumer_dir (repo root), with working_dir
    // relative to consumer_dir
    let working_dir = if let Some(ref sub_dir) = cmd_def.working_dir {
        consumer_dir.join(sub_dir)
    } else {
        consumer_dir.clone()
    };

    // Inject GRAFT_STATE_DIR (always) and GRAFT_DEP_DIR (dep commands only)
    let env = {
        let mut env_map = cmd_def.env.unwrap_or_default();
        let run_state_dir = consumer_dir.join(".graft").join("run-state");
        let _ = std::fs::create_dir_all(&run_state_dir);
        env_map.insert(
            "GRAFT_STATE_DIR".to_string(),
            run_state_dir.to_string_lossy().to_string(),
        );
        if source_dir != consumer_dir {
            env_map.insert(
                "GRAFT_DEP_DIR".to_string(),
                source_dir.to_string_lossy().to_string(),
            );
        }
        Some(env_map)
    };

    let logging = prepare_run_logging(run_ctx.as_ref());

    // For local commands (source_dir == consumer_dir), unset GRAFT_DEP_DIR to prevent
    // inheriting it from a parent shell that was invoked from a dep command.
    let env_remove = if source_dir == consumer_dir {
        vec!["GRAFT_DEP_DIR".to_string()]
    } else {
        vec![]
    };

    let config = ProcessConfig {
        command: shell_cmd.clone(),
        working_dir,
        env,
        env_remove,
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

    if let Some(ref l) = logging {
        let _ = tx.send(CommandEvent::LogPath(l.log_path.clone()));
    }

    // Bridge ProcessEvent → CommandEvent. The channel closes after Completed or Failed.
    bridge_events(rx, tx, logging.as_ref(), &args_clone, &shell_cmd);
}

/// Spawn a pre-assembled shell command in the background.
///
/// Unlike `spawn_command`, this does not re-read graft.yaml or look up a command name.
/// The caller has already assembled the full shell command (from the form overlay).
/// `working_dir_override` and `env` are forwarded from the original `CommandDef`.
#[allow(clippy::needless_pass_by_value, dead_code)]
pub fn spawn_command_assembled(
    shell_cmd: String,
    repo_path: String,
    working_dir_override: Option<String>,
    env: Option<std::collections::HashMap<String, String>>,
    run_ctx: Option<RunContext>,
    tx: Sender<CommandEvent>,
) {
    let consumer_dir = PathBuf::from(&repo_path);
    let working_dir = if let Some(ref sub_dir) = working_dir_override {
        consumer_dir.join(sub_dir)
    } else {
        consumer_dir.clone()
    };

    // Inject GRAFT_STATE_DIR into the environment
    let env = {
        let mut env_map = env.unwrap_or_default();
        let run_state_dir = consumer_dir.join(".graft").join("run-state");
        let _ = std::fs::create_dir_all(&run_state_dir);
        env_map.insert(
            "GRAFT_STATE_DIR".to_string(),
            run_state_dir.to_string_lossy().to_string(),
        );
        Some(env_map)
    };

    let logging = prepare_run_logging(run_ctx.as_ref());

    let config = ProcessConfig {
        command: shell_cmd.clone(),
        working_dir,
        env,
        env_remove: vec![],
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

    if let Some(ref l) = logging {
        let _ = tx.send(CommandEvent::LogPath(l.log_path.clone()));
    }

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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    /// Helper: create a graft.yaml with the given YAML content inside a temp directory.
    /// Returns the `TempDir` (caller must hold it to keep the directory alive).
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

    /// Collect all events from `spawn_command` into a Vec.
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

    #[test]
    fn spawn_command_substitutes_placeholder_with_arg_value() {
        let repo = setup_repo(
            r#"
commands:
  greet:
    run: "echo {name}"
"#,
        );

        let events = collect_events(
            "greet",
            vec!["hello".to_string()],
            repo.path().to_str().unwrap(),
        );

        // The arg value should appear in the output
        assert!(
            events
                .iter()
                .any(|e| matches!(e, CommandEvent::OutputLine(s) if s.contains("hello"))),
            "arg value should be substituted into the run string"
        );
        // The literal placeholder must not appear
        assert!(
            !events
                .iter()
                .any(|e| matches!(e, CommandEvent::OutputLine(s) if s.contains("{name}"))),
            "literal placeholder must not reach the shell"
        );
        assert!(events
            .iter()
            .any(|e| matches!(e, CommandEvent::Completed(0))));
    }

    #[test]
    fn spawn_command_substitutes_placeholder_with_spaced_arg() {
        let repo = setup_repo(
            r#"
commands:
  echo-msg:
    run: "echo {msg}"
"#,
        );

        let events = collect_events(
            "echo-msg",
            vec!["hello world".to_string()],
            repo.path().to_str().unwrap(),
        );

        // Both words should appear (shell quoting preserved the space)
        assert!(
            events
                .iter()
                .any(|e| matches!(e, CommandEvent::OutputLine(s) if s.contains("hello world"))),
            "spaced arg should be passed as a single argument via shell quoting"
        );
        assert!(events
            .iter()
            .any(|e| matches!(e, CommandEvent::Completed(0))));
    }
}
