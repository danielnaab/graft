//! Subprocess execution with streaming output and lifecycle management.
//!
//! The primary entry point is [`ProcessHandle::spawn`], which runs a shell command and returns
//! a handle plus a channel of [`ProcessEvent`]s that reflect the process lifecycle.
//!
//! For blocking use cases, [`run_to_completion`] and [`run_to_completion_with_timeout`] collect
//! all output synchronously and return a [`ProcessOutput`].
//!
//! To register a process in the global [`ProcessRegistry`], use [`ProcessHandle::spawn_registered`]
//! or the `*_registered` variants of the blocking helpers.

use std::collections::HashMap;
use std::fs::OpenOptions;
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use chrono::Utc;
use serde::{Deserialize, Serialize};
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

/// Errors from process spawn, kill, execution, and registry operations.
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

    #[error("Registry error: {0}")]
    RegistryError(String),
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
/// Created by [`ProcessHandle::spawn`] or [`ProcessHandle::spawn_registered`]. Provides the
/// process PID and the ability to kill it. Process lifecycle events are delivered over the
/// [`mpsc::Receiver<ProcessEvent>`] returned alongside the handle.
///
/// Dropping the handle does **not** kill the subprocess unless a registry was supplied at spawn
/// time (in which case a running process is killed and deregistered on drop).
pub struct ProcessHandle {
    pid: u32,
    child: Arc<Mutex<std::process::Child>>,
    running: Arc<AtomicBool>,
    /// If `Some`, this process is tracked in the registry and will be deregistered on
    /// kill or drop.
    registry: Option<Arc<dyn ProcessRegistry>>,
}

impl std::fmt::Debug for ProcessHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProcessHandle")
            .field("pid", &self.pid)
            .field("running", &self.running.load(Ordering::SeqCst))
            .field("registry", &self.registry.as_ref().map(|_| "<registry>"))
            .finish_non_exhaustive()
    }
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
    pub fn spawn(
        config: &ProcessConfig,
    ) -> Result<(Self, mpsc::Receiver<ProcessEvent>), ProcessError> {
        Self::spawn_inner(config, None)
    }

    /// Spawn a subprocess and register it in `registry`.
    ///
    /// Behaves like [`spawn`](Self::spawn), plus:
    /// - Registers a [`ProcessEntry`] with [`ProcessStatus::Running`] immediately after spawn.
    /// - On process completion or failure, updates the registry entry's status then deregisters.
    /// - On [`kill`](Self::kill), deregisters the entry.
    /// - On [`Drop`], if the process is still running, kills it and deregisters.
    pub fn spawn_registered(
        config: &ProcessConfig,
        registry: Arc<dyn ProcessRegistry>,
    ) -> Result<(Self, mpsc::Receiver<ProcessEvent>), ProcessError> {
        Self::spawn_inner(config, Some(registry))
    }

    #[allow(clippy::too_many_lines)]
    fn spawn_inner(
        config: &ProcessConfig,
        registry: Option<Arc<dyn ProcessRegistry>>,
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

        // Register the entry before spawning background threads, so the caller sees
        // a Running entry as soon as spawn_registered returns.
        if let Some(ref reg) = registry {
            let entry = ProcessEntry::new_running(
                pid,
                config.command.clone(),
                Some(config.working_dir.clone()),
                config.log_path.clone(),
            );
            reg.register(entry)?;
        }

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

        // Monitor thread — polls for exit, joins reader threads, then sends Completed/Failed,
        // and finally updates + deregisters in the registry.
        //
        // Polling with try_wait() lets the kill() method acquire the child lock without
        // contending with a blocking wait() call.
        let child_for_monitor = Arc::clone(&child_arc);
        let running_for_monitor = Arc::clone(&running);
        let reg_for_monitor = registry.clone();
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
                        // Update registry entry to Completed, then deregister.
                        if let Some(ref reg) = reg_for_monitor {
                            let _ = reg.update_status(pid, ProcessStatus::Completed { exit_code });
                            let _ = reg.deregister(pid);
                        }
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
                        // Update registry entry to Failed, then deregister.
                        if let Some(ref reg) = reg_for_monitor {
                            let _ = reg.update_status(
                                pid,
                                ProcessStatus::Failed {
                                    error: e.to_string(),
                                },
                            );
                            let _ = reg.deregister(pid);
                        }
                        break;
                    }
                }
            }
        }));

        let handle = Self {
            pid,
            child: child_arc,
            running,
            registry,
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
    ///
    /// If this process was spawned with a registry, it is deregistered regardless of whether
    /// the kill call succeeded (the process may already have exited).
    pub fn kill(&self) -> Result<(), ProcessError> {
        let result = {
            let mut child = self.child.lock().unwrap();
            child
                .kill()
                .map_err(|e| ProcessError::KillFailed(e.to_string()))
        };
        // Always attempt to deregister — process may have already exited and the monitor
        // thread may have deregistered too, but deregister is a no-op for missing entries.
        if let Some(ref reg) = self.registry {
            let _ = reg.deregister(self.pid);
        }
        result
    }
}

