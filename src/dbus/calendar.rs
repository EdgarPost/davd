//! D-Bus calendar service implementation
//!
//! Provides the org.davd.Calendar interface for applications to query events.

use zbus::{interface, ConnectionBuilder};
use std::sync::Arc;
use chrono::DateTime;
use tracing::{info, error, warn};
use crate::{Database, Result};

/// D-Bus service for calendar access
pub struct CalendarService {
    /// Database connection (shared)
    db: Arc<Database>,
}

/// D-Bus representation of a calendar event
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, zbus::zvariant::Type)]
pub struct DbusEvent {
    /// Event ID
    pub id: i64,
    /// Event UID
    pub uid: String,
    /// Event summary/title
    pub summary: String,
    /// Event description
    pub description: String,
    /// Event location
    pub location: String,
    /// Start time (ISO 8601 string)
    pub start_time: String,
    /// End time (ISO 8601 string)
    pub end_time: String,
}

impl CalendarService {
    /// Create a new calendar service
    ///
    /// # Arguments
    /// * `db` - Shared database connection
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    /// Start the D-Bus service
    ///
    /// # Returns
    /// Ok(()) if service started successfully, Err otherwise
    ///
    /// # Design Decision
    /// We use the session bus instead of system bus because:
    /// - Calendar data is user-specific, not system-wide
    /// - No need for elevated privileges
    /// - Easier setup (no system bus policy files needed)
    pub async fn start(self) -> Result<()> {
        info!("Starting D-Bus calendar service");

        // Build D-Bus connection on session bus
        let _connection = ConnectionBuilder::session()?
            .name("org.davd.Calendar")?
            .serve_at("/org/davd/Calendar", self)?
            .build()
            .await?;

        info!("D-Bus service started at org.davd.Calendar");

        // Keep the connection alive
        // In a real daemon, this would be integrated with the main event loop
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        }
    }

    /// Convert a database Event to DbusEvent
    fn event_to_dbus(event: &crate::Event) -> DbusEvent {
        DbusEvent {
            id: event.id,
            uid: event.uid.clone(),
            summary: event.summary.clone().unwrap_or_default(),
            description: event.description.clone().unwrap_or_default(),
            location: event.location.clone().unwrap_or_default(),
            start_time: event.start_time.to_rfc3339(),
            end_time: event.end_time
                .map(|dt| dt.to_rfc3339())
                .unwrap_or_default(),
        }
    }
}

/// D-Bus interface implementation
#[interface(name = "org.davd.Calendar")]
impl CalendarService {
    /// List events within a time range
    ///
    /// # Arguments
    /// * `start` - Start time (ISO 8601 string)
    /// * `end` - End time (ISO 8601 string)
    ///
    /// # Returns
    /// A list of events in the time range
    ///
    /// # Errors
    /// Returns empty vec on error (D-Bus methods should not panic)
    async fn list_events(&self, start: String, end: String) -> Vec<DbusEvent> {
        // Parse ISO 8601 timestamps
        let start_time = match DateTime::parse_from_rfc3339(&start) {
            Ok(dt) => dt.with_timezone(&chrono::Utc),
            Err(e) => {
                error!("Failed to parse start time '{}': {}", start, e);
                return Vec::new();
            }
        };

        let end_time = match DateTime::parse_from_rfc3339(&end) {
            Ok(dt) => dt.with_timezone(&chrono::Utc),
            Err(e) => {
                error!("Failed to parse end time '{}': {}", end, e);
                return Vec::new();
            }
        };

        // Query database for events in range (all calendars)
        match self.db.list_events(None, start_time, end_time) {
            Ok(events) => {
                info!("Found {} events between {} and {}", events.len(), start, end);
                events.iter().map(Self::event_to_dbus).collect()
            }
            Err(e) => {
                error!("Failed to query events: {}", e);
                Vec::new()
            }
        }
    }

    /// Get a single event by ID
    ///
    /// # Arguments
    /// * `id` - Event ID
    ///
    /// # Returns
    /// The event details (empty values if not found)
    ///
    /// # Design Decision
    /// We return a DbusEvent with empty values instead of an error because:
    /// - D-Bus error handling is more complex
    /// - Callers can check for id == 0 or empty uid
    /// - Simpler for Phase 1 MVP
    async fn get_event(&self, id: i64) -> DbusEvent {
        match self.db.get_event(id) {
            Ok(Some(event)) => {
                info!("Retrieved event {}", id);
                Self::event_to_dbus(&event)
            }
            Ok(None) => {
                warn!("Event {} not found", id);
                // Return empty event to indicate not found
                DbusEvent {
                    id: 0,
                    uid: String::new(),
                    summary: String::new(),
                    description: String::new(),
                    location: String::new(),
                    start_time: String::new(),
                    end_time: String::new(),
                }
            }
            Err(e) => {
                error!("Failed to query event {}: {}", id, e);
                DbusEvent {
                    id: 0,
                    uid: String::new(),
                    summary: String::new(),
                    description: String::new(),
                    location: String::new(),
                    start_time: String::new(),
                    end_time: String::new(),
                }
            }
        }
    }

