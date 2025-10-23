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
///
/// # Design Decision
/// We use a simple line-based parser instead of a full XML/grammar parser because:
/// - iCalendar is a line-based format, not XML
/// - We only need basic VEVENT properties for Phase 1 MVP
/// - A simple parser is easier to debug and maintain
/// - Performance is adequate for typical calendar sizes
pub fn parse_event(ical_data: &str) -> Result<ICalEvent> {
    // First, unfold lines (handle continuation lines that start with space/tab)
    let unfolded = unfold_lines(ical_data);

    // Extract VEVENT component
    let vevent_data = extract_vevent(&unfolded)?;

    // Extract required properties
    let uid = extract_property(vevent_data, "UID")
        .ok_or_else(|| anyhow::anyhow!("Missing required property: UID"))?;

    let dtstart_str = extract_property(vevent_data, "DTSTART")
        .ok_or_else(|| anyhow::anyhow!("Missing required property: DTSTART"))?;
    let start_time = parse_datetime(&dtstart_str)?;

    // Extract optional properties
    let summary = extract_property(vevent_data, "SUMMARY");
    let description = extract_property(vevent_data, "DESCRIPTION");
    let location = extract_property(vevent_data, "LOCATION");
    let rrule = extract_property(vevent_data, "RRULE");

    let end_time = extract_property(vevent_data, "DTEND")
        .map(|s| parse_datetime(&s))
        .transpose()?;

    Ok(ICalEvent {
        uid,
        summary,
        description,
        location,
        start_time,
        end_time,
        rrule,
    })
}

/// Unfold lines according to RFC 5545 section 3.1
///
/// Lines that begin with a space or tab are continuations of the previous line.
/// This function joins such lines together by removing the CRLF and the
/// single whitespace character that follows it.
///
/// # Design Decision
/// We process the entire input at once rather than line-by-line because:
/// - It's simpler to reason about
/// - We need to look ahead to detect folded lines
/// - The input size is small (single calendar event)
///
/// Per RFC 5545, we strictly remove the CRLF and folding whitespace.
/// The original line should have been properly formatted before folding.
fn unfold_lines(data: &str) -> String {
    let mut result = String::new();
    let mut lines = data.lines();

    if let Some(first_line) = lines.next() {
        result.push_str(first_line);

        for line in lines {
            if line.starts_with(' ') || line.starts_with('\t') {
                // Continuation line - remove the folding whitespace
                result.push_str(&line[1..]);
            } else {
                // New line
                result.push('\n');
                result.push_str(line);
            }
        }
    }

    result
}

/// Extract the VEVENT component from iCalendar data
///
/// # Returns
/// The content between BEGIN:VEVENT and END:VEVENT
fn extract_vevent(data: &str) -> Result<&str> {
    let start_marker = "BEGIN:VEVENT";
    let end_marker = "END:VEVENT";

    let start = data.find(start_marker)
        .ok_or_else(|| anyhow::anyhow!("No VEVENT component found"))?;

    let end = data.find(end_marker)
        .ok_or_else(|| anyhow::anyhow!("VEVENT component not properly closed"))?;

    Ok(&data[start..end + end_marker.len()])
}

/// Extract a property value from iCalendar data
///
/// # Arguments
/// * `data` - The iCalendar data
/// * `property` - The property name (e.g., "UID", "SUMMARY")
///
/// # Returns
/// The property value if found, None otherwise
///
/// # Design Decision
/// We use simple string searching instead of regex because:
/// - iCalendar property format is simple: "PROPERTY:value" or "PROPERTY;params:value"
/// - String operations are faster than regex for simple patterns
/// - Less dependencies and easier to debug
fn extract_property(data: &str, property: &str) -> Option<String> {
    for line in data.lines() {
        // Check if line starts with the property name
        if line.starts_with(property) {
            // Make sure it's followed by ':' or ';' (not just a prefix match)
            if let Some(colon_pos) = line.find(':') {
                let before_colon = &line[..colon_pos];
                // Check this is our property (not a property that starts with our name)
                if before_colon == property || before_colon.starts_with(&format!("{};", property)) {
                    let value = &line[colon_pos + 1..];
                    return Some(unescape_text(value));
                }
            }
        }
    }

    None
}

