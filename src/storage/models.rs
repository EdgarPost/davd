//! Data models for storage layer
//!
//! Defines the core data structures for accounts, calendars, and events.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Represents a CalDAV account
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    /// Unique account ID
    pub id: i64,
    /// Account name (user-defined)
    pub name: String,
    /// CalDAV server URL
    pub server_url: String,
    /// Username for authentication
    pub username: String,
    /// Password (stored in plaintext for Phase 1, secrets store in Phase 2)
    pub password: String,
    /// When the account was created
    pub created_at: DateTime<Utc>,
    /// Last successful sync timestamp
    pub last_sync: Option<DateTime<Utc>>,
}

/// Represents a calendar collection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Calendar {
    /// Unique calendar ID
    pub id: i64,
    /// Foreign key to account
    pub account_id: i64,
    /// Calendar name/display name
    pub name: String,
    /// CalDAV URL for this calendar
    pub url: String,
    /// Current sync token (for efficient syncing)
    pub sync_token: Option<String>,
    /// Calendar color (hex format, e.g., "#FF5733")
    pub color: Option<String>,
    /// Whether this calendar is enabled for syncing
    pub enabled: bool,
}

/// Represents a calendar event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    /// Unique event ID (local database)
    pub id: i64,
    /// Foreign key to calendar
    pub calendar_id: i64,
    /// Event UID from iCalendar (required for CalDAV sync)
    pub uid: String,
    /// Event summary/title
    pub summary: Option<String>,
    /// Event description
    pub description: Option<String>,
    /// Event location
    pub location: Option<String>,
    /// Event start time
    pub start_time: DateTime<Utc>,
    /// Event end time
    pub end_time: Option<DateTime<Utc>>,
    /// ETag from CalDAV (for change detection)
    pub etag: Option<String>,
    /// Full iCalendar data (stored for round-trip compatibility)
    pub ical_data: String,
    /// Last modification time
    pub last_modified: DateTime<Utc>,
}

impl Account {
    /// Create a new account
    pub fn new(name: String, server_url: String, username: String, password: String) -> Self {
        Self {
            id: 0, // Will be set by database
            name,
            server_url,
            username,
            password,
            created_at: Utc::now(),
            last_sync: None,
        }
    }
}

impl Calendar {
    /// Create a new calendar
    pub fn new(account_id: i64, name: String, url: String) -> Self {
        Self {
            id: 0, // Will be set by database
            account_id,
            name,
            url,
            sync_token: None,
            color: None,
            enabled: true,
        }
    }
}

impl Event {
    /// Create a new event
    pub fn new(calendar_id: i64, uid: String, ical_data: String) -> Self {
        Self {
            id: 0, // Will be set by database
            calendar_id,
            uid,
            summary: None,
            description: None,
            location: None,
            start_time: Utc::now(),
            end_time: None,
            etag: None,
            ical_data,
            last_modified: Utc::now(),
        }
    }
}
