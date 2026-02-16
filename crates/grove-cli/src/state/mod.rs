//! State query integration for Grove.
//!
//! This module provides functionality to discover, read, and display
//! state query results from graft repositories.
pub mod discovery;
pub mod query;

pub use discovery::discover_state_queries;
pub use query::{format_state_summary, StateQuery};

// Re-export shared types and functions from graft-common.
// Some re-exports are only consumed by integration tests.
#[allow(unused_imports)]
pub use graft_common::state::{
    compute_workspace_hash, read_all_cached_for_query, read_cached_state, read_latest_cached,
    StateMetadata, StateResult,
};
