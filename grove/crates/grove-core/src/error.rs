//! Error types for Grove.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum CoreError {
    #[error("workspace name cannot be empty")]
    EmptyWorkspaceName,

    #[error("repository path cannot be empty")]
    EmptyRepoPath,

    #[error("invalid repository path: {path}")]
    InvalidRepoPath { path: String },

    #[error("repository not found: {path}")]
    RepoNotFound { path: String },

    #[error("not a git repository: {path}")]
    NotGitRepo { path: String },

    #[error("invalid workspace configuration: {details}")]
    InvalidConfig { details: String },

    #[error("git operation failed: {details}")]
    GitError { details: String },

    #[error("git operation timed out after {timeout_ms}ms: {operation}")]
    GitTimeout { operation: String, timeout_ms: u64 },

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, CoreError>;
