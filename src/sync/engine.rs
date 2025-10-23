//! Sync engine that orchestrates CalDAV → Parser → Storage flow
//!
//! The SyncEngine coordinates the entire synchronization process:
//! 1. Discovers calendars from the CalDAV server
//! 2. Syncs calendar metadata to local database
//! 3. For each calendar, syncs events by comparing ETags
//! 4. Downloads and parses only changed events
//! 5. Stores events in the local database
//!
//! # Design Decision
//! We use a pull-based sync model instead of push notifications because:
//! - Most CalDAV servers don't support push notifications
//! - Pull-based is simpler and more reliable
//! - We can control sync frequency
//! - Works offline with cached data

use std::sync::Arc;
use crate::{Database, Result};
use crate::sync::{CalDavClient, CalendarInfo, CalDavEvent};
use crate::ical::{parse_event, ICalEvent};
use crate::storage::{Calendar, Event};
use chrono::Utc;
use tracing::{info, warn, error, debug};

/// Result of a sync operation
#[derive(Debug, Clone)]
pub struct SyncResult {
    /// Number of calendars synced
    pub calendars_synced: usize,
    /// Number of events synced (new or updated)
    pub events_synced: usize,
    /// Number of events that failed to sync
    pub events_failed: usize,
    /// Errors encountered during sync
    pub errors: Vec<String>,
}

/// Orchestrates the complete CalDAV sync flow
pub struct SyncEngine {
    /// Database for storing synced data
    db: Arc<Database>,
    /// CalDAV client for server communication
    client: CalDavClient,
    /// Account information
    account_id: i64,
}

impl SyncEngine {
    /// Create a new sync engine
    ///
    /// # Arguments
    /// * `db` - Database handle
    /// * `client` - CalDAV client
    /// * `account_id` - ID of the account to sync
    pub fn new(db: Arc<Database>, client: CalDavClient, account_id: i64) -> Self {
        Self {
            db,
            client,
            account_id,
        }
    }

    /// Run a complete sync operation
    ///
    /// # Returns
    /// A SyncResult with statistics and any errors encountered
    ///
    /// # Design Decision
    /// We continue syncing even if individual events fail because:
    /// - One bad event shouldn't block the entire sync
    /// - Users can still access successfully synced events
    /// - We log errors for debugging
    pub async fn sync(&self) -> Result<SyncResult> {
        info!("Starting sync for account {}", self.account_id);

        let mut result = SyncResult {
            calendars_synced: 0,
            events_synced: 0,
            events_failed: 0,
            errors: Vec::new(),
        };

        // Step 1: Discover calendar home
        let calendar_home = match self.client.discover_calendar_home().await {
            Ok(home) => {
                info!("Discovered calendar home: {}", home);
                home
            }
            Err(e) => {
                let error_msg = format!("Failed to discover calendar home: {}", e);
                error!("{}", error_msg);
                result.errors.push(error_msg);
                return Ok(result);
            }
        };

        // Step 2: List calendars
        let calendars = match self.client.list_calendars(&calendar_home).await {
            Ok(cals) => {
                info!("Found {} calendars", cals.len());
                cals
            }
            Err(e) => {
                let error_msg = format!("Failed to list calendars: {}", e);
                error!("{}", error_msg);
                result.errors.push(error_msg);
                return Ok(result);
            }
        };

        // Step 3: Sync each calendar
        for calendar_info in calendars {
            match self.sync_calendar(&calendar_info, &mut result).await {
                Ok(_) => {
                    result.calendars_synced += 1;
                }
                Err(e) => {
                    let error_msg = format!("Failed to sync calendar '{}': {}", calendar_info.name, e);
                    warn!("{}", error_msg);
                    result.errors.push(error_msg);
                }
            }
        }

        // Step 4: Update account's last_sync timestamp
        if let Ok(Some(mut account)) = self.db.get_account(self.account_id) {
            account.last_sync = Some(Utc::now());
            let _ = self.db.update_account(&account);
        }

        info!(
            "Sync complete: {} calendars, {} events synced, {} events failed",
            result.calendars_synced, result.events_synced, result.events_failed
        );

        Ok(result)
    }

