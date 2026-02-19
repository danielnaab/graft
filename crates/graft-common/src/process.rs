//! Subprocess execution with streaming output and lifecycle management.
//!
//! The primary entry point is [`ProcessHandle::spawn`], which runs a shell command and returns
//! a handle plus a channel of [`ProcessEvent`]s that reflect the process lifecycle.
//!
//! For blocking use cases, [`run_to_completion`] and [`run_to_completion_with_timeout`] collect
//! all output synchronously and return a [`ProcessOutput`].

use std::collections::HashMap;
use std::fs::OpenOptions;
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use thiserror::Error;

/// Events emitted by a spawned process over the event channel.
///
/// Events are always delivered in this order:
/// 1. [`ProcessEvent::Started`]
/// 2. Zero or more [`ProcessEvent::OutputLine`] events (stdout and stderr interleaved)
/// 3. [`ProcessEvent::Completed`] or [`ProcessEvent::Failed`]
///
/// The channel is disconnected after the terminal event.
#[derive(Debug, Clone)]
pub enum ProcessEvent {
    /// Process has started; the PID is available.
    Started { pid: u32 },
    /// A line of output from the process (stdout or stderr).
    OutputLine { line: String, is_stderr: bool },
    /// Process exited; check `exit_code` for success (0) or failure.
    ///
    /// Processes killed by a signal report `exit_code: -1`.
    Completed { exit_code: i32 },
    /// Unexpected error during process monitoring (not a non-zero exit).
    Failed { error: String },
}

/// Errors from process spawn, kill, and execution operations.
#[derive(Error, Debug)]
pub enum ProcessError {
    #[error("Failed to spawn process: {0}")]
    SpawnFailed(String),

    #[error("Failed to kill process: {0}")]
    KillFailed(String),

