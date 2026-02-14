///! State query integration for Grove.
///!
///! This module provides functionality to discover, read, and display
///! state query results from graft repositories.

pub mod cache;
pub mod discovery;
pub mod query;

pub use cache::{compute_workspace_hash, read_all_cached_for_query, read_cached_state, read_latest_cached};
pub use discovery::discover_state_queries;
pub use query::{StateMetadata, StateQuery, StateResult};