    /// Sync a single calendar
    ///
    /// # Arguments
    /// * `calendar_info` - Calendar information from CalDAV
    /// * `result` - Mutable sync result to update
    async fn sync_calendar(
        &self,
        calendar_info: &CalendarInfo,
        result: &mut SyncResult,
    ) -> Result<()> {
        info!("Syncing calendar: {}", calendar_info.name);

        // Step 1: Upsert calendar to database
        let calendar_id = self.upsert_calendar(calendar_info)?;
        debug!("Calendar ID: {}", calendar_id);

        // Step 2: List events from CalDAV server
        let caldav_events = self.client.list_events(&calendar_info.url).await?;
        info!("Found {} events in calendar '{}'", caldav_events.len(), calendar_info.name);

        // Step 3: Sync each event
        for caldav_event in caldav_events {
            match self.sync_event(calendar_id, &caldav_event).await {
                Ok(true) => {
                    result.events_synced += 1;
                }
                Ok(false) => {
                    // Event unchanged, skip
                    debug!("Event {} unchanged, skipping", caldav_event.uid);
                }
                Err(e) => {
                    let error_msg = format!(
                        "Failed to sync event '{}' in calendar '{}': {}",
                        caldav_event.uid, calendar_info.name, e
                    );
                    warn!("{}", error_msg);
                    result.errors.push(error_msg);
                    result.events_failed += 1;
                }
            }
        }

        Ok(())
    }

    /// Upsert a calendar to the database
    ///
    /// # Arguments
    /// * `calendar_info` - Calendar information from CalDAV
    ///
    /// # Returns
    /// The calendar ID in the database
    fn upsert_calendar(&self, calendar_info: &CalendarInfo) -> Result<i64> {
        // Try to find existing calendar by account_id + url
        let existing_calendars = self.db.list_calendars(self.account_id)?;

        if let Some(existing) = existing_calendars.iter().find(|c| c.url == calendar_info.url) {
            // Update existing calendar
            let mut calendar = existing.clone();
            calendar.name = calendar_info.name.clone();
            calendar.color = calendar_info.color.clone();
            self.db.update_calendar(&calendar)?;
            Ok(calendar.id)
        } else {
            // Insert new calendar
            let mut calendar = Calendar::new(
                self.account_id,
                calendar_info.name.clone(),
                calendar_info.url.clone(),
            );
            calendar.color = calendar_info.color.clone();
            let id = self.db.insert_calendar(&calendar)?;
            Ok(id)
        }
    }

    /// Sync a single event
    ///
    /// # Arguments
    /// * `calendar_id` - Local calendar ID
    /// * `caldav_event` - Event information from CalDAV
    ///
    /// # Returns
    /// Ok(true) if event was synced (new or updated), Ok(false) if unchanged
    ///
    /// # Design Decision
    /// We use ETags for change detection because:
    /// - ETags are designed for this purpose (HTTP caching)
    /// - More reliable than comparing timestamps
    /// - Avoids downloading unchanged event data
    async fn sync_event(&self, calendar_id: i64, caldav_event: &CalDavEvent) -> Result<bool> {
        // Check if event exists and compare ETags
        if let Some(existing_event) = self.db.get_event_by_uid(calendar_id, &caldav_event.uid)? {
            if existing_event.etag.as_ref() == Some(&caldav_event.etag) {
                // ETag matches, event is unchanged
                debug!("Event {} unchanged (ETag match)", caldav_event.uid);
                return Ok(false);
            }
            debug!("Event {} has changed ETag, re-syncing", caldav_event.uid);
        } else {
            debug!("New event: {}", caldav_event.uid);
        }

        // Download full event data
        let ical_data = self.client.get_event(&caldav_event.url).await?;

        // Parse iCalendar data
        let parsed_event = parse_event(&ical_data)?;

        // Convert to database Event
        let mut event = self.ical_to_event(calendar_id, parsed_event, ical_data);
        event.etag = Some(caldav_event.etag.clone());
        event.uid = caldav_event.uid.clone();

        // Upsert to database
        self.db.upsert_event(&event)?;

        Ok(true)
    }

