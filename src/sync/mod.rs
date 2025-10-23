//! CalDAV synchronization engine
//!
//! This module handles:
//! - CalDAV protocol implementation (PROPFIND, GET requests)
//! - Calendar discovery via well-known URLs
//! - Event downloading and parsing
//! - Authentication (Basic Auth for Phase 1)
//! - Orchestrating the complete sync flow

mod caldav;
mod engine;

pub use caldav::{CalDavClient, CalendarInfo, CalDavEvent};
pub use engine::{SyncEngine, SyncResult};

// #[cfg(test)]
// mod tests;
