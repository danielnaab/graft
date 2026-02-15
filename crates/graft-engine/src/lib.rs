//! Graft engine: business logic, adapters, and services.
//!
//! This crate implements the service layer for graft, including
//! config parsing, dependency resolution, lock management,
//! upgrades, and adapter implementations.

pub mod config;
pub mod lock;

// Re-export commonly used functions
pub use config::parse_graft_yaml;
pub use lock::{parse_lock_file, write_lock_file};