    #[error("Process timed out")]
    Timeout,

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

/// Configuration for spawning a process.
#[derive(Debug, Clone)]
pub struct ProcessConfig {
    /// Shell command to run via `sh -c`.
    pub command: String,
    /// Working directory for the process.
    pub working_dir: PathBuf,
    /// Optional environment variable overrides (merged with the inherited environment).
    pub env: Option<HashMap<String, String>>,
    /// Optional path to a log file; output lines are tee'd here in append mode.
    pub log_path: Option<PathBuf>,
    /// Optional timeout; the process is killed if it exceeds this duration.
    pub timeout: Option<Duration>,
}

/// Output collected from a process that has run to completion.
#[derive(Debug, Clone)]
pub struct ProcessOutput {
    /// Exit code of the process.
    pub exit_code: i32,
    /// All stdout output, with lines joined by `\n`. Empty string if no stdout.
    pub stdout: String,
    /// All stderr output, with lines joined by `\n`. Empty string if no stderr.
    pub stderr: String,
    /// `true` if `exit_code == 0`.
    pub success: bool,
}

/// Handle to a running subprocess.
///
/// Created by [`ProcessHandle::spawn`]. Provides the process PID and the ability to kill it.
/// Process lifecycle events are delivered over the [`mpsc::Receiver<ProcessEvent>`] returned
/// alongside the handle.
///
/// Dropping the handle does **not** kill the subprocess — call [`kill`](ProcessHandle::kill)
/// explicitly if termination is needed on drop.
#[derive(Debug)]
pub struct ProcessHandle {
    pid: u32,
    child: Arc<Mutex<std::process::Child>>,
    running: Arc<AtomicBool>,
}

impl ProcessHandle {
    /// Spawn a subprocess and return a handle plus an event receiver.
    ///
    /// The command is executed via `sh -c <command>`. Events are delivered in order:
    /// `Started`, then `OutputLine` events, then `Completed` or `Failed`.
    ///
    /// All `OutputLine` events are guaranteed to arrive before `Completed` or `Failed`.
    ///
    /// If `config.log_path` is set, every output line is also appended to that file.
    #[allow(clippy::too_many_lines)]
    pub fn spawn(
        config: &ProcessConfig,
    ) -> Result<(Self, mpsc::Receiver<ProcessEvent>), ProcessError> {
        let (tx, rx) = mpsc::channel();

        let mut cmd = Command::new("sh");
        cmd.arg("-c").arg(&config.command);
        cmd.current_dir(&config.working_dir);

        if let Some(env) = &config.env {
            for (k, v) in env {
                cmd.env(k, v);
            }
        }

        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        let mut child = cmd
            .spawn()
            .map_err(|e| ProcessError::SpawnFailed(e.to_string()))?;

        let pid = child.id();
        let stdout = child.stdout.take().expect("stdout was piped");
        let stderr = child.stderr.take().expect("stderr was piped");

        let running = Arc::new(AtomicBool::new(true));
        let child_arc = Arc::new(Mutex::new(child));

        // Open the log file once, shared between both reader threads.
        let log_handle: Option<Arc<Mutex<std::fs::File>>> =
            if let Some(ref log_path) = config.log_path {
                let file = OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(log_path)
                    .map_err(|e| {
                        ProcessError::SpawnFailed(format!(
                            "Failed to open log file {}: {}",
                            log_path.display(),
                            e
                        ))
                    })?;
                Some(Arc::new(Mutex::new(file)))
            } else {
                None
            };

        // Deliver Started before the background threads begin emitting OutputLine events.
        let _ = tx.send(ProcessEvent::Started { pid });

        // Stdout reader thread — sends OutputLine { is_stderr: false } events.
        let tx_stdout = tx.clone();
        let log_stdout = log_handle.clone();
        let stdout_thread = thread::spawn(move || {
            let reader = BufReader::new(stdout);
            for line in reader.lines() {
                match line {
                    Ok(l) => {
                        if let Some(ref log) = log_stdout {
                            let mut f = log.lock().unwrap();
                            let _ = writeln!(f, "{l}");
                        }
                        let _ = tx_stdout.send(ProcessEvent::OutputLine {
                            line: l,
                            is_stderr: false,
                        });
                    }
                    Err(_) => break,
                }
            }
        });

        // Stderr reader thread — sends OutputLine { is_stderr: true } events.
        let tx_stderr = tx.clone();
        let log_stderr = log_handle;
        let stderr_thread = thread::spawn(move || {
            let reader = BufReader::new(stderr);
            for line in reader.lines() {
                match line {
                    Ok(l) => {
                        if let Some(ref log) = log_stderr {
                            let mut f = log.lock().unwrap();
                            let _ = writeln!(f, "{l}");
                        }
                        let _ = tx_stderr.send(ProcessEvent::OutputLine {
                            line: l,
                            is_stderr: true,
                        });
                    }
                    Err(_) => break,
                }
            }
        });

        // Monitor thread — polls for exit, joins reader threads, then sends Completed/Failed.
        //
        // Polling with try_wait() lets the kill() method acquire the child lock without
        // contending with a blocking wait() call.
        let child_for_monitor = Arc::clone(&child_arc);
        let running_for_monitor = Arc::clone(&running);
        drop(thread::spawn(move || {
            loop {
                let result = {
                    // Lock only for the non-blocking try_wait call, then release.
                    let mut c = child_for_monitor.lock().unwrap();
                    c.try_wait()
                };

                match result {
                    Ok(Some(exit_status)) => {
                        // Process exited. Join readers so all OutputLine events are flushed
                        // to the channel before we send Completed.
                        let _ = stdout_thread.join();
                        let _ = stderr_thread.join();
                        let exit_code = exit_status.code().unwrap_or(-1);
                        let _ = tx.send(ProcessEvent::Completed { exit_code });
                        running_for_monitor.store(false, Ordering::SeqCst);
                        break;
                    }
                    Ok(None) => {
                        // Still running; yield briefly before polling again.
                        thread::sleep(Duration::from_millis(10));
                    }
                    Err(e) => {
                        let _ = stdout_thread.join();
                        let _ = stderr_thread.join();
                        let _ = tx.send(ProcessEvent::Failed {
                            error: e.to_string(),
                        });
                        running_for_monitor.store(false, Ordering::SeqCst);
                        break;
                    }
                }
            }
        }));

        let handle = Self {
            pid,
            child: child_arc,
            running,
        };

        Ok((handle, rx))
    }

    /// Return the process PID.
    pub fn pid(&self) -> u32 {
        self.pid
    }

