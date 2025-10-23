//! Command-line interface for davd
//!
//! Provides commands for:
//! - Triggering manual sync
//! - Querying events
//! - Managing daemon lifecycle

mod commands;

pub use commands::{Cli, Commands};