    /// Convert ICalEvent to database Event
    ///
    /// # Arguments
    /// * `calendar_id` - Calendar ID this event belongs to
    /// * `ical_event` - Parsed iCalendar event
    /// * `ical_data` - Raw iCalendar data (for round-trip compatibility)
    fn ical_to_event(&self, calendar_id: i64, ical_event: ICalEvent, ical_data: String) -> Event {
        Event {
            id: 0, // Will be set by database
            calendar_id,
            uid: ical_event.uid,
            summary: ical_event.summary,
            description: ical_event.description,
            location: ical_event.location,
            start_time: ical_event.start_time,
            end_time: ical_event.end_time,
            etag: None, // Will be set by caller
            ical_data,
            last_modified: Utc::now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito;
    use crate::storage::Account;
    use chrono::TimeZone;

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
    async fn test_sync_engine_new_events() {
        let db = Arc::new(Database::in_memory().unwrap());
        let account_id = create_test_account(&db).unwrap();

        let mut server = mockito::Server::new_async().await;

        // Mock calendar discovery
        let _mock_discovery = server.mock("PROPFIND", "/.well-known/caldav")
            .with_status(207)
            .with_body(r#"<?xml version="1.0"?>
<multistatus xmlns="DAV:" xmlns:C="urn:ietf:params:xml:ns:caldav">
  <response>
    <propstat>
      <prop>
        <C:calendar-home-set>
          <href>/calendars/testuser/</href>
        </C:calendar-home-set>
      </prop>
    </propstat>
  </response>
</multistatus>"#)
            .create_async()
            .await;

        // Mock calendar list
        let _mock_calendars = server.mock("PROPFIND", "/calendars/testuser/")
            .with_status(207)
            .with_body(r#"<?xml version="1.0"?>
<multistatus xmlns="DAV:" xmlns:C="urn:ietf:params:xml:ns:caldav">
  <response>
    <href>/calendars/testuser/default/</href>
    <propstat>
      <prop>
        <resourcetype>
          <collection/>
          <C:calendar/>
        </resourcetype>
        <displayname>My Calendar</displayname>
      </prop>
    </propstat>
  </response>
</multistatus>"#)
            .create_async()
            .await;

        // Mock event list
        let _mock_events = server.mock("REPORT", "/calendars/testuser/default/")
            .with_status(207)
            .with_body(r#"<?xml version="1.0"?>
<multistatus xmlns="DAV:" xmlns:C="urn:ietf:params:xml:ns:caldav">
  <response>
    <href>/calendars/testuser/default/event1.ics</href>
    <propstat>
      <prop>
        <getetag>"etag-123"</getetag>
        <C:calendar-data>
          <C:comp name="VCALENDAR">
            <C:comp name="VEVENT">
              <C:prop name="UID">event-1</C:prop>
            </C:comp>
          </C:comp>
        </C:calendar-data>
      </prop>
    </propstat>
  </response>
</multistatus>"#)
            .create_async()
            .await;

        // Mock event download
        let _mock_event_download = server.mock("GET", "/calendars/testuser/default/event1.ics")
            .with_status(200)
            .with_body(r#"BEGIN:VCALENDAR
VERSION:2.0
BEGIN:VEVENT
UID:event-1
DTSTART:20240101T120000Z
DTEND:20240101T130000Z
SUMMARY:Test Event
END:VEVENT
END:VCALENDAR"#)
            .create_async()
            .await;

        let client = CalDavClient::new(
            server.url(),
            "testuser".to_string(),
            "testpass".to_string(),
        ).unwrap();

        let engine = SyncEngine::new(db.clone(), client, account_id);
        let result = engine.sync().await.unwrap();

        assert_eq!(result.calendars_synced, 1);
        assert_eq!(result.events_synced, 1);
        assert_eq!(result.events_failed, 0);
        assert!(result.errors.is_empty());

        // Verify calendar was created
        let calendars = db.list_calendars(account_id).unwrap();
        assert_eq!(calendars.len(), 1);
        assert_eq!(calendars[0].name, "My Calendar");

        // Verify event was created
        let events = db.list_events(
            Some(calendars[0].id),
            chrono::Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap(),
            chrono::Utc.with_ymd_and_hms(2024, 1, 2, 0, 0, 0).unwrap(),
        ).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].uid, "event-1");
        assert_eq!(events[0].summary, Some("Test Event".to_string()));
        assert_eq!(events[0].etag, Some("etag-123".to_string()));
    }

    #[tokio::test]
    async fn test_sync_engine_update_changed_etag() {
        let db = Arc::new(Database::in_memory().unwrap());
        let account_id = create_test_account(&db).unwrap();

        // Create calendar and existing event
        let calendar = Calendar::new(
            account_id,
            "My Calendar".to_string(),
            "/calendars/testuser/default/".to_string(),
        );
        let calendar_id = db.insert_calendar(&calendar).unwrap();

        let mut event = Event::new(
            calendar_id,
            "event-1".to_string(),
            "BEGIN:VCALENDAR\nEND:VCALENDAR".to_string(),
        );
        event.summary = Some("Old Summary".to_string());
        event.etag = Some("old-etag".to_string());
        event.start_time = chrono::Utc.with_ymd_and_hms(2024, 1, 1, 12, 0, 0).unwrap();
        db.upsert_event(&event).unwrap();

        let mut server = mockito::Server::new_async().await;

        // Mock calendar discovery
        let _mock_discovery = server.mock("PROPFIND", "/.well-known/caldav")
            .with_status(207)
            .with_body(r#"<?xml version="1.0"?>
<multistatus xmlns="DAV:" xmlns:C="urn:ietf:params:xml:ns:caldav">
  <response>
    <propstat>
      <prop>
        <C:calendar-home-set>
          <href>/calendars/testuser/</href>
        </C:calendar-home-set>
      </prop>
    </propstat>
  </response>
</multistatus>"#)
            .create_async()
            .await;

        // Mock calendar list
        let _mock_calendars = server.mock("PROPFIND", "/calendars/testuser/")
            .with_status(207)
            .with_body(r#"<?xml version="1.0"?>
<multistatus xmlns="DAV:" xmlns:C="urn:ietf:params:xml:ns:caldav">
  <response>
    <href>/calendars/testuser/default/</href>
    <propstat>
      <prop>
        <resourcetype>
          <collection/>
          <C:calendar/>
        </resourcetype>
        <displayname>My Calendar</displayname>
      </prop>
    </propstat>
  </response>
</multistatus>"#)
            .create_async()
            .await;

        // Mock event list with new ETag
        let _mock_events = server.mock("REPORT", "/calendars/testuser/default/")
            .with_status(207)
            .with_body(r#"<?xml version="1.0"?>
<multistatus xmlns="DAV:" xmlns:C="urn:ietf:params:xml:ns:caldav">
  <response>
    <href>/calendars/testuser/default/event1.ics</href>
    <propstat>
      <prop>
        <getetag>"new-etag"</getetag>
        <C:calendar-data>
          <C:comp name="VCALENDAR">
            <C:comp name="VEVENT">
              <C:prop name="UID">event-1</C:prop>
            </C:comp>
          </C:comp>
        </C:calendar-data>
      </prop>
    </propstat>
  </response>
</multistatus>"#)
            .create_async()
            .await;

        // Mock event download with updated data
        let _mock_event_download = server.mock("GET", "/calendars/testuser/default/event1.ics")
            .with_status(200)
            .with_body(r#"BEGIN:VCALENDAR
VERSION:2.0
BEGIN:VEVENT
UID:event-1
DTSTART:20240101T120000Z
DTEND:20240101T130000Z
SUMMARY:Updated Summary
END:VEVENT
END:VCALENDAR"#)
            .create_async()
            .await;

        let client = CalDavClient::new(
            server.url(),
            "testuser".to_string(),
            "testpass".to_string(),
        ).unwrap();

        let engine = SyncEngine::new(db.clone(), client, account_id);
        let result = engine.sync().await.unwrap();

        assert_eq!(result.calendars_synced, 1);
        assert_eq!(result.events_synced, 1); // Event was updated
        assert_eq!(result.events_failed, 0);

        // Verify event was updated
        let updated_event = db.get_event_by_uid(calendar_id, "event-1").unwrap().unwrap();
        assert_eq!(updated_event.summary, Some("Updated Summary".to_string()));
        assert_eq!(updated_event.etag, Some("new-etag".to_string()));
    }

    #[tokio::test]
    async fn test_sync_engine_skip_unchanged_etag() {
        let db = Arc::new(Database::in_memory().unwrap());
        let account_id = create_test_account(&db).unwrap();

        // Create calendar and existing event with matching ETag
        let calendar = Calendar::new(
            account_id,
            "My Calendar".to_string(),
            "/calendars/testuser/default/".to_string(),
        );
        let calendar_id = db.insert_calendar(&calendar).unwrap();

        let mut event = Event::new(
            calendar_id,
            "event-1".to_string(),
            "BEGIN:VCALENDAR\nEND:VCALENDAR".to_string(),
        );
        event.summary = Some("Existing Event".to_string());
        event.etag = Some("same-etag".to_string());
        event.start_time = chrono::Utc.with_ymd_and_hms(2024, 1, 1, 12, 0, 0).unwrap();
        db.upsert_event(&event).unwrap();

        let mut server = mockito::Server::new_async().await;

        // Mock calendar discovery
        let _mock_discovery = server.mock("PROPFIND", "/.well-known/caldav")
            .with_status(207)
            .with_body(r#"<?xml version="1.0"?>
<multistatus xmlns="DAV:" xmlns:C="urn:ietf:params:xml:ns:caldav">
  <response>
    <propstat>
      <prop>
        <C:calendar-home-set>
          <href>/calendars/testuser/</href>
        </C:calendar-home-set>
      </prop>
    </propstat>
  </response>
</multistatus>"#)
            .create_async()
            .await;

        // Mock calendar list
        let _mock_calendars = server.mock("PROPFIND", "/calendars/testuser/")
            .with_status(207)
            .with_body(r#"<?xml version="1.0"?>
<multistatus xmlns="DAV:" xmlns:C="urn:ietf:params:xml:ns:caldav">
  <response>
    <href>/calendars/testuser/default/</href>
    <propstat>
      <prop>
        <resourcetype>
          <collection/>
          <C:calendar/>
        </resourcetype>
        <displayname>My Calendar</displayname>
      </prop>
    </propstat>
  </response>
</multistatus>"#)
            .create_async()
            .await;

        // Mock event list with same ETag
        let _mock_events = server.mock("REPORT", "/calendars/testuser/default/")
            .with_status(207)
            .with_body(r#"<?xml version="1.0"?>
<multistatus xmlns="DAV:" xmlns:C="urn:ietf:params:xml:ns:caldav">
  <response>
    <href>/calendars/testuser/default/event1.ics</href>
    <propstat>
      <prop>
        <getetag>"same-etag"</getetag>
        <C:calendar-data>
          <C:comp name="VCALENDAR">
            <C:comp name="VEVENT">
              <C:prop name="UID">event-1</C:prop>
            </C:comp>
          </C:comp>
        </C:calendar-data>
      </prop>
    </propstat>
  </response>
</multistatus>"#)
            .create_async()
            .await;

        // No GET mock - event should not be downloaded

        let client = CalDavClient::new(
            server.url(),
            "testuser".to_string(),
            "testpass".to_string(),
        ).unwrap();

        let engine = SyncEngine::new(db.clone(), client, account_id);
        let result = engine.sync().await.unwrap();

        assert_eq!(result.calendars_synced, 1);
        assert_eq!(result.events_synced, 0); // Event was skipped
        assert_eq!(result.events_failed, 0);

        // Verify event was not changed
        let unchanged_event = db.get_event_by_uid(calendar_id, "event-1").unwrap().unwrap();
        assert_eq!(unchanged_event.summary, Some("Existing Event".to_string()));
    }

    #[tokio::test]
    async fn test_sync_engine_handles_errors() {
        let db = Arc::new(Database::in_memory().unwrap());
        let account_id = create_test_account(&db).unwrap();

        let mut server = mockito::Server::new_async().await;

        // Mock calendar discovery failure
        let _mock_discovery = server.mock("PROPFIND", "/.well-known/caldav")
            .with_status(500)
            .create_async()
            .await;

        let client = CalDavClient::new(
            server.url(),
            "testuser".to_string(),
            "testpass".to_string(),
        ).unwrap();

        let engine = SyncEngine::new(db.clone(), client, account_id);
        let result = engine.sync().await.unwrap();

        // Sync should complete but with errors
        assert_eq!(result.calendars_synced, 0);
        assert_eq!(result.events_synced, 0);
        assert!(!result.errors.is_empty());
        assert!(result.errors[0].contains("calendar home"));
    }
}
