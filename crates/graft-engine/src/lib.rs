//! Graft engine: domain types, business logic, adapters, and services.
//!
//! This crate implements the domain types, service layer, config parsing,
//! dependency resolution, lock management, upgrades, and adapter
//! implementations for graft.

pub mod domain;
pub mod error;

pub mod command;
pub mod config;
pub mod lock;
pub mod management;
pub mod mutation;
pub mod query;
pub mod resolution;
pub mod snapshot;
pub mod state;
pub mod validation;

// Re-export domain types and errors at crate root
pub use domain::*;
pub use error::{GraftError, Result};

// Re-export commonly used functions
pub use command::{execute_command, execute_command_by_name, CommandResult};
pub use config::parse_graft_yaml;
pub use lock::{parse_lock_file, write_lock_file};
pub use management::{
    add_dependency_to_config, is_submodule, remove_dependency_from_config,
    remove_dependency_from_lock, remove_submodule, AddResult, RemoveResult,
};
pub use mutation::{apply_lock, upgrade_dependency, ApplyResult, UpgradeResult};
pub use query::{
    filter_breaking_changes, filter_changes_by_type, get_all_status, get_change_by_ref,
    get_change_details, get_changes_for_dependency, get_dependency_status, ChangeDetails,
    DependencyStatus,
};
pub use resolution::{
    fetch_all_dependencies, fetch_dependency, resolve_all_dependencies, resolve_and_create_lock,
    resolve_dependency, sync_all_dependencies, sync_dependency, FetchResult, ResolutionResult,
    SyncResult,
};
pub use snapshot::SnapshotManager;
pub use state::{
    execute_state_query, get_cache_path, get_state, invalidate_cached_state, list_state_queries,
    read_cached_state, write_cached_state, StateMetadata, StateQueryStatus, StateResult,
};
pub use validation::{
    validate_config_schema, validate_integrity, IntegrityResult, ValidationError,
};
