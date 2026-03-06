//! Graft engine: domain types, business logic, adapters, and services.
//!
//! This crate implements the domain types, service layer, config parsing,
//! dependency resolution, lock management, upgrades, and adapter
//! implementations for graft.

pub mod dependency_graph;
pub mod domain;
pub mod error;

pub mod command;
pub mod config;
pub mod lock;
pub mod management;
pub mod mutation;
pub mod query;
pub mod resolution;
pub mod scion;
pub mod sequence;
pub mod snapshot;
pub mod state;
pub mod template;
pub mod validation;

// Re-export domain types and errors at crate root
pub use dependency_graph::DependencyGraph;
pub use domain::*;
pub use error::{GraftError, Result};

// Re-export commonly used functions
pub use command::{
    capture_written_state, execute_command, execute_command_by_name, execute_command_with_context,
    has_placeholders, resolve_command_stdin, resolve_script_in_command, setup_run_state,
    substitute_named_placeholders, substitute_placeholders, CommandContext, CommandResult,
};
pub use config::{load_dep_configs, parse_graft_yaml};
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
pub use scion::{
    branch_name, classify_verify_value, execute_hook_chain, resolve_base_branch,
    resolve_hook_chain, scion_attach_check, scion_create, scion_fuse, scion_list, scion_prune,
    scion_session_id, scion_start, scion_stop, worktree_path, HookChainError, HookEvent,
    ResolvedHook, ScionEnv, ScionInfo, VerifyLevel,
};
pub use sequence::{execute_sequence, write_sequence_state};
pub use snapshot::SnapshotManager;
pub use state::{
    execute_state_query, get_cache_path, get_run_state_entry, get_state, invalidate_cached_state,
    list_state_queries, read_cached_state, write_cached_state, StateMetadata, StateQueryStatus,
    StateResult,
};
pub use template::{render_template, resolve_stdin, TemplateContext};
pub use validation::{
    validate_config_schema, validate_integrity, IntegrityResult, ValidationError,
};
