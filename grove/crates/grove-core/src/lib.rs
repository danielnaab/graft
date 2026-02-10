//! Core types, traits, and errors for Grove.
//!
//! This crate defines:
//! - Domain types (newtypes, validated at construction)
//! - Trait definitions (ports for dependency injection)
//! - Error types (structured errors using thiserror)

pub mod domain;
pub mod error;
pub mod traits;

// Re-export commonly used types
pub use domain::{
    RefreshStats, RepoPath, RepoStatus, RepositoryDeclaration, WorkspaceConfig, WorkspaceName,
};
pub use error::{CoreError, Result};
pub use traits::{ConfigLoader, GitStatus, RepoRegistry};
