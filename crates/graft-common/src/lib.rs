//! Shared infrastructure for graft and grove.
//!
//! This crate contains common functionality used by both the graft and grove tools,
//! including command execution with timeouts, git primitives, state caching, and
//! graft.yaml parsing utilities.

pub mod command;
pub mod config;
pub mod git;
pub mod process;
pub mod runs;
pub mod runtime;
pub mod state;

use chrono::{DateTime, Utc};

/// Format an RFC 3339 timestamp as a human-readable "time ago" string.
///
/// Returns `"just now"`, `"{n}m ago"`, `"{n}h ago"`, or `"{n}d ago"`.
/// Returns `"unknown"` when the timestamp cannot be parsed.
pub fn format_time_ago(rfc3339: &str) -> String {
    match DateTime::parse_from_rfc3339(rfc3339) {
        Ok(parsed) => {
            let ts = parsed.with_timezone(&Utc);
            let duration = Utc::now().signed_duration_since(ts);

            if duration.num_seconds() < 60 {
                "just now".to_string()
            } else if duration.num_minutes() < 60 {
                let mins = duration.num_minutes();
                format!("{mins}m ago")
            } else if duration.num_hours() < 24 {
                let hours = duration.num_hours();
                format!("{hours}h ago")
            } else {
                let days = duration.num_days();
                format!("{days}d ago")
            }
        }
        Err(_) => "unknown".to_string(),
    }
}

/// Extract the repo name (last path component) from a path string.
///
/// Returns `"unknown"` when the path has no final component.
pub fn repo_name_from_path(path: &str) -> &str {
    std::path::Path::new(path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
}

pub use command::{run_command_with_timeout, CommandError};
pub use config::{
    parse_commands, parse_commands_from_str, parse_dependency_names,
    parse_dependency_names_from_str, parse_sequences_from_str, parse_state_queries, ArgDef,
    ArgType, CommandDef, EntityDef, OnStepFail, SequenceDef, StateQueryDef, StdinDef, StepDef,
    WhenCondition,
};
pub use git::{
    get_current_commit, git_ahead_behind, git_branch_delete, git_checkout, git_delete_ref,
    git_diff_output, git_diff_stat, git_fast_forward, git_fetch, git_has_tracked_changes,
    git_is_dirty, git_last_commit_time, git_log_output, git_merge_to_ref, git_reset_hard,
    git_rev_parse, git_worktree_add, git_worktree_list, git_worktree_remove, is_git_repo, GitError,
    WorktreeInfo,
};
pub use process::{
    run_to_completion, run_to_completion_registered, run_to_completion_with_timeout,
    run_to_completion_with_timeout_registered, FsProcessRegistry, ProcessConfig, ProcessEntry,
    ProcessError, ProcessEvent, ProcessHandle, ProcessOutput, ProcessRegistry, ProcessStatus,
};
pub use runs::{
    get_run_log_dir, list_runs, read_run_log, run_file_stem, run_log_path, write_run_meta, RunMeta,
};
pub use runtime::{RuntimeError, SessionRuntime, TmuxRuntime};
pub use state::{
    compute_input_cache_key, compute_workspace_hash, get_cache_path, get_query_cache_dir,
    invalidate_cached_state, read_all_cached_for_query, read_cached_state, read_latest_cached,
    write_cached_state, StateMetadata, StateResult,
};
