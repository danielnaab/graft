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

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("yaml error: {0}")]
    Yaml(String),
}

pub type Result<T> = std::result::Result<T, GraftError>;
