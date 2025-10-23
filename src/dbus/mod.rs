//! D-Bus interface for calendar access
//!
//! Provides the org.davd.Calendar interface for applications to:
//! - List events within a time range
//! - Get individual event details
//! - Receive notifications when events change (Phase 2)

mod calendar;

pub use calendar::CalendarService;

#[cfg(test)]
mod tests;