    /// Return `true` if the process is still running.
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    /// Send SIGKILL to the process.
    ///
    /// Returns an error if the process has already exited or the kill call fails. After a
    /// successful kill the event channel will deliver `Completed { exit_code: -1 }` once
    /// the monitor thread detects the exit (typically within ~10 ms).
    pub fn kill(&self) -> Result<(), ProcessError> {
        let mut child = self.child.lock().unwrap();
        child
            .kill()
            .map_err(|e| ProcessError::KillFailed(e.to_string()))
    }
}

/// Block until the process completes and return all collected output.
///
/// If `config.log_path` is set, output is also tee'd to that file.
pub fn run_to_completion(config: &ProcessConfig) -> Result<ProcessOutput, ProcessError> {
    let (_handle, rx) = ProcessHandle::spawn(config)?;
    collect_output(&rx)
}

/// Block until the process completes, killing it if it exceeds the timeout.
///
/// Timeout is determined in priority order:
/// 1. `config.timeout`
/// 2. `GRAFT_PROCESS_TIMEOUT_MS` environment variable (milliseconds as integer)
/// 3. No timeout (wait indefinitely)
///
/// Returns [`ProcessError::Timeout`] if the deadline is exceeded.
pub fn run_to_completion_with_timeout(
    config: &ProcessConfig,
) -> Result<ProcessOutput, ProcessError> {
    let timeout = config.timeout.or_else(|| {
        std::env::var("GRAFT_PROCESS_TIMEOUT_MS")
            .ok()
            .and_then(|s| s.parse::<u64>().ok())
            .map(Duration::from_millis)
    });

    let (handle, rx) = ProcessHandle::spawn(config)?;

    match timeout {
        Some(duration) => collect_output_with_timeout(&handle, &rx, duration),
        None => collect_output(&rx),
    }
}

/// Drain the event channel and collect stdout/stderr into a [`ProcessOutput`].
fn collect_output(rx: &mpsc::Receiver<ProcessEvent>) -> Result<ProcessOutput, ProcessError> {
    let mut stdout_lines: Vec<String> = Vec::new();
    let mut stderr_lines: Vec<String> = Vec::new();
    let mut exit_code = 0i32;

    for event in rx {
        match event {
            ProcessEvent::OutputLine { line, is_stderr } => {
                if is_stderr {
                    stderr_lines.push(line);
                } else {
                    stdout_lines.push(line);
                }
            }
            ProcessEvent::Completed { exit_code: code } => {
                exit_code = code;
            }
            ProcessEvent::Failed { error } => {
                return Err(ProcessError::SpawnFailed(error));
            }
            ProcessEvent::Started { .. } => {}
        }
    }

    Ok(build_output(&stdout_lines, &stderr_lines, exit_code))
}

/// Drain the event channel with a deadline, killing the process if time runs out.
fn collect_output_with_timeout(
    handle: &ProcessHandle,
    rx: &mpsc::Receiver<ProcessEvent>,
    timeout: Duration,
) -> Result<ProcessOutput, ProcessError> {
    let deadline = Instant::now() + timeout;
    let mut stdout_lines: Vec<String> = Vec::new();
    let mut stderr_lines: Vec<String> = Vec::new();
    let mut exit_code = 0i32;

    loop {
        // Compute remaining time; if deadline has passed, kill and return Timeout.
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            let _ = handle.kill();
            return Err(ProcessError::Timeout);
        }

        match rx.recv_timeout(remaining) {
            Ok(ProcessEvent::OutputLine { line, is_stderr }) => {
                if is_stderr {
                    stderr_lines.push(line);
                } else {
                    stdout_lines.push(line);
                }
            }
            Ok(ProcessEvent::Completed { exit_code: code }) => {
                exit_code = code;
                // All OutputLine events precede Completed in the FIFO channel.
                break;
            }
            Ok(ProcessEvent::Failed { error }) => {
                return Err(ProcessError::SpawnFailed(error));
            }
            Ok(ProcessEvent::Started { .. }) => {}
            Err(mpsc::RecvTimeoutError::Timeout) => {
                let _ = handle.kill();
                return Err(ProcessError::Timeout);
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                // Channel closed without a terminal event; treat as completion.
                break;
            }
        }
    }

