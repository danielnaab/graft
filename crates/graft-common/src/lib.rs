//! Shared infrastructure for graft and grove.
//!
//! This crate contains common functionality used by both the graft and grove tools,
//! including command execution with timeouts, git primitives, and state caching.

pub mod command;
pub mod git;

pub use command::{run_command_with_timeout, CommandError};
pub use git::{get_current_commit, git_checkout, git_fetch, git_rev_parse, is_git_repo, GitError};