/// Unescape iCalendar text according to RFC 5545 section 3.3.11
///
/// Escaped characters:
/// - \n or \N -> newline
/// - \, -> comma
/// - \; -> semicolon
/// - \\ -> backslash
fn unescape_text(text: &str) -> String {
    let mut result = String::new();
    let mut chars = text.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '\\' {
            if let Some(&next_ch) = chars.peek() {
                match next_ch {
                    'n' | 'N' => {
                        chars.next();
                        result.push('\n');
                    }
                    ',' => {
                        chars.next();
                        result.push(',');
                    }
                    ';' => {
                        chars.next();
                        result.push(';');
                    }
                    '\\' => {
                        chars.next();
                        result.push('\\');
                    }
                    _ => {
                        // Unknown escape sequence - keep the backslash
                        result.push(ch);
                    }
                }
            } else {
                // Backslash at end of string
                result.push(ch);
            }
        } else {
            result.push(ch);
        }
    }

    result
}

/// Parse an iCalendar date/time value
///
/// # Arguments
/// * `value` - The date/time string (e.g., "20240101T120000Z")
///
/// # Returns
/// A parsed DateTime<Utc>
///
/// # Supported Formats
/// - DATE-TIME with UTC: "20240101T120000Z"
/// - DATE (all-day): "20240101" (interpreted as midnight UTC)
///
/// # Design Decision
/// For Phase 1 MVP, we only support UTC times (Z suffix) and DATE format.
/// We don't handle:
/// - TZID parameters (local timezone times)
/// - Floating times (times without timezone)
/// This covers the common case for FastMail calendars and keeps the implementation simple.
/// Full timezone support can be added in Phase 2 if needed.
fn parse_datetime(value: &str) -> Result<DateTime<Utc>> {
    use chrono::TimeZone;

    // Strip any leading/trailing whitespace
    let value = value.trim();

    // Check if this is a DATE-TIME format (contains 'T')
    if value.contains('T') {
        // DATE-TIME format: "20240101T120000Z"
        // We only support UTC (Z suffix) for Phase 1
        if !value.ends_with('Z') {
            return Err(anyhow::anyhow!(
                "Only UTC times (ending with 'Z') are supported in Phase 1. Got: {}",
                value
            ));
        }

        // Parse: YYYYMMDDTHHMMSSZ
        if value.len() < 16 {
            return Err(anyhow::anyhow!("Invalid DATE-TIME format: {}", value));
        }

        let year = value[0..4].parse::<i32>()
            .map_err(|_| anyhow::anyhow!("Invalid year in: {}", value))?;
        let month = value[4..6].parse::<u32>()
            .map_err(|_| anyhow::anyhow!("Invalid month in: {}", value))?;
        let day = value[6..8].parse::<u32>()
            .map_err(|_| anyhow::anyhow!("Invalid day in: {}", value))?;
        let hour = value[9..11].parse::<u32>()
            .map_err(|_| anyhow::anyhow!("Invalid hour in: {}", value))?;
        let minute = value[11..13].parse::<u32>()
            .map_err(|_| anyhow::anyhow!("Invalid minute in: {}", value))?;
        let second = value[13..15].parse::<u32>()
            .map_err(|_| anyhow::anyhow!("Invalid second in: {}", value))?;

        Utc.with_ymd_and_hms(year, month, day, hour, minute, second)
            .single()
            .ok_or_else(|| anyhow::anyhow!("Invalid datetime: {}", value))
    } else {
        // DATE format: "20240101" (treat as midnight UTC)
        if value.len() != 8 {
            return Err(anyhow::anyhow!("Invalid DATE format: {}", value));
        }

        let year = value[0..4].parse::<i32>()
            .map_err(|_| anyhow::anyhow!("Invalid year in: {}", value))?;
        let month = value[4..6].parse::<u32>()
            .map_err(|_| anyhow::anyhow!("Invalid month in: {}", value))?;
        let day = value[6..8].parse::<u32>()
            .map_err(|_| anyhow::anyhow!("Invalid day in: {}", value))?;

        Utc.with_ymd_and_hms(year, month, day, 0, 0, 0)
            .single()
            .ok_or_else(|| anyhow::anyhow!("Invalid date: {}", value))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

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

        let event = parse_event(ical).unwrap();
        assert_eq!(event.uid, "test-event-123");
        assert_eq!(event.summary, Some("Test Event".to_string()));
        assert_eq!(event.description, Some("This is a test event".to_string()));
        assert_eq!(event.location, Some("Test Location".to_string()));
        assert_eq!(event.start_time, Utc.with_ymd_and_hms(2024, 1, 1, 12, 0, 0).unwrap());
        assert_eq!(event.end_time, Some(Utc.with_ymd_and_hms(2024, 1, 1, 13, 0, 0).unwrap()));
    }

    #[test]
    fn test_parse_event_minimal() {
        // Only UID and DTSTART are required
        let ical = r#"BEGIN:VCALENDAR
BEGIN:VEVENT
UID:minimal-event
DTSTART:20240215T090000Z
END:VEVENT
END:VCALENDAR"#;

        let event = parse_event(ical).unwrap();
        assert_eq!(event.uid, "minimal-event");
        assert_eq!(event.summary, None);
        assert_eq!(event.description, None);
        assert_eq!(event.location, None);
        assert_eq!(event.end_time, None);
        assert_eq!(event.start_time, Utc.with_ymd_and_hms(2024, 2, 15, 9, 0, 0).unwrap());
    }

    #[test]
    fn test_parse_date_only() {
        // All-day event (DATE format without time)
        let ical = r#"BEGIN:VEVENT
UID:allday-event
DTSTART:20240301
DTEND:20240302
SUMMARY:All Day Event
END:VEVENT"#;

        let event = parse_event(ical).unwrap();
        assert_eq!(event.uid, "allday-event");
        // DATE format should be interpreted as midnight UTC
        assert_eq!(event.start_time, Utc.with_ymd_and_hms(2024, 3, 1, 0, 0, 0).unwrap());
        assert_eq!(event.end_time, Some(Utc.with_ymd_and_hms(2024, 3, 2, 0, 0, 0).unwrap()));
    }

    #[test]
    fn test_parse_with_line_folding() {
        // iCalendar allows lines to be folded with leading space
        // Per RFC 5545: folding inserts CRLF + space, and any existing spacing
        // must be on the line before the fold point
        // Note: Using format! to explicitly include trailing space after "across"
        let ical = format!("BEGIN:VEVENT\nUID:folded-event\nDTSTART:20240101T120000Z\nSUMMARY:This is a very long summary that has been folded across \n multiple lines in the iCalendar format\nDESCRIPTION:Line one\n Line two continuation\n Line three continuation\nEND:VEVENT");

        let event = parse_event(&ical).unwrap();
        assert_eq!(event.uid, "folded-event");
        assert_eq!(
            event.summary,
            Some("This is a very long summary that has been folded across multiple lines in the iCalendar format".to_string())
        );
        // Note: DESCRIPTION lines are concatenated without spaces because
        // there's no space before the fold point in the original text
        assert_eq!(
            event.description,
            Some("Line oneLine two continuationLine three continuation".to_string())
        );
    }

    #[test]
    fn test_parse_with_escaped_characters() {
        // iCalendar escapes \n, \,, \;, \\
        let ical = r#"BEGIN:VEVENT
UID:escaped-event
DTSTART:20240101T120000Z
SUMMARY:Summary with\, comma and\; semicolon
DESCRIPTION:Line 1\nLine 2\nLine 3
LOCATION:Building A\\Room 101
END:VEVENT"#;

        let event = parse_event(ical).unwrap();
        assert_eq!(event.summary, Some("Summary with, comma and; semicolon".to_string()));
        assert_eq!(event.description, Some("Line 1\nLine 2\nLine 3".to_string()));
        assert_eq!(event.location, Some("Building A\\Room 101".to_string()));
    }

    #[test]
    fn test_parse_missing_uid() {
        let ical = r#"BEGIN:VEVENT
DTSTART:20240101T120000Z
SUMMARY:No UID Event
END:VEVENT"#;

        let result = parse_event(ical);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("UID"));
    }

    #[test]
    fn test_parse_missing_dtstart() {
        let ical = r#"BEGIN:VEVENT
UID:no-dtstart
SUMMARY:No Start Time
END:VEVENT"#;

        let result = parse_event(ical);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("DTSTART"));
    }

    #[test]
    fn test_parse_invalid_datetime() {
        let ical = r#"BEGIN:VEVENT
UID:invalid-date
DTSTART:not-a-date
END:VEVENT"#;

        let result = parse_event(ical);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_no_vevent() {
        let ical = r#"BEGIN:VCALENDAR
VERSION:2.0
END:VCALENDAR"#;

        let result = parse_event(ical);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("VEVENT"));
    }

    #[test]
    fn test_extract_property() {
        let data = "UID:test-123\nSUMMARY:Test\n";
        assert_eq!(extract_property(data, "UID"), Some("test-123".to_string()));
        assert_eq!(extract_property(data, "SUMMARY"), Some("Test".to_string()));
        assert_eq!(extract_property(data, "LOCATION"), None);
    }

    #[test]
    fn test_parse_datetime_formats() {
        // UTC time with Z suffix
        let dt = parse_datetime("20240101T120000Z").unwrap();
        assert_eq!(dt, Utc.with_ymd_and_hms(2024, 1, 1, 12, 0, 0).unwrap());

        // Date only (treated as midnight UTC for Phase 1)
        let dt = parse_datetime("20240101").unwrap();
        assert_eq!(dt, Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap());

        // Invalid format
        assert!(parse_datetime("not-a-date").is_err());
        assert!(parse_datetime("2024-01-01").is_err()); // Wrong format
    }
}
