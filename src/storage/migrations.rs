//! Database migrations
//!
//! Manages schema creation and evolution.

use rusqlite::{Connection, OptionalExtension};
use crate::Result;

/// Current schema version
const CURRENT_VERSION: i32 = 1;

/// Apply all necessary migrations to bring the database up to date
///
/// # Arguments
/// * `conn` - The SQLite connection to migrate
///
/// # Returns
/// Ok(()) if migrations succeeded, Err otherwise
pub fn run_migrations(conn: &Connection) -> Result<()> {
    let current_version = get_schema_version(conn)?;

    if current_version == 0 {
        // Fresh database, create initial schema
        create_initial_schema(conn)?;
        set_schema_version(conn, CURRENT_VERSION)?;
    } else if current_version < CURRENT_VERSION {
        // Apply incremental migrations in the future
        // For now, we only have version 1
        anyhow::bail!("Database schema version {} is unsupported", current_version);
    }

    Ok(())
}

/// Create the initial database schema (version 1)
fn create_initial_schema(conn: &Connection) -> Result<()> {
    // Enable foreign key enforcement
    conn.execute("PRAGMA foreign_keys = ON", [])?;

    // Create schema_version table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS schema_version (
            version INTEGER PRIMARY KEY
        )",
        [],
    )?;

    // Create accounts table
    conn.execute(
        "CREATE TABLE accounts (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL UNIQUE,
            server_url TEXT NOT NULL,
            username TEXT NOT NULL,
            password TEXT NOT NULL,
            created_at TIMESTAMP NOT NULL,
            last_sync TIMESTAMP
        )",
        [],
    )?;

    // Create calendars table
    conn.execute(
        "CREATE TABLE calendars (
            id INTEGER PRIMARY KEY,
            account_id INTEGER NOT NULL,
            name TEXT NOT NULL,
            url TEXT NOT NULL,
            sync_token TEXT,
            color TEXT,
            enabled BOOLEAN NOT NULL DEFAULT 1,
            FOREIGN KEY (account_id) REFERENCES accounts(id) ON DELETE CASCADE,
            UNIQUE(account_id, url)
        )",
        [],
    )?;

    // Create events table
    conn.execute(
        "CREATE TABLE events (
            id INTEGER PRIMARY KEY,
            calendar_id INTEGER NOT NULL,
            uid TEXT NOT NULL,
            summary TEXT,
            description TEXT,
            location TEXT,
            start_time TIMESTAMP NOT NULL,
            end_time TIMESTAMP,
            etag TEXT,
            ical_data TEXT NOT NULL,
            last_modified TIMESTAMP NOT NULL,
            FOREIGN KEY (calendar_id) REFERENCES calendars(id) ON DELETE CASCADE,
            UNIQUE(calendar_id, uid)
        )",
        [],
    )?;

    // Create indexes for efficient queries
    conn.execute(
        "CREATE INDEX idx_events_time ON events(start_time, end_time)",
        [],
    )?;

    conn.execute(
        "CREATE INDEX idx_events_calendar ON events(calendar_id)",
        [],
    )?;

    conn.execute(
        "CREATE INDEX idx_calendars_account ON calendars(account_id)",
        [],
    )?;

    Ok(())
}

/// Get the current schema version
fn get_schema_version(conn: &Connection) -> Result<i32> {
    // Check if schema_version table exists
    let table_exists: bool = conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='schema_version'",
        [],
        |row| row.get(0),
    )?;

    if !table_exists {
        return Ok(0); // Fresh database
    }

    // Get the version
    let version: Option<i32> = conn
        .query_row("SELECT version FROM schema_version LIMIT 1", [], |row| {
            row.get(0)
        })
        .optional()?;

    Ok(version.unwrap_or(0))
}

/// Set the schema version
fn set_schema_version(conn: &Connection, version: i32) -> Result<()> {
    // Delete any existing version and insert the new one
    conn.execute("DELETE FROM schema_version", [])?;
    conn.execute("INSERT INTO schema_version (version) VALUES (?1)", [version])?;
    Ok(())
}