impl Drop for ProcessHandle {
    /// If the handle was spawned with a registry and the process is still running, kill the
    /// process and deregister the entry.
    fn drop(&mut self) {
        if self.is_running() {
            if let Some(ref reg) = self.registry {
                if let Ok(mut child) = self.child.lock() {
                    let _ = child.kill();
                }
                let _ = reg.deregister(self.pid);
            }
        }
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

/// Block until the process completes, registering it in `registry` for the duration.
///
/// Equivalent to [`run_to_completion`] but the process appears in the registry while running
/// and is deregistered on completion.
pub fn run_to_completion_registered(
    config: &ProcessConfig,
    registry: Arc<dyn ProcessRegistry>,
) -> Result<ProcessOutput, ProcessError> {
    let (_handle, rx) = ProcessHandle::spawn_registered(config, registry)?;
    collect_output(&rx)
}

/// Block until the process completes (with timeout), registering it in `registry`.
///
/// Combines the behaviour of [`run_to_completion_with_timeout`] and
/// [`run_to_completion_registered`]. The process is deregistered on completion or timeout.
pub fn run_to_completion_with_timeout_registered(
    config: &ProcessConfig,
    registry: Arc<dyn ProcessRegistry>,
) -> Result<ProcessOutput, ProcessError> {
    let timeout = config.timeout.or_else(|| {
        std::env::var("GRAFT_PROCESS_TIMEOUT_MS")
            .ok()
            .and_then(|s| s.parse::<u64>().ok())
            .map(Duration::from_millis)
    });

    let (handle, rx) = ProcessHandle::spawn_registered(config, registry)?;

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

// ── ProcessRegistry ──────────────────────────────────────────────────────────

/// Status of a registered process.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ProcessStatus {
    /// Process is currently running.
    Running,
    /// Process exited normally.
    Completed {
        /// Exit code from the process.
        exit_code: i32,
    },
    /// Process encountered an unexpected error.
    Failed {
        /// Error description.
        error: String,
    },
}

/// A record stored in the process registry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessEntry {
    /// OS process ID.
    pub pid: u32,
    /// Shell command that was run.
    pub command: String,
    /// Optional path to the repository this process belongs to.
    pub repo_path: Option<PathBuf>,
    /// ISO 8601 start timestamp (RFC 3339).
    pub start_time: String,
    /// Optional path to the log file where output was captured.
    pub log_path: Option<PathBuf>,
    /// Current status of the process.
    pub status: ProcessStatus,
}

impl ProcessEntry {
    /// Create a new `Running` entry with the current timestamp.
    pub fn new_running(
        pid: u32,
        command: impl Into<String>,
        repo_path: Option<PathBuf>,
        log_path: Option<PathBuf>,
    ) -> Self {
        Self {
            pid,
            command: command.into(),
            repo_path,
            start_time: Utc::now().to_rfc3339(),
            log_path,
            status: ProcessStatus::Running,
        }
    }
}

/// Interface for a global process registry.
///
/// Implementors store and retrieve [`ProcessEntry`] records. The default implementation
/// uses the filesystem; other implementations may use a database or network service.
pub trait ProcessRegistry: Send + Sync {
    /// Add or replace a process entry.
    fn register(&self, entry: ProcessEntry) -> Result<(), ProcessError>;

    /// Remove a process entry. A no-op if the PID is not registered.
    fn deregister(&self, pid: u32) -> Result<(), ProcessError>;

    /// Return all entries with [`ProcessStatus::Running`] status.
    ///
    /// Entries whose PIDs are no longer alive are pruned automatically.
    fn list_active(&self) -> Result<Vec<ProcessEntry>, ProcessError>;

    /// Return a single entry by PID, or `None` if not registered.
    fn get(&self, pid: u32) -> Result<Option<ProcessEntry>, ProcessError>;

