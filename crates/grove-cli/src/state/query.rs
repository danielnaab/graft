//! State query data structures.
use serde::{Deserialize, Serialize};

/// A state query definition from graft.yaml.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateQuery {
    pub name: String,
    pub run: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub deterministic: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<u64>,
}

// Re-export shared types from graft-common (StateMetadata used in tests)
#[allow(unused_imports)]
pub use graft_common::state::{StateMetadata, StateResult};
