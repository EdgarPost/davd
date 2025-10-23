//! CalDAV synchronization engine
//!
//! This module handles:
//! - CalDAV protocol implementation (PROPFIND, GET requests)
//! - Calendar discovery via well-known URLs
//! - Event downloading and parsing
//! - Authentication (Basic Auth for Phase 1)

mod caldav;

pub use caldav::CalDavClient;

#[cfg(test)]
mod tests;