    /// Get the full iCalendar data for an event
    ///
    /// # Arguments
    /// * `id` - Event ID
    ///
    /// # Returns
    /// The raw iCalendar data as a string (empty if not found)
    async fn get_event_ical(&self, id: i64) -> String {
        match self.db.get_event(id) {
            Ok(Some(event)) => {
                info!("Retrieved iCalendar data for event {}", id);
                event.ical_data
            }
            Ok(None) => {
                warn!("Event {} not found", id);
                String::new()
            }
            Err(e) => {
                error!("Failed to query event {}: {}", id, e);
                String::new()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::{Account, Calendar, Event};
    use chrono::{TimeZone, Utc};

    /// Create a test account in the database
    fn create_test_account(db: &Database) -> Result<i64> {
        let account = Account::new(
            "Test Account".to_string(),
            "https://caldav.example.com".to_string(),
            "testuser".to_string(),
            "testpass".to_string(),
        );
        db.insert_account(&account)
    }

    #[tokio::test]
    async fn test_list_events() {
        let db = Arc::new(Database::in_memory().unwrap());
        let account_id = create_test_account(&db).unwrap();

        // Create calendar
        let calendar = Calendar::new(
            account_id,
            "Test Calendar".to_string(),
            "/calendars/test/".to_string(),
        );
        let calendar_id = db.insert_calendar(&calendar).unwrap();

        // Create test events
        let mut event1 = Event::new(
            calendar_id,
            "event-1".to_string(),
            "BEGIN:VCALENDAR\nEND:VCALENDAR".to_string(),
        );
        event1.summary = Some("Event 1".to_string());
        event1.description = Some("Description 1".to_string());
        event1.location = Some("Location 1".to_string());
        event1.start_time = Utc.with_ymd_and_hms(2024, 1, 15, 10, 0, 0).unwrap();
        event1.end_time = Some(Utc.with_ymd_and_hms(2024, 1, 15, 11, 0, 0).unwrap());
        db.upsert_event(&event1).unwrap();

        let mut event2 = Event::new(
            calendar_id,
            "event-2".to_string(),
            "BEGIN:VCALENDAR\nEND:VCALENDAR".to_string(),
        );
        event2.summary = Some("Event 2".to_string());
        event2.start_time = Utc.with_ymd_and_hms(2024, 1, 20, 14, 0, 0).unwrap();
        event2.end_time = Some(Utc.with_ymd_and_hms(2024, 1, 20, 15, 0, 0).unwrap());
        db.upsert_event(&event2).unwrap();

        // Create service
        let service = CalendarService::new(db.clone());

        // Test list_events
        let events = service.list_events(
            "2024-01-01T00:00:00Z".to_string(),
            "2024-02-01T00:00:00Z".to_string(),
        ).await;

        assert_eq!(events.len(), 2);
        assert_eq!(events[0].uid, "event-1");
        assert_eq!(events[0].summary, "Event 1");
        assert_eq!(events[0].description, "Description 1");
        assert_eq!(events[0].location, "Location 1");
        assert_eq!(events[1].uid, "event-2");
        assert_eq!(events[1].summary, "Event 2");
    }

    #[tokio::test]
    async fn test_list_events_with_filter() {
        let db = Arc::new(Database::in_memory().unwrap());
        let account_id = create_test_account(&db).unwrap();

        // Create calendar
        let calendar = Calendar::new(
            account_id,
            "Test Calendar".to_string(),
            "/calendars/test/".to_string(),
        );
        let calendar_id = db.insert_calendar(&calendar).unwrap();

        // Create events with different dates
        let mut event1 = Event::new(
            calendar_id,
            "event-1".to_string(),
            "BEGIN:VCALENDAR\nEND:VCALENDAR".to_string(),
        );
        event1.summary = Some("January Event".to_string());
        event1.start_time = Utc.with_ymd_and_hms(2024, 1, 15, 10, 0, 0).unwrap();
        db.upsert_event(&event1).unwrap();

        let mut event2 = Event::new(
            calendar_id,
            "event-2".to_string(),
            "BEGIN:VCALENDAR\nEND:VCALENDAR".to_string(),
        );
        event2.summary = Some("February Event".to_string());
        event2.start_time = Utc.with_ymd_and_hms(2024, 2, 15, 10, 0, 0).unwrap();
        db.upsert_event(&event2).unwrap();

        // Create service
        let service = CalendarService::new(db.clone());

        // Query only January events
        let events = service.list_events(
            "2024-01-01T00:00:00Z".to_string(),
            "2024-02-01T00:00:00Z".to_string(),
        ).await;

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].summary, "January Event");
    }

    #[tokio::test]
    async fn test_list_events_invalid_dates() {
        let db = Arc::new(Database::in_memory().unwrap());
        let service = CalendarService::new(db);

        // Test with invalid date format
        let events = service.list_events(
            "not-a-date".to_string(),
            "2024-02-01T00:00:00Z".to_string(),
        ).await;

        // Should return empty vec on error
        assert_eq!(events.len(), 0);
    }

    #[tokio::test]
    async fn test_get_event() {
        let db = Arc::new(Database::in_memory().unwrap());
        let account_id = create_test_account(&db).unwrap();

        // Create calendar and event
        let calendar = Calendar::new(
            account_id,
            "Test Calendar".to_string(),
            "/calendars/test/".to_string(),
        );
        let calendar_id = db.insert_calendar(&calendar).unwrap();

        let mut event = Event::new(
            calendar_id,
            "test-event".to_string(),
            "BEGIN:VCALENDAR\nEND:VCALENDAR".to_string(),
        );
        event.summary = Some("Test Event".to_string());
        event.description = Some("Test Description".to_string());
        event.location = Some("Test Location".to_string());
        event.start_time = Utc.with_ymd_and_hms(2024, 1, 15, 10, 0, 0).unwrap();
        event.end_time = Some(Utc.with_ymd_and_hms(2024, 1, 15, 11, 0, 0).unwrap());
        let event_id = db.upsert_event(&event).unwrap();

        // Create service
        let service = CalendarService::new(db.clone());

        // Test get_event
        let dbus_event = service.get_event(event_id).await;

        assert_eq!(dbus_event.id, event_id);
        assert_eq!(dbus_event.uid, "test-event");
        assert_eq!(dbus_event.summary, "Test Event");
        assert_eq!(dbus_event.description, "Test Description");
        assert_eq!(dbus_event.location, "Test Location");
        assert!(!dbus_event.start_time.is_empty());
        assert!(!dbus_event.end_time.is_empty());
    }

    #[tokio::test]
    async fn test_get_event_not_found() {
        let db = Arc::new(Database::in_memory().unwrap());
        let service = CalendarService::new(db);

        // Test with non-existent event ID
        let dbus_event = service.get_event(99999).await;

        // Should return empty event
        assert_eq!(dbus_event.id, 0);
        assert_eq!(dbus_event.uid, "");
        assert_eq!(dbus_event.summary, "");
    }

    #[tokio::test]
    async fn test_get_event_ical() {
        let db = Arc::new(Database::in_memory().unwrap());
        let account_id = create_test_account(&db).unwrap();

        // Create calendar and event
        let calendar = Calendar::new(
            account_id,
            "Test Calendar".to_string(),
            "/calendars/test/".to_string(),
        );
        let calendar_id = db.insert_calendar(&calendar).unwrap();

        let ical_data = r#"BEGIN:VCALENDAR
VERSION:2.0
BEGIN:VEVENT
UID:test-event
DTSTART:20240115T100000Z
SUMMARY:Test Event
END:VEVENT
END:VCALENDAR"#;

        let mut event = Event::new(
            calendar_id,
            "test-event".to_string(),
            ical_data.to_string(),
        );
        event.start_time = Utc.with_ymd_and_hms(2024, 1, 15, 10, 0, 0).unwrap();
        let event_id = db.upsert_event(&event).unwrap();

        // Create service
        let service = CalendarService::new(db.clone());

        // Test get_event_ical
        let returned_ical = service.get_event_ical(event_id).await;

        assert_eq!(returned_ical, ical_data);
        assert!(returned_ical.contains("UID:test-event"));
        assert!(returned_ical.contains("SUMMARY:Test Event"));
    }

    #[tokio::test]
    async fn test_get_event_ical_not_found() {
        let db = Arc::new(Database::in_memory().unwrap());
        let service = CalendarService::new(db);

        // Test with non-existent event ID
        let ical_data = service.get_event_ical(99999).await;

        // Should return empty string
        assert_eq!(ical_data, "");
    }
}