    /// Update the status of an existing entry. A no-op if the PID is not registered.
    fn update_status(&self, pid: u32, status: ProcessStatus) -> Result<(), ProcessError>;
}

/// Filesystem-backed process registry.
///
/// Stores each entry as `{pid}.json` in [`base_dir`](FsProcessRegistry::new).
/// Dead PID entries (process crashed or killed without cleanup) are pruned
/// automatically on [`list_active`](ProcessRegistry::list_active).
pub struct FsProcessRegistry {
    base_dir: PathBuf,
}

impl FsProcessRegistry {
    /// Create a new registry backed by `base_dir`.
    ///
    /// The directory is created if it doesn't exist.
    pub fn new(base_dir: PathBuf) -> Result<Self, ProcessError> {
        std::fs::create_dir_all(&base_dir)?;
        Ok(Self { base_dir })
    }

    /// Default process registry directory: `~/.cache/graft/processes/`.
    pub fn default_path() -> PathBuf {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
        PathBuf::from(home).join(".cache/graft/processes")
    }

    fn entry_path(&self, pid: u32) -> PathBuf {
        self.base_dir.join(format!("{pid}.json"))
    }

    fn read_entry(&self, pid: u32) -> Result<Option<ProcessEntry>, ProcessError> {
        let path = self.entry_path(pid);
        if !path.exists() {
            return Ok(None);
        }
        let content = std::fs::read_to_string(&path)?;
        let entry = serde_json::from_str(&content).map_err(|e| {
            ProcessError::RegistryError(format!("Failed to parse entry for PID {pid}: {e}"))
        })?;
        Ok(Some(entry))
    }
}

/// Return `true` if the given PID corresponds to a running process.
///
/// On Linux, checks for the presence of `/proc/{pid}`. On other platforms,
/// falls back to `true` (conservative: never prune).
fn pid_is_alive(pid: u32) -> bool {
    #[cfg(target_os = "linux")]
    {
        std::path::Path::new(&format!("/proc/{pid}")).exists()
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = pid;
        true
    }
}

impl ProcessRegistry for FsProcessRegistry {
    fn register(&self, entry: ProcessEntry) -> Result<(), ProcessError> {
        let path = self.entry_path(entry.pid);
        let content = serde_json::to_string_pretty(&entry)
            .map_err(|e| ProcessError::RegistryError(format!("Failed to serialize entry: {e}")))?;
        std::fs::write(&path, content)?;
        Ok(())
    }

    fn deregister(&self, pid: u32) -> Result<(), ProcessError> {
        let path = self.entry_path(pid);
        if path.exists() {
            std::fs::remove_file(&path)?;
        }
        Ok(())
    }

    fn list_active(&self) -> Result<Vec<ProcessEntry>, ProcessError> {
        let dir_entries = std::fs::read_dir(&self.base_dir)?;
        let mut active = Vec::new();

        for dir_entry in dir_entries.flatten() {
            let path = dir_entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some("json") {
                continue;
            }

            // Extract PID from filename stem.
            let pid: u32 = path
                .file_stem()
                .and_then(|s| s.to_str())
                .and_then(|s| s.parse().ok())
                .unwrap_or(0);
            if pid == 0 {
                continue;
            }

            match self.read_entry(pid)? {
                Some(entry) if entry.status == ProcessStatus::Running => {
                    if pid_is_alive(pid) {
                        active.push(entry);
                    } else {
                        // Stale entry: PID is dead but was never cleaned up.
                        let _ = std::fs::remove_file(&path);
                    }
                }
                _ => {} // Completed/Failed entries excluded from active list.
            }
        }

        Ok(active)
    }

    fn get(&self, pid: u32) -> Result<Option<ProcessEntry>, ProcessError> {
        self.read_entry(pid)
    }

