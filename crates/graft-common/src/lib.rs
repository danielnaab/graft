//! Shared infrastructure for graft and grove.
//!
//! This crate contains common functionality used by both the graft and grove tools,
//! including command execution with timeouts, git primitives, and state caching.

pub mod command;

pub use command::{run_command_with_timeout, CommandError};
