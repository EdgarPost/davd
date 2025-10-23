//! iCalendar parser for VEVENT components
//!
//! Parses iCalendar data from CalDAV responses into structured events.

use chrono::{DateTime, Utc};
use crate::Result;

/// Parsed iCalendar event
#[derive(Debug, Clone)]
pub struct ICalEvent {
    /// Event UID (required)
    pub uid: String,
    /// Event summary/title
    pub summary: Option<String>,
    /// Event description
    pub description: Option<String>,
    /// Event location
    pub location: Option<String>,
    /// Start time
    pub start_time: DateTime<Utc>,
    /// End time
    pub end_time: Option<DateTime<Utc>>,
    /// Recurrence rule (not parsed in Phase 1)
    pub rrule: Option<String>,
}

/// Parse an iCalendar VEVENT component
///
/// # Arguments
/// * `ical_data` - The raw iCalendar data (should contain a VEVENT)
///
/// # Returns
/// A parsed ICalEvent with extracted properties
///
/// # Errors
/// Returns an error if:
/// - The data is not valid iCalendar format
/// - Required properties are missing (UID, DTSTART)
/// - Dates cannot be parsed
pub fn parse_event(_ical_data: &str) -> Result<ICalEvent> {
    // TODO: Parse iCalendar data
    // TODO: Extract VEVENT component
    // TODO: Parse required properties (UID, DTSTART)
    // TODO: Parse optional properties (SUMMARY, DESCRIPTION, LOCATION, DTEND)
    // TODO: Handle date formats (DATE, DATE-TIME, with/without timezone)
    // TODO: Return parsed event

    todo!("Implement parse_event")
}

/// Extract a property value from iCalendar data
///
/// # Arguments
/// * `data` - The iCalendar data
/// * `property` - The property name (e.g., "UID", "SUMMARY")
///
/// # Returns
/// The property value if found, None otherwise
fn extract_property(_data: &str, _property: &str) -> Option<String> {
    // TODO: Find property line
    // TODO: Extract value (handle line folding)
    // TODO: Unescape text (\\n, \\,, etc.)
    todo!("Implement extract_property")
}

/// Parse an iCalendar date/time value
///
/// # Arguments
/// * `value` - The date/time string (e.g., "20240101T120000Z")
///
/// # Returns
/// A parsed DateTime<Utc>
fn parse_datetime(_value: &str) -> Result<DateTime<Utc>> {
    // TODO: Parse DATE-TIME format (e.g., "20240101T120000Z")
    // TODO: Parse DATE format (e.g., "20240101")
    // TODO: Handle timezone parameters
    todo!("Implement parse_datetime")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_event() {
        let ical = r#"BEGIN:VCALENDAR
VERSION:2.0
PRODID:-//Test//Test//EN
BEGIN:VEVENT
UID:test-event-123
DTSTART:20240101T120000Z
DTEND:20240101T130000Z
SUMMARY:Test Event
DESCRIPTION:This is a test event
LOCATION:Test Location
END:VEVENT
END:VCALENDAR"#;

        // TODO: Uncomment when implemented
        // let event = parse_event(ical).unwrap();
        // assert_eq!(event.uid, "test-event-123");
        // assert_eq!(event.summary, Some("Test Event".to_string()));
    }
}
