//! Graft core: domain types, traits, and errors.
//!
//! This crate defines the foundational types for the graft semantic
//! dependency manager. It contains no business logic — only type
//! definitions, validation, error enums, and trait ports.

pub mod domain;
pub mod error;

// Re-export commonly used types
pub use domain::{
    Change, Command, DependencySpec, GitRef, GitUrl, GraftConfig, Metadata, StateCache, StateQuery,
};
pub use error::{GraftError, Result};
