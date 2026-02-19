//! Shared infrastructure for graft and grove.
//!
//! This crate contains common functionality used by both the graft and grove tools,
//! including command execution with timeouts, git primitives, state caching, and
//! graft.yaml parsing utilities.

pub mod command;
pub mod config;
pub mod git;
pub mod process;
pub mod state;

pub use command::{run_command_with_timeout, CommandError};
pub use config::{parse_commands, parse_state_queries, CommandDef, StateQueryDef};
pub use git::{get_current_commit, git_checkout, git_fetch, git_rev_parse, is_git_repo, GitError};
pub use process::{ProcessConfig, ProcessError, ProcessEvent, ProcessHandle};
pub use state::{
    compute_workspace_hash, get_cache_path, get_query_cache_dir, invalidate_cached_state,
    read_all_cached_for_query, read_cached_state, read_latest_cached, write_cached_state,
    StateMetadata, StateResult,
};