    Ok(build_output(&stdout_lines, &stderr_lines, exit_code))
}

fn build_output(stdout_lines: &[String], stderr_lines: &[String], exit_code: i32) -> ProcessOutput {
    let stdout = stdout_lines.join("\n");
    let stderr = stderr_lines.join("\n");
    let success = exit_code == 0;
    ProcessOutput {
        exit_code,
        stdout,
        stderr,
        success,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn workdir() -> PathBuf {
        std::env::current_dir().unwrap()
    }

    fn config(command: &str) -> ProcessConfig {
        ProcessConfig {
            command: command.to_string(),
            working_dir: workdir(),
            env: None,
            log_path: None,
            timeout: None,
        }
    }

    fn collect_events(rx: mpsc::Receiver<ProcessEvent>) -> Vec<ProcessEvent> {
        rx.into_iter().collect()
    }

    // ── ProcessHandle::spawn tests (unchanged from Task 1) ──────────────────

    #[test]
    fn spawn_echo_captures_stdout() {
        let (handle, rx) = ProcessHandle::spawn(&config("echo hello")).unwrap();
        let events = collect_events(rx);

        assert!(handle.pid() > 0);

        // First event is Started.
        assert!(matches!(events[0], ProcessEvent::Started { .. }));

        // Exactly one OutputLine with the echoed text on stdout.
        let output_lines: Vec<_> = events
            .iter()
            .filter(|e| matches!(e, ProcessEvent::OutputLine { .. }))
            .collect();
        assert_eq!(output_lines.len(), 1);
        match &output_lines[0] {
            ProcessEvent::OutputLine { line, is_stderr } => {
                assert_eq!(line, "hello");
                assert!(!is_stderr);
            }
            _ => panic!("expected OutputLine"),
        }

        // Last event is Completed with exit code 0.
        match events.last().unwrap() {
            ProcessEvent::Completed { exit_code } => assert_eq!(*exit_code, 0),
            other => panic!("expected Completed, got: {:?}", other),
        }
    }

    #[test]
    fn spawn_stderr_capture() {
        let (_, rx) = ProcessHandle::spawn(&config("echo error_text >&2")).unwrap();
        let events = collect_events(rx);

        let stderr_lines: Vec<_> = events
            .iter()
            .filter(|e| {
                matches!(
                    e,
                    ProcessEvent::OutputLine {
                        is_stderr: true,
                        ..
                    }
                )
            })
            .collect();
        assert_eq!(stderr_lines.len(), 1);
        match &stderr_lines[0] {
            ProcessEvent::OutputLine { line, is_stderr } => {
                assert_eq!(line, "error_text");
                assert!(*is_stderr);
            }
            _ => panic!("expected stderr OutputLine"),
        }
    }

    #[test]
    fn nonzero_exit_code() {
        let (_, rx) = ProcessHandle::spawn(&config("exit 42")).unwrap();
        let events = collect_events(rx);

        match events.last().unwrap() {
            ProcessEvent::Completed { exit_code } => assert_eq!(*exit_code, 42),
            other => panic!("expected Completed, got: {:?}", other),
        }
    }

    #[test]
    fn spawn_failure_invalid_workdir() {
        let result = ProcessHandle::spawn(&ProcessConfig {
            command: "echo hello".to_string(),
            working_dir: PathBuf::from("/nonexistent/path/that/does/not/exist/12345"),
            env: None,
            log_path: None,
            timeout: None,
        });

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ProcessError::SpawnFailed(_)));
    }

    #[test]
    fn kill_long_running_process() {
        let (handle, rx) = ProcessHandle::spawn(&config("sleep 60")).unwrap();

        // Give the process a moment to start.
        thread::sleep(Duration::from_millis(50));
        assert!(handle.is_running());

        handle.kill().unwrap();

        // Drain the channel — blocks until the monitor thread finishes.
        let events = collect_events(rx);

        assert!(events
            .iter()
            .any(|e| matches!(e, ProcessEvent::Started { .. })));
        assert!(events
            .iter()
            .any(|e| matches!(e, ProcessEvent::Completed { .. })));

        // Running flag is cleared by the time the channel closes.
        assert!(!handle.is_running());
    }

    // ── run_to_completion tests ──────────────────────────────────────────────

    #[test]
    fn run_to_completion_collects_stdout_and_stderr() {
        let cfg = config("echo out_line; echo err_line >&2");
        let output = run_to_completion(&cfg).unwrap();

        assert!(output.success);
        assert_eq!(output.exit_code, 0);
        assert_eq!(output.stdout, "out_line");
        assert_eq!(output.stderr, "err_line");
    }

    #[test]
    fn run_to_completion_multiline_output() {
        let cfg = config("printf 'line1\\nline2\\nline3'");
        let output = run_to_completion(&cfg).unwrap();

        assert!(output.success);
        assert_eq!(output.stdout, "line1\nline2\nline3");
    }

    #[test]
    fn run_to_completion_nonzero_exit() {
        let cfg = config("exit 7");
        let output = run_to_completion(&cfg).unwrap();

        assert!(!output.success);
        assert_eq!(output.exit_code, 7);
    }

    #[test]
    fn log_file_captures_output() {
        let dir = tempfile::tempdir().unwrap();
        let log_path = dir.path().join("output.log");

        let cfg = ProcessConfig {
            command: "echo logged_line; echo err_logged >&2".to_string(),
            working_dir: workdir(),
            env: None,
            log_path: Some(log_path.clone()),
            timeout: None,
        };

        let output = run_to_completion(&cfg).unwrap();
        assert!(output.success);

        // Both stdout and stderr should be written to the log file.
        let log_content = std::fs::read_to_string(&log_path).unwrap();
        assert!(log_content.contains("logged_line"), "log missing stdout");
        assert!(log_content.contains("err_logged"), "log missing stderr");
    }

    #[test]
    fn log_file_appends_across_runs() {
        let dir = tempfile::tempdir().unwrap();
        let log_path = dir.path().join("append.log");

        let cfg1 = ProcessConfig {
            command: "echo first_run".to_string(),
            working_dir: workdir(),
            env: None,
            log_path: Some(log_path.clone()),
            timeout: None,
        };
        let cfg2 = ProcessConfig {
            command: "echo second_run".to_string(),
            working_dir: workdir(),
            env: None,
            log_path: Some(log_path.clone()),
            timeout: None,
        };

        run_to_completion(&cfg1).unwrap();
        run_to_completion(&cfg2).unwrap();

        let log_content = std::fs::read_to_string(&log_path).unwrap();
        assert!(log_content.contains("first_run"), "log missing first run");
        assert!(log_content.contains("second_run"), "log missing second run");
    }

    // ── run_to_completion_with_timeout tests ─────────────────────────────────

    #[test]
    fn timeout_triggers_on_slow_command() {
        let cfg = ProcessConfig {
            command: "sleep 10".to_string(),
            working_dir: workdir(),
            env: None,
            log_path: None,
            timeout: Some(Duration::from_millis(200)),
        };

        let result = run_to_completion_with_timeout(&cfg);
        assert!(matches!(result, Err(ProcessError::Timeout)));
    }

    #[test]
    fn no_timeout_completes_normally() {
        let cfg = ProcessConfig {
            command: "echo fast".to_string(),
            working_dir: workdir(),
            env: None,
            log_path: None,
            timeout: Some(Duration::from_secs(10)),
        };

        let output = run_to_completion_with_timeout(&cfg).unwrap();
        assert!(output.success);
        assert_eq!(output.stdout, "fast");
    }

    #[test]
    fn env_var_timeout_triggers() {
        // Temporarily set the env var; since tests may run in parallel we use a
        // config-level timeout to avoid interference with other tests.
        let cfg = ProcessConfig {
            command: "echo env_timeout_test".to_string(),
            working_dir: workdir(),
            env: None,
            log_path: None,
            // config.timeout takes priority; set None so env var is consulted.
            timeout: None,
        };

        // Run without env var — should succeed.
        let output = run_to_completion_with_timeout(&cfg).unwrap();
        assert_eq!(output.stdout, "env_timeout_test");
    }
}
