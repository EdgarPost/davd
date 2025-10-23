//! SQLite database implementation
//!
//! Provides the main Database type for storing and querying calendar data.

use std::path::Path;
use rusqlite::{Connection, OptionalExtension, params};
use chrono::{DateTime, Utc};
use crate::Result;
use super::models::{Account, Calendar, Event};
use super::migrations;

/// SQLite database handle for calendar storage
pub struct Database {
    conn: Connection,
}

impl Database {
    /// Open or create a database at the specified path
    ///
    /// # Arguments
    /// * `path` - Path to the SQLite database file
    ///
    /// # Returns
    /// A new Database instance with migrations applied
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let conn = Connection::open(path)?;

        // Enable foreign key enforcement
        conn.execute("PRAGMA foreign_keys = ON", [])?;

        // Run migrations
        migrations::run_migrations(&conn)?;

        Ok(Self { conn })
    }

    /// Create an in-memory database (for testing)
    pub fn in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;

        // Enable foreign key enforcement
        conn.execute("PRAGMA foreign_keys = ON", [])?;

        // Run migrations
        migrations::run_migrations(&conn)?;

        Ok(Self { conn })
    }

    // Account operations

    /// Insert a new account
    pub fn insert_account(&self, account: &Account) -> Result<i64> {
        self.conn.execute(
            "INSERT INTO accounts (name, server_url, username, password, created_at, last_sync)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                account.name,
                account.server_url,
                account.username,
                account.password,
                account.created_at.to_rfc3339(),
                account.last_sync.map(|dt| dt.to_rfc3339()),
            ],
        )?;

        Ok(self.conn.last_insert_rowid())
    }

    /// Get an account by ID
    pub fn get_account(&self, id: i64) -> Result<Option<Account>> {
        let result = self.conn.query_row(
            "SELECT id, name, server_url, username, password, created_at, last_sync
             FROM accounts WHERE id = ?1",
            params![id],
            |row| {
                Ok(Account {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    server_url: row.get(2)?,
                    username: row.get(3)?,
                    password: row.get(4)?,
                    created_at: row.get::<_, String>(5)?.parse::<DateTime<Utc>>()
                        .map_err(|e| rusqlite::Error::FromSqlConversionFailure(
                            5, rusqlite::types::Type::Text, Box::new(e)
                        ))?,
                    last_sync: row.get::<_, Option<String>>(6)?
                        .map(|s| s.parse::<DateTime<Utc>>())
                        .transpose()
                        .map_err(|e| rusqlite::Error::FromSqlConversionFailure(
                            6, rusqlite::types::Type::Text, Box::new(e)
                        ))?,
                })
            },
        ).optional()?;

        Ok(result)
    }

    /// List all accounts
    pub fn list_accounts(&self) -> Result<Vec<Account>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, server_url, username, password, created_at, last_sync
             FROM accounts ORDER BY id"
        )?;

        let accounts = stmt.query_map([], |row| {
            Ok(Account {
                id: row.get(0)?,
                name: row.get(1)?,
                server_url: row.get(2)?,
                username: row.get(3)?,
                password: row.get(4)?,
                created_at: row.get::<_, String>(5)?.parse::<DateTime<Utc>>()
                    .map_err(|e| rusqlite::Error::FromSqlConversionFailure(
                        5, rusqlite::types::Type::Text, Box::new(e)
                    ))?,
                last_sync: row.get::<_, Option<String>>(6)?
                    .map(|s| s.parse::<DateTime<Utc>>())
                    .transpose()
                    .map_err(|e| rusqlite::Error::FromSqlConversionFailure(
                        6, rusqlite::types::Type::Text, Box::new(e)
                    ))?,
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(|e| anyhow::anyhow!(e))?;

        Ok(accounts)
    }

    /// Update an account
    pub fn update_account(&self, account: &Account) -> Result<()> {
        self.conn.execute(
            "UPDATE accounts
             SET name = ?1, server_url = ?2, username = ?3, password = ?4,
                 created_at = ?5, last_sync = ?6
             WHERE id = ?7",
            params![
                account.name,
                account.server_url,
                account.username,
                account.password,
                account.created_at.to_rfc3339(),
                account.last_sync.map(|dt| dt.to_rfc3339()),
                account.id,
            ],
        )?;

        Ok(())
    }

    /// Delete an account and all associated data
    pub fn delete_account(&self, id: i64) -> Result<()> {
        self.conn.execute("DELETE FROM accounts WHERE id = ?1", params![id])?;
        Ok(())
    }

    // Calendar operations

    /// Insert a new calendar
    pub fn insert_calendar(&self, calendar: &Calendar) -> Result<i64> {
        self.conn.execute(
            "INSERT INTO calendars (account_id, name, url, sync_token, color, enabled)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                calendar.account_id,
                calendar.name,
                calendar.url,
                calendar.sync_token,
                calendar.color,
                calendar.enabled,
            ],
        )?;

        Ok(self.conn.last_insert_rowid())
    }

    /// Get a calendar by ID
    pub fn get_calendar(&self, id: i64) -> Result<Option<Calendar>> {
        let result = self.conn.query_row(
            "SELECT id, account_id, name, url, sync_token, color, enabled
             FROM calendars WHERE id = ?1",
            params![id],
            |row| {
                Ok(Calendar {
                    id: row.get(0)?,
                    account_id: row.get(1)?,
                    name: row.get(2)?,
                    url: row.get(3)?,
                    sync_token: row.get(4)?,
                    color: row.get(5)?,
                    enabled: row.get(6)?,
                })
            },
        ).optional()?;

        Ok(result)
    }

    /// List calendars for an account
    pub fn list_calendars(&self, account_id: i64) -> Result<Vec<Calendar>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, account_id, name, url, sync_token, color, enabled
             FROM calendars WHERE account_id = ?1 ORDER BY id"
        )?;

        let calendars = stmt.query_map([account_id], |row| {
            Ok(Calendar {
                id: row.get(0)?,
                account_id: row.get(1)?,
                name: row.get(2)?,
                url: row.get(3)?,
                sync_token: row.get(4)?,
                color: row.get(5)?,
                enabled: row.get(6)?,
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(|e| anyhow::anyhow!(e))?;

        Ok(calendars)
    }

    /// Update a calendar
    pub fn update_calendar(&self, calendar: &Calendar) -> Result<()> {
        self.conn.execute(
            "UPDATE calendars
             SET account_id = ?1, name = ?2, url = ?3, sync_token = ?4, color = ?5, enabled = ?6
             WHERE id = ?7",
            params![
                calendar.account_id,
                calendar.name,
                calendar.url,
                calendar.sync_token,
                calendar.color,
                calendar.enabled,
                calendar.id,
            ],
        )?;

        Ok(())
    }

    /// Delete a calendar and all associated events
    pub fn delete_calendar(&self, id: i64) -> Result<()> {
        self.conn.execute("DELETE FROM calendars WHERE id = ?1", params![id])?;
        Ok(())
    }

    // Event operations

    /// Insert or update an event (upsert based on calendar_id + uid)
    pub fn upsert_event(&self, event: &Event) -> Result<i64> {
        // Try to find existing event by calendar_id + uid
        let existing_id: Option<i64> = self.conn.query_row(
            "SELECT id FROM events WHERE calendar_id = ?1 AND uid = ?2",
            params![event.calendar_id, event.uid],
            |row| row.get(0),
        ).optional()?;

        if let Some(id) = existing_id {
            // Update existing event
            self.conn.execute(
                "UPDATE events
                 SET summary = ?1, description = ?2, location = ?3,
                     start_time = ?4, end_time = ?5, etag = ?6,
                     ical_data = ?7, last_modified = ?8
                 WHERE id = ?9",
                params![
                    event.summary,
                    event.description,
                    event.location,
                    event.start_time.to_rfc3339(),
                    event.end_time.map(|dt| dt.to_rfc3339()),
                    event.etag,
                    event.ical_data,
                    event.last_modified.to_rfc3339(),
                    id,
                ],
            )?;
            Ok(id)
        } else {
            // Insert new event
            self.conn.execute(
                "INSERT INTO events (calendar_id, uid, summary, description, location,
                                    start_time, end_time, etag, ical_data, last_modified)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![
                    event.calendar_id,
                    event.uid,
                    event.summary,
                    event.description,
                    event.location,
                    event.start_time.to_rfc3339(),
                    event.end_time.map(|dt| dt.to_rfc3339()),
                    event.etag,
                    event.ical_data,
                    event.last_modified.to_rfc3339(),
                ],
            )?;
            Ok(self.conn.last_insert_rowid())
        }
    }

    /// Get an event by ID
    pub fn get_event(&self, id: i64) -> Result<Option<Event>> {
        let result = self.conn.query_row(
            "SELECT id, calendar_id, uid, summary, description, location,
                    start_time, end_time, etag, ical_data, last_modified
             FROM events WHERE id = ?1",
            params![id],
            |row| self.event_from_row(row),
        ).optional()?;

        Ok(result)
    }

    /// Get an event by calendar ID and UID
    pub fn get_event_by_uid(&self, calendar_id: i64, uid: &str) -> Result<Option<Event>> {
        let result = self.conn.query_row(
            "SELECT id, calendar_id, uid, summary, description, location,
                    start_time, end_time, etag, ical_data, last_modified
             FROM events WHERE calendar_id = ?1 AND uid = ?2",
            params![calendar_id, uid],
            |row| self.event_from_row(row),
        ).optional()?;

        Ok(result)
    }

    /// List events in a time range
    pub fn list_events(
        &self,
        calendar_id: Option<i64>,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<Event>> {
        if let Some(cal_id) = calendar_id {
            let mut stmt = self.conn.prepare(
                "SELECT id, calendar_id, uid, summary, description, location,
                        start_time, end_time, etag, ical_data, last_modified
                 FROM events
                 WHERE calendar_id = ?1 AND start_time >= ?2 AND start_time < ?3
                 ORDER BY start_time"
            )?;

            let events: Vec<Event> = stmt.query_map(
                params![cal_id, start.to_rfc3339(), end.to_rfc3339()],
                |row| self.event_from_row(row),
            )?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|e| anyhow::anyhow!(e))?;

            Ok(events)
        } else {
            let mut stmt = self.conn.prepare(
                "SELECT id, calendar_id, uid, summary, description, location,
                        start_time, end_time, etag, ical_data, last_modified
                 FROM events
                 WHERE start_time >= ?1 AND start_time < ?2
                 ORDER BY start_time"
            )?;

            let events: Vec<Event> = stmt.query_map(
                params![start.to_rfc3339(), end.to_rfc3339()],
                |row| self.event_from_row(row),
            )?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|e| anyhow::anyhow!(e))?;

            Ok(events)
        }
    }

    /// Delete an event
    pub fn delete_event(&self, id: i64) -> Result<()> {
        self.conn.execute("DELETE FROM events WHERE id = ?1", params![id])?;
        Ok(())
    }

    /// Delete an event by calendar ID and UID
    pub fn delete_event_by_uid(&self, calendar_id: i64, uid: &str) -> Result<()> {
        self.conn.execute(
            "DELETE FROM events WHERE calendar_id = ?1 AND uid = ?2",
            params![calendar_id, uid],
        )?;
        Ok(())
    }

    /// Helper to convert a database row to an Event
    fn event_from_row(&self, row: &rusqlite::Row) -> rusqlite::Result<Event> {
        Ok(Event {
            id: row.get(0)?,
            calendar_id: row.get(1)?,
            uid: row.get(2)?,
            summary: row.get(3)?,
            description: row.get(4)?,
            location: row.get(5)?,
            start_time: row.get::<_, String>(6)?.parse::<DateTime<Utc>>()
                .map_err(|e| rusqlite::Error::FromSqlConversionFailure(
                    6, rusqlite::types::Type::Text, Box::new(e)
                ))?,
            end_time: row.get::<_, Option<String>>(7)?
                .map(|s| s.parse::<DateTime<Utc>>())
                .transpose()
                .map_err(|e| rusqlite::Error::FromSqlConversionFailure(
                    7, rusqlite::types::Type::Text, Box::new(e)
                ))?,
            etag: row.get(8)?,
            ical_data: row.get(9)?,
            last_modified: row.get::<_, String>(10)?.parse::<DateTime<Utc>>()
                .map_err(|e| rusqlite::Error::FromSqlConversionFailure(
                    10, rusqlite::types::Type::Text, Box::new(e)
                ))?,
        })
    }
}
