//! Error types for Graft.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum GraftError {
    #[error("validation error: {0}")]
    Validation(String),

    #[error("config file not found: {path}")]
    ConfigFileNotFound { path: String },

    #[error("config parse error in {path}: {reason}")]
    ConfigParse { path: String, reason: String },

    #[error("config validation error in {path}, field '{field}': {reason}")]
    ConfigValidation {
        path: String,
        field: String,
        reason: String,
    },

    #[error("invalid git URL: {0}")]
    InvalidGitUrl(String),

    #[error("invalid git ref: {0}")]
    InvalidGitRef(String),

    #[error("invalid command name: {0}")]
    InvalidCommandName(String),

    #[error("invalid dependency name: {0}")]
    InvalidDependencyName(String),

    #[error("invalid commit hash: {0}")]
    InvalidCommitHash(String),

    #[error("invalid timestamp: {0}")]
    InvalidTimestamp(String),

    #[error("invalid lock entry: {0}")]
    InvalidLockEntry(String),

    #[error("unsupported API version: {0}")]
    UnsupportedApiVersion(String),

    #[error("lock file not found: {path}")]
    LockFileNotFound { path: String },

    #[error("lock file parse error in {path}: {reason}")]
    LockFileParse { path: String, reason: String },

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("yaml error: {0}")]
    Yaml(String),
}

pub type Result<T> = std::result::Result<T, GraftError>;
