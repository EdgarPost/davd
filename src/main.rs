//! davd daemon entry point
//!
//! This is the main entry point for the davd daemon. It can be run in two modes:
//! - CLI mode: for manual sync triggers and queries
//! - Daemon mode: background service that runs the D-Bus interface

use clap::Parser;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use std::sync::Arc;
use chrono::DateTime;

mod cli;

use cli::{Cli, Commands};

// Phase 1: Hardcoded credentials for testing
// TODO: Phase 2 - Move to config file and Secret Service
const SERVER_URL: &str = "https://caldav.fastmail.com";
const USERNAME: &str = "test@fastmail.com";
const PASSWORD: &str = "app-password";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "davd=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Sync { account } => {
            cmd_sync(account).await?;
        }
        Commands::ListEvents { start, end } => {
            cmd_list_events(start, end).await?;
        }
        Commands::Daemon => {
            cmd_daemon().await?;
        }
    }

    Ok(())
}

/// Handle the sync command
///
/// # Arguments
/// * `account` - Optional account name to sync (syncs all if None)
///
/// # Design Decision
/// For Phase 1, we hardcode FastMail credentials for testing.
/// Phase 2 will add proper account management with config files and Secret Service.
async fn cmd_sync(account: Option<String>) -> anyhow::Result<()> {
    use davd::{Database, sync::{CalDavClient, SyncEngine}};
    use davd::storage::Account;

    tracing::info!("Starting sync for account: {:?}", account);

    // Get database path (XDG_DATA_HOME or ~/.local/share)
    let db_path = get_db_path()?;
    tracing::info!("Using database at: {}", db_path.display());

    // Open database
    let db = Arc::new(Database::open(&db_path)?);

    // For Phase 1, we use a hardcoded account
    // Check if account exists, create if not
    let account_id = match db.list_accounts()?.into_iter().find(|a| a.username == USERNAME) {
        Some(acc) => {
            tracing::info!("Using existing account: {}", acc.name);
            acc.id
        }
        None => {
            tracing::info!("Creating new account for {}", USERNAME);
            let account = Account::new(
                account.unwrap_or_else(|| "FastMail".to_string()),
                SERVER_URL.to_string(),
                USERNAME.to_string(),
                PASSWORD.to_string(),
            );
            db.insert_account(&account)?
        }
    };

    // Create CalDAV client
    tracing::info!("Connecting to CalDAV server: {}", SERVER_URL);
    let client = CalDavClient::new(
        SERVER_URL.to_string(),
        USERNAME.to_string(),
        PASSWORD.to_string(),
    )?;

    // Create sync engine and run sync
    let engine = SyncEngine::new(db.clone(), client, account_id);
    let result = engine.sync().await?;

    // Print results
    println!("\n=== Sync Results ===");
    println!("Calendars synced: {}", result.calendars_synced);
    println!("Events synced: {}", result.events_synced);
    println!("Events failed: {}", result.events_failed);

    if !result.errors.is_empty() {
        println!("\nErrors:");
        for error in &result.errors {
            println!("  - {}", error);
        }
    }

    if result.errors.is_empty() {
        println!("\nSync completed successfully!");
    } else {
        println!("\nSync completed with {} errors", result.errors.len());
    }

    Ok(())
}

/// Handle the list-events command
///
/// # Arguments
/// * `start` - Start time (ISO 8601 format)
/// * `end` - End time (ISO 8601 format)
async fn cmd_list_events(start: String, end: String) -> anyhow::Result<()> {
    use davd::Database;

    tracing::info!("Listing events from {} to {}", start, end);

    // Parse timestamps
    let start_time = DateTime::parse_from_rfc3339(&start)
        .map_err(|e| anyhow::anyhow!("Invalid start time '{}': {}", start, e))?
        .with_timezone(&chrono::Utc);

    let end_time = DateTime::parse_from_rfc3339(&end)
        .map_err(|e| anyhow::anyhow!("Invalid end time '{}': {}", end, e))?
        .with_timezone(&chrono::Utc);

    // Get database path
    let db_path = get_db_path()?;
    let db = Database::open(&db_path)?;

    // Query events
    let events = db.list_events(None, start_time, end_time)?;

    if events.is_empty() {
        println!("No events found in the specified time range.");
        return Ok(());
    }

    // Print events in a formatted table
    println!("\n=== Events ({}) ===", events.len());
    println!("{:<5} {:<20} {:<30} {:<20}", "ID", "Start Time", "Summary", "Location");
    println!("{}", "-".repeat(80));

    for event in events {
        let summary = event.summary.as_deref().unwrap_or("(No title)");
        let location = event.location.as_deref().unwrap_or("");
        let start = event.start_time.format("%Y-%m-%d %H:%M");

        println!(
            "{:<5} {:<20} {:<30} {:<20}",
            event.id,
            start,
            truncate(summary, 30),
            truncate(location, 20)
        );
    }

    println!();

    Ok(())
}

/// Handle the daemon command
///
/// # Design Decision
/// For Phase 1, we just start the D-Bus service.
/// Phase 2 will add periodic sync scheduling.
async fn cmd_daemon() -> anyhow::Result<()> {
    use davd::{Database, dbus::CalendarService};

    tracing::info!("Starting davd daemon");

    // Get database path
    let db_path = get_db_path()?;
    tracing::info!("Using database at: {}", db_path.display());

    // Open database
    let db = Arc::new(Database::open(&db_path)?);

    // Create and start D-Bus service
    let service = CalendarService::new(db);

    println!("davd daemon starting...");
    println!("D-Bus service: org.davd.Calendar");
    println!("Object path: /org/davd/Calendar");
    println!("\nPress Ctrl+C to stop");

    // Start service (blocks until stopped)
    service.start().await?;

    Ok(())
}

/// Get the database file path
///
/// # Returns
/// Path to the SQLite database file
///
/// # Design Decision
/// We use XDG_DATA_HOME for the database location because:
/// - It's the standard location for application data on Linux
/// - Respects user preferences via environment variables
/// - Default to ~/.local/share if not set
fn get_db_path() -> anyhow::Result<std::path::PathBuf> {
    let data_dir = if let Ok(xdg_data_home) = std::env::var("XDG_DATA_HOME") {
        std::path::PathBuf::from(xdg_data_home)
    } else {
        let home = std::env::var("HOME")
            .map_err(|_| anyhow::anyhow!("HOME environment variable not set"))?;
        std::path::PathBuf::from(home).join(".local").join("share")
    };

    let davd_dir = data_dir.join("davd");

    // Create directory if it doesn't exist
    std::fs::create_dir_all(&davd_dir)?;

    Ok(davd_dir.join("davd.db"))
}

/// Truncate a string to a maximum length
fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}
