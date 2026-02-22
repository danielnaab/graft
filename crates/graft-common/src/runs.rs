//! Command run logging: metadata storage and discovery.
//!
//! Each command run in grove produces two files:
//! - `{timestamp}-{command}.log` — plain-text output (written by `ProcessHandle` via `log_path`)
//! - `{timestamp}-{command}.meta.json` — structured metadata (written after completion)
//!
//! Layout: `~/.cache/graft/{workspace-hash}/{repo}/runs/`

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

use crate::state::compute_workspace_hash;

/// Metadata for a single command run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunMeta {
    /// Command name (e.g. "implement", "plan").
    pub command: String,
    /// Arguments passed to the command.
    pub args: Vec<String>,
    /// Full shell command that was executed.
    pub shell_cmd: String,
    /// ISO 8601 start timestamp.
    pub start_time: String,
    /// ISO 8601 end timestamp (set after completion).
    pub end_time: Option<String>,
    /// Process exit code (set after completion).
    pub exit_code: Option<i32>,
    /// Base name of the log file (e.g. "20260222-153000-implement.log").
    pub log_file: String,
}

impl RunMeta {
    /// Parse `start_time` as `DateTime`.
    pub fn start_time_parsed(&self) -> Option<DateTime<Utc>> {
        DateTime::parse_from_rfc3339(&self.start_time)
            .ok()
            .map(|dt| dt.with_timezone(&Utc))
    }

    /// Human-readable time-ago string for the start time.
    pub fn time_ago(&self) -> String {
        crate::format_time_ago(&self.start_time)
    }

    /// Short display string for the exit status.
    pub fn status_display(&self) -> &str {
        match self.exit_code {
            Some(0) => "ok",
            Some(_) => "failed",
            None => "running",
        }
    }
}

/// Get the runs directory for a repo within a workspace.
///
/// Format: `~/.cache/graft/{workspace-hash}/{repo}/runs/`
pub fn get_run_log_dir(workspace_name: &str, repo_name: &str) -> PathBuf {
    let workspace_hash = compute_workspace_hash(workspace_name);
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());

    PathBuf::from(home)
        .join(".cache/graft")
        .join(workspace_hash)
        .join(repo_name)
        .join("runs")
}

/// Generate a timestamped file stem for a run (e.g. "20260222-153000-implement").
///
/// Accepts an explicit timestamp so the caller can reuse it for `RunMeta.start_time`.
pub fn run_file_stem(command_name: &str, now: DateTime<Utc>) -> String {
    let ts = now.format("%Y%m%d-%H%M%S");
    // Sanitize command name for filesystem
    let safe_name: String = command_name
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '-'
            }
        })
        .collect();
    format!("{ts}-{safe_name}")
}

/// Compute the log file path for a new run.
///
/// Returns `(full_path, log_filename, start_time)` — the `start_time` is the same
/// instant used for the filename, so callers can store it in `RunMeta.start_time`
/// without clock skew.
pub fn run_log_path(
    workspace_name: &str,
    repo_name: &str,
    command_name: &str,
) -> (PathBuf, String, DateTime<Utc>) {
    let now = Utc::now();
    let dir = get_run_log_dir(workspace_name, repo_name);
    let stem = run_file_stem(command_name, now);
    let log_file = format!("{stem}.log");
    (dir.join(&log_file), log_file, now)
}

/// Write run metadata to the sidecar JSON file.
pub fn write_run_meta(
    workspace_name: &str,
    repo_name: &str,
    meta: &RunMeta,
) -> std::io::Result<()> {
    let dir = get_run_log_dir(workspace_name, repo_name);
    fs::create_dir_all(&dir)?;

    // Derive meta path from log_file name
    let meta_file = meta.log_file.replace(".log", ".meta.json");
    let meta_path = dir.join(meta_file);

    let content = serde_json::to_string_pretty(meta).map_err(std::io::Error::other)?;
    fs::write(&meta_path, content)
}

/// List runs for a repo, sorted newest-first, capped at `limit`.
///
/// Pass `0` for unlimited results.
pub fn list_runs(workspace_name: &str, repo_name: &str, limit: usize) -> Vec<RunMeta> {
    let dir = get_run_log_dir(workspace_name, repo_name);
    if !dir.exists() {
        return Vec::new();
    }

    let mut runs: Vec<RunMeta> = Vec::new();

    let Ok(entries) = fs::read_dir(&dir) else {
        return Vec::new();
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("json")
            && path
                .file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.ends_with(".meta.json"))
        {
            if let Ok(content) = fs::read_to_string(&path) {
                if let Ok(meta) = serde_json::from_str::<RunMeta>(&content) {
                    runs.push(meta);
                }
            }
        }
    }

    // Sort newest-first by start_time (lexicographic on ISO 8601 works)
    runs.sort_by(|a, b| b.start_time.cmp(&a.start_time));
    if limit > 0 {
        runs.truncate(limit);
    }
    runs
}

