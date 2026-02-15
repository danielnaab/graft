//! Graft engine: business logic, adapters, and services.
//!
//! This crate implements the service layer for graft, including
//! config parsing, dependency resolution, lock management,
//! upgrades, and adapter implementations.

pub mod config;
pub mod lock;
pub mod query;
pub mod resolution;
pub mod validation;

// Re-export commonly used functions
pub use config::parse_graft_yaml;
pub use lock::{parse_lock_file, write_lock_file};
pub use query::{
    filter_breaking_changes, filter_changes_by_type, get_all_status, get_change_by_ref,
    get_change_details, get_changes_for_dependency, get_dependency_status, ChangeDetails,
    DependencyStatus,
};
pub use resolution::{resolve_all_dependencies, resolve_dependency, ResolutionResult};
pub use validation::{
    validate_config_schema, validate_integrity, IntegrityResult, ValidationError,
};
