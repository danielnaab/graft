//! Business logic for Grove.
//!
//! This crate implements:
//! - Configuration loading adapters
//! - Git status querying adapters
//! - Multi-repository registry

pub mod config;
pub mod git;
pub mod registry;

// Re-export public API
pub use config::{GraftYamlConfigLoader, YamlConfigLoader};
pub use git::GitoxideStatus;
pub use registry::WorkspaceRegistry;
