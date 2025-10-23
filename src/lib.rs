//! davd - CalDAV/CardDAV sync daemon for Linux
//!
//! This library provides the core functionality for syncing calendars and contacts
//! via CalDAV/CardDAV protocols and exposing them through a D-Bus API.

pub mod storage;
pub mod sync;
pub mod dbus;
pub mod ical;

// Re-export common types
pub use storage::{Database, Event};

/// Result type used throughout the library
pub type Result<T> = anyhow::Result<T>;
