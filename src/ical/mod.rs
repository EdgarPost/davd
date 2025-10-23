//! iCalendar parsing and serialization
//!
//! Handles parsing of iCalendar VEVENT components from CalDAV responses.
//! For Phase 1, we focus on basic event properties:
//! - Summary, Description, Location
//! - Start time, End time
//! - UID (required for CalDAV sync)

mod parser;

pub use parser::{parse_event, ICalEvent};

// #[cfg(test)]
// mod tests;