/// Read the log file contents for a run.
pub fn read_run_log(workspace_name: &str, repo_name: &str, log_file: &str) -> Option<String> {
    let dir = get_run_log_dir(workspace_name, repo_name);
    let log_path = dir.join(log_file);
    fs::read_to_string(&log_path).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex, MutexGuard};
    use tempfile::TempDir;

    // Serialize all tests that mutate the process-global HOME env var.
    static HOME_LOCK: Mutex<()> = Mutex::new(());

    fn with_temp_home<F: FnOnce(&str, MutexGuard<'_, ()>)>(f: F) {
        let tmp = TempDir::new().unwrap();
        let lock = HOME_LOCK.lock().unwrap();
        let prev = std::env::var("HOME").ok();
        std::env::set_var("HOME", tmp.path());
        f("test-ws", lock);
        match prev {
            Some(v) => std::env::set_var("HOME", v),
            None => std::env::remove_var("HOME"),
        }
    }

    #[test]
    fn run_log_dir_uses_workspace_hash() {
        let dir = get_run_log_dir("my-workspace", "my-repo");
        let hash = compute_workspace_hash("my-workspace");
        assert!(dir.to_str().unwrap().contains(&hash));
        assert!(dir.to_str().unwrap().contains("my-repo/runs"));
    }

    #[test]
    fn run_file_stem_format() {
        let stem = run_file_stem("implement", Utc::now());
        assert!(stem.ends_with("-implement"));
        assert!(stem.len() > 20);
    }

    #[test]
    fn run_file_stem_sanitizes_colons() {
        let stem = run_file_stem("dep:command", Utc::now());
        assert!(stem.ends_with("-dep-command"));
        assert!(!stem.contains(':'));
    }

    #[test]
    fn write_and_list_runs() {
        with_temp_home(|ws, _lock| {
            let meta = RunMeta {
                command: "test".to_string(),
                args: vec!["arg1".to_string()],
                shell_cmd: "echo hello".to_string(),
                start_time: "2026-02-22T15:30:00Z".to_string(),
                end_time: Some("2026-02-22T15:30:05Z".to_string()),
                exit_code: Some(0),
                log_file: "20260222-153000-test.log".to_string(),
            };

            write_run_meta(ws, "repo", &meta).unwrap();

            let runs = list_runs(ws, "repo", 0);
            assert_eq!(runs.len(), 1);
            assert_eq!(runs[0].command, "test");
            assert_eq!(runs[0].exit_code, Some(0));
        });
    }

    #[test]
    fn list_runs_returns_newest_first() {
        with_temp_home(|ws, _lock| {
            let meta_old = RunMeta {
                command: "old".to_string(),
                args: vec![],
                shell_cmd: "echo old".to_string(),
                start_time: "2026-02-22T10:00:00Z".to_string(),
                end_time: Some("2026-02-22T10:00:05Z".to_string()),
                exit_code: Some(0),
                log_file: "20260222-100000-old.log".to_string(),
            };
            let meta_new = RunMeta {
                command: "new".to_string(),
                args: vec![],
                shell_cmd: "echo new".to_string(),
                start_time: "2026-02-22T15:00:00Z".to_string(),
                end_time: Some("2026-02-22T15:00:05Z".to_string()),
                exit_code: Some(0),
                log_file: "20260222-150000-new.log".to_string(),
            };

            write_run_meta(ws, "repo", &meta_old).unwrap();
            write_run_meta(ws, "repo", &meta_new).unwrap();

            let runs = list_runs(ws, "repo", 0);
            assert_eq!(runs.len(), 2);
            assert_eq!(runs[0].command, "new");
            assert_eq!(runs[1].command, "old");
        });
    }

    #[test]
    fn list_runs_handles_missing_directory() {
        with_temp_home(|ws, _lock| {
            let runs = list_runs(ws, "nonexistent", 0);
            assert!(runs.is_empty());
        });
    }

    #[test]
    fn read_run_log_works() {
        with_temp_home(|ws, _lock| {
            let dir = get_run_log_dir(ws, "repo");
            fs::create_dir_all(&dir).unwrap();
            fs::write(dir.join("test.log"), "line1\nline2\n").unwrap();

            let content = read_run_log(ws, "repo", "test.log");
            assert_eq!(content, Some("line1\nline2\n".to_string()));
        });
    }

    #[test]
    fn read_run_log_returns_none_for_missing() {
        with_temp_home(|ws, _lock| {
            let content = read_run_log(ws, "repo", "nonexistent.log");
            assert!(content.is_none());
        });
    }

    #[test]
    fn run_meta_status_display() {
        let mut meta = RunMeta {
            command: "test".to_string(),
            args: vec![],
            shell_cmd: "echo".to_string(),
            start_time: Utc::now().to_rfc3339(),
            end_time: None,
            exit_code: None,
            log_file: "test.log".to_string(),
        };

        assert_eq!(meta.status_display(), "running");
        meta.exit_code = Some(0);
        assert_eq!(meta.status_display(), "ok");
        meta.exit_code = Some(1);
        assert_eq!(meta.status_display(), "failed");
    }

    #[test]
    fn run_meta_time_ago() {
        let meta = RunMeta {
            command: "test".to_string(),
            args: vec![],
            shell_cmd: "echo".to_string(),
            start_time: Utc::now().to_rfc3339(),
            end_time: None,
            exit_code: None,
            log_file: "test.log".to_string(),
        };
        assert_eq!(meta.time_ago(), "just now");
    }
}