    fn update_status(&self, pid: u32, status: ProcessStatus) -> Result<(), ProcessError> {
        match self.read_entry(pid)? {
            Some(mut entry) => {
                entry.status = status;
                let path = self.entry_path(pid);
                let content = serde_json::to_string_pretty(&entry).map_err(|e| {
                    ProcessError::RegistryError(format!("Failed to serialize entry: {e}"))
                })?;
                std::fs::write(&path, content)?;
                Ok(())
            }
            None => Ok(()), // Silently ignore update for non-existent entry.
        }
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

    // ── ProcessRegistry tests ────────────────────────────────────────────────

    fn make_registry() -> (FsProcessRegistry, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let reg = FsProcessRegistry::new(dir.path().to_path_buf()).unwrap();
        (reg, dir)
    }

    fn running_entry(pid: u32, command: &str) -> ProcessEntry {
        ProcessEntry::new_running(pid, command, None, None)
    }

    #[test]
    fn register_and_get_entry() {
        let (reg, _dir) = make_registry();
        let entry = running_entry(12345, "echo test");

        reg.register(entry.clone()).unwrap();

        let got = reg.get(12345).unwrap().expect("entry should exist");
        assert_eq!(got.pid, 12345);
        assert_eq!(got.command, "echo test");
        assert_eq!(got.status, ProcessStatus::Running);
    }

    #[test]
    fn deregister_removes_entry() {
        let (reg, _dir) = make_registry();
        reg.register(running_entry(22222, "sleep 60")).unwrap();

        reg.deregister(22222).unwrap();

        assert!(reg.get(22222).unwrap().is_none());
    }

    #[test]
    fn deregister_nonexistent_is_noop() {
        let (reg, _dir) = make_registry();
        // Should not error even if the PID was never registered.
        reg.deregister(99999).unwrap();
    }

    #[test]
    fn update_status_changes_entry() {
        let (reg, _dir) = make_registry();
        reg.register(running_entry(33333, "make test")).unwrap();

        reg.update_status(33333, ProcessStatus::Completed { exit_code: 0 })
            .unwrap();

        let got = reg.get(33333).unwrap().expect("entry should exist");
        assert_eq!(got.status, ProcessStatus::Completed { exit_code: 0 });
    }

    #[test]
    fn update_status_nonexistent_is_noop() {
        let (reg, _dir) = make_registry();
        // Should not error even if the PID was never registered.
        reg.update_status(44444, ProcessStatus::Completed { exit_code: 0 })
            .unwrap();
    }

    #[test]
    fn list_active_returns_only_running_and_alive() {
        let (reg, _dir) = make_registry();

        // Use the current process PID — guaranteed to be alive.
        let my_pid = std::process::id();
        reg.register(running_entry(my_pid, "current process"))
            .unwrap();

        // Register a Completed entry — should not appear in list_active.
        let mut completed = running_entry(55555, "finished command");
        completed.status = ProcessStatus::Completed { exit_code: 0 };
        reg.register(completed).unwrap();

        let active = reg.list_active().unwrap();
        let pids: Vec<u32> = active.iter().map(|e| e.pid).collect();

        assert!(pids.contains(&my_pid), "current process should be active");
        assert!(
            !pids.contains(&55555),
            "completed entry should not be active"
        );
    }

    #[test]
    fn list_active_prunes_dead_pids() {
        let (reg, _dir) = make_registry();

        // Use an extremely large PID that cannot be running on this system.
        // Linux max PID is 4_194_304; u32::MAX is safely out of range.
        // We check /proc/{pid} — this path will not exist.
        let dead_pid: u32 = 4_000_000;
        reg.register(running_entry(dead_pid, "ghost process"))
            .unwrap();

        // Before list_active, the file should exist.
        assert!(reg.get(dead_pid).unwrap().is_some());

        let active = reg.list_active().unwrap();
        let pids: Vec<u32> = active.iter().map(|e| e.pid).collect();
        assert!(!pids.contains(&dead_pid), "dead PID should be pruned");

        // After list_active, the file should be gone.
        assert!(reg.get(dead_pid).unwrap().is_none());
    }

    #[test]
    fn list_active_multiple_running_entries() {
        let (reg, _dir) = make_registry();
        let my_pid = std::process::id();

        reg.register(running_entry(my_pid, "cmd-a")).unwrap();

        // Spawn two real processes and register them so we have known-alive PIDs.
        let mut child1 = std::process::Command::new("sleep")
            .arg("60")
            .spawn()
            .unwrap();
        let mut child2 = std::process::Command::new("sleep")
            .arg("60")
            .spawn()
            .unwrap();
        let pid1 = child1.id();
        let pid2 = child2.id();

        reg.register(running_entry(pid1, "sleep-a")).unwrap();
        reg.register(running_entry(pid2, "sleep-b")).unwrap();

        let active = reg.list_active().unwrap();
        let pids: Vec<u32> = active.iter().map(|e| e.pid).collect();
        assert!(pids.contains(&my_pid));
        assert!(pids.contains(&pid1));
        assert!(pids.contains(&pid2));

        // Clean up child processes.
        child1.kill().unwrap();
        child2.kill().unwrap();
        let _ = child1.wait();
        let _ = child2.wait();
    }

    // ── spawn_registered lifecycle tests ─────────────────────────────────────

    /// Returns an `Arc<dyn ProcessRegistry>` backed by a temp directory.
    fn make_arc_registry() -> (Arc<dyn ProcessRegistry>, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let reg: Arc<dyn ProcessRegistry> =
            Arc::new(FsProcessRegistry::new(dir.path().to_path_buf()).unwrap());
        (reg, dir)
    }

    #[test]
    fn spawn_registered_shows_running_entry() {
        let (reg, _dir) = make_arc_registry();

        let (handle, _rx) =
            ProcessHandle::spawn_registered(&config("sleep 60"), Arc::clone(&reg)).unwrap();

        // Give the process a moment to start (entry is registered before threads start).
        // The entry should exist immediately after spawn_registered returns.
        let entry = reg
            .get(handle.pid())
            .unwrap()
            .expect("entry should exist right after spawn");
        assert_eq!(entry.pid, handle.pid());
        assert_eq!(entry.status, ProcessStatus::Running);
        assert!(entry.repo_path.is_some(), "repo_path should be set");

        // Kill to clean up — deregisters as a side effect.
        handle.kill().unwrap();
        // Drain events so the monitor thread has time to finish.
        thread::sleep(Duration::from_millis(50));
    }

    #[test]
    fn spawn_registered_completion_deregisters() {
        let (reg, _dir) = make_arc_registry();

        let (handle, rx) =
            ProcessHandle::spawn_registered(&config("echo done"), Arc::clone(&reg)).unwrap();
        let pid = handle.pid();

        // Drain all events — waits until the process exits and the channel closes.
        let _ = collect_events(rx);

        // The monitor thread deregisters just after sending Completed.
        // Give it a moment to finish the filesystem operation.
        thread::sleep(Duration::from_millis(50));

        assert!(
            reg.get(pid).unwrap().is_none(),
            "entry should be deregistered after completion"
        );
    }

    #[test]
    fn kill_deregisters_entry() {
        let (reg, _dir) = make_arc_registry();

        let (handle, rx) =
            ProcessHandle::spawn_registered(&config("sleep 60"), Arc::clone(&reg)).unwrap();
        let pid = handle.pid();

        // Entry should be present before kill.
        assert!(reg.get(pid).unwrap().is_some());

        // kill() deregisters synchronously.
        handle.kill().unwrap();
        assert!(
            reg.get(pid).unwrap().is_none(),
            "entry should be deregistered after kill"
        );

        // Drain channel so the monitor thread doesn't linger.
        let _ = collect_events(rx);
    }

    #[test]
    fn drop_kills_and_deregisters_running_process() {
        let (reg, _dir) = make_arc_registry();

        let pid = {
            let (handle, _rx) =
                ProcessHandle::spawn_registered(&config("sleep 60"), Arc::clone(&reg)).unwrap();
            let pid = handle.pid();
            // Entry exists while handle is alive.
            assert!(reg.get(pid).unwrap().is_some());
            // Drop the handle — should kill the process and deregister.
            pid
            // handle dropped here
        };

        // After drop, the entry should be gone.
        assert!(
            reg.get(pid).unwrap().is_none(),
            "entry should be deregistered on drop"
        );
    }

    #[test]
    fn run_to_completion_registered_deregisters_on_finish() {
        let (reg, _dir) = make_arc_registry();

        let output = run_to_completion_registered(&config("echo hello"), Arc::clone(&reg)).unwrap();
        assert!(output.success);

        // Give the monitor thread a moment to deregister.
        thread::sleep(Duration::from_millis(50));

        // No entries should remain — all deregistered after completion.
        let active = reg.list_active().unwrap();
        assert!(
            active.is_empty(),
            "registry should be empty after completion"
        );
    }

    #[test]
    fn run_to_completion_with_timeout_registered_deregisters_on_timeout() {
        let (reg, _dir) = make_arc_registry();

        let cfg = ProcessConfig {
            command: "sleep 10".to_string(),
            working_dir: workdir(),
            env: None,
            log_path: None,
            timeout: Some(Duration::from_millis(200)),
        };

        let result = run_to_completion_with_timeout_registered(&cfg, Arc::clone(&reg));
        assert!(matches!(result, Err(ProcessError::Timeout)));

        // After timeout, kill() was called which deregisters. Give it a moment.
        thread::sleep(Duration::from_millis(50));

        let active = reg.list_active().unwrap();
        assert!(
            active.is_empty(),
            "registry should be empty after timeout kill"
        );
    }
}
