//! State query integration for Grove.
//!
//! This module provides functionality to discover, read, and display
//! state query results from graft repositories.
pub mod cache;
pub mod discovery;
pub mod query;

pub use cache::{compute_workspace_hash, read_latest_cached};
#[allow(unused_imports)]
pub use cache::{read_all_cached_for_query, read_cached_state};
pub use discovery::discover_state_queries;
pub use query::{StateQuery, StateResult};

// Re-export shared types from graft-common for convenience (used in tests)
#[allow(unused_imports)]
pub use graft_common::state::StateMetadata;
