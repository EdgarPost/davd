//! Database storage layer for calendar events and sync metadata
//!
//! This module provides SQLite-backed storage for:
//! - Calendar events (with iCalendar data)
//! - Sync metadata (ETags, sync tokens)
//! - Accounts and calendar collections

mod db;
mod models;
mod migrations;

pub use db::Database;
pub use models::{Account, Calendar, Event};

#[cfg(test)]
mod tests;
