//! Graft engine: business logic, adapters, and services.
//!
//! This crate implements the service layer for graft, including
//! config parsing, dependency resolution, lock management,
//! upgrades, and adapter implementations.

pub mod config;
pub mod lock;
pub mod query;

// Re-export commonly used functions
pub use config::parse_graft_yaml;
pub use lock::{parse_lock_file, write_lock_file};
pub use query::{get_all_status, get_dependency_status, DependencyStatus};
