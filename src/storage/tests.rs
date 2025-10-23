//! Tests for the storage layer
//!
//! These tests verify database operations, constraints, and data integrity.

use super::*;
use chrono::Utc;

/// Helper to create a test database
fn setup_db() -> Database {
    Database::in_memory().expect("Failed to create test database")
}

#[test]
fn test_database_creation() {
    let db = setup_db();
    // If we got here, the database was created successfully with migrations

    // Verify we can list accounts (should be empty initially)
    let accounts = db.list_accounts().expect("Failed to list accounts");
    assert_eq!(accounts.len(), 0);
}

// --- Account CRUD Tests ---

#[test]
fn test_insert_account() {
    let db = setup_db();

    let account = Account::new(
        "test_account".to_string(),
        "https://caldav.example.com".to_string(),
        "user@example.com".to_string(),
        "password123".to_string(),
    );

    let id = db.insert_account(&account).expect("Failed to insert account");
    assert!(id > 0, "Account ID should be positive");
}

#[test]
fn test_get_account() {
    let db = setup_db();

    let account = Account::new(
        "test_account".to_string(),
        "https://caldav.example.com".to_string(),
        "user@example.com".to_string(),
        "password123".to_string(),
    );

    let id = db.insert_account(&account).expect("Failed to insert account");

    let retrieved = db.get_account(id).expect("Failed to get account")
        .expect("Account should exist");

    assert_eq!(retrieved.id, id);
    assert_eq!(retrieved.name, "test_account");
    assert_eq!(retrieved.username, "user@example.com");
    assert_eq!(retrieved.server_url, "https://caldav.example.com");
    assert_eq!(retrieved.password, "password123");
}

#[test]
fn test_get_nonexistent_account() {
    let db = setup_db();

    let result = db.get_account(999).expect("Failed to query account");
    assert!(result.is_none(), "Nonexistent account should return None");
}

#[test]
fn test_list_accounts() {
    let db = setup_db();

    // Insert multiple accounts
    let account1 = Account::new(
        "account1".to_string(),
        "https://caldav1.example.com".to_string(),
        "user1@example.com".to_string(),
        "pass1".to_string(),
    );

    let account2 = Account::new(
        "account2".to_string(),
        "https://caldav2.example.com".to_string(),
        "user2@example.com".to_string(),
        "pass2".to_string(),
    );

    db.insert_account(&account1).expect("Failed to insert account1");
    db.insert_account(&account2).expect("Failed to insert account2");

    let accounts = db.list_accounts().expect("Failed to list accounts");
    assert_eq!(accounts.len(), 2);
}

#[test]
fn test_update_account() {
    let db = setup_db();

    let mut account = Account::new(
        "test_account".to_string(),
        "https://caldav.example.com".to_string(),
        "user@example.com".to_string(),
        "password123".to_string(),
    );

    let id = db.insert_account(&account).expect("Failed to insert account");
    account.id = id;

    // Update the account
    account.last_sync = Some(Utc::now());
    account.name = "updated_account".to_string();

    db.update_account(&account).expect("Failed to update account");

    // Retrieve and verify
    let retrieved = db.get_account(id).expect("Failed to get account")
        .expect("Account should exist");

    assert_eq!(retrieved.name, "updated_account");
    assert!(retrieved.last_sync.is_some());
}

#[test]
fn test_delete_account() {
    let db = setup_db();

    let account = Account::new(
        "test_account".to_string(),
        "https://caldav.example.com".to_string(),
        "user@example.com".to_string(),
        "password123".to_string(),
    );

    let id = db.insert_account(&account).expect("Failed to insert account");

    db.delete_account(id).expect("Failed to delete account");

    let result = db.get_account(id).expect("Failed to query account");
    assert!(result.is_none(), "Deleted account should not exist");
}

#[test]
fn test_account_unique_name() {
    let db = setup_db();

    let account1 = Account::new(
        "duplicate_name".to_string(),
        "https://caldav.example.com".to_string(),
        "user@example.com".to_string(),
        "password123".to_string(),
    );

    let account2 = Account::new(
        "duplicate_name".to_string(),
        "https://caldav2.example.com".to_string(),
        "user2@example.com".to_string(),
        "password456".to_string(),
    );

    db.insert_account(&account1).expect("Failed to insert first account");

    // Second insert should fail due to unique constraint
    let result = db.insert_account(&account2);
    assert!(result.is_err(), "Duplicate account name should fail");
}

// --- Calendar CRUD Tests ---

#[test]
fn test_insert_calendar() {
    let db = setup_db();

    let account = Account::new(
        "test_account".to_string(),
        "https://caldav.example.com".to_string(),
        "user@example.com".to_string(),
        "password123".to_string(),
    );
    let account_id = db.insert_account(&account).expect("Failed to insert account");

    let calendar = Calendar::new(
        account_id,
        "Work Calendar".to_string(),
        "https://caldav.example.com/calendars/work".to_string(),
    );

    let id = db.insert_calendar(&calendar).expect("Failed to insert calendar");
    assert!(id > 0, "Calendar ID should be positive");
}

#[test]
fn test_get_calendar() {
    let db = setup_db();

    let account = Account::new(
        "test_account".to_string(),
        "https://caldav.example.com".to_string(),
        "user@example.com".to_string(),
        "password123".to_string(),
    );
    let account_id = db.insert_account(&account).expect("Failed to insert account");

    let calendar = Calendar::new(
        account_id,
        "Work Calendar".to_string(),
        "https://caldav.example.com/calendars/work".to_string(),
    );

    let id = db.insert_calendar(&calendar).expect("Failed to insert calendar");

    let retrieved = db.get_calendar(id).expect("Failed to get calendar")
        .expect("Calendar should exist");

    assert_eq!(retrieved.id, id);
    assert_eq!(retrieved.account_id, account_id);
    assert_eq!(retrieved.name, "Work Calendar");
    assert_eq!(retrieved.url, "https://caldav.example.com/calendars/work");
    assert!(retrieved.enabled);
}

#[test]
fn test_list_calendars() {
    let db = setup_db();

    let account = Account::new(
        "test_account".to_string(),
        "https://caldav.example.com".to_string(),
        "user@example.com".to_string(),
        "password123".to_string(),
    );
    let account_id = db.insert_account(&account).expect("Failed to insert account");

    let cal1 = Calendar::new(
        account_id,
        "Work".to_string(),
        "https://caldav.example.com/calendars/work".to_string(),
    );
    let cal2 = Calendar::new(
        account_id,
        "Personal".to_string(),
        "https://caldav.example.com/calendars/personal".to_string(),
    );

    db.insert_calendar(&cal1).expect("Failed to insert calendar 1");
    db.insert_calendar(&cal2).expect("Failed to insert calendar 2");

    let calendars = db.list_calendars(account_id).expect("Failed to list calendars");
    assert_eq!(calendars.len(), 2);
}

#[test]
fn test_update_calendar() {
    let db = setup_db();

    let account = Account::new(
        "test_account".to_string(),
        "https://caldav.example.com".to_string(),
        "user@example.com".to_string(),
        "password123".to_string(),
    );
    let account_id = db.insert_account(&account).expect("Failed to insert account");

    let mut calendar = Calendar::new(
        account_id,
        "Work Calendar".to_string(),
        "https://caldav.example.com/calendars/work".to_string(),
    );

    let id = db.insert_calendar(&calendar).expect("Failed to insert calendar");
    calendar.id = id;

    // Update the calendar
    calendar.name = "Updated Work Calendar".to_string();
    calendar.color = Some("#FF5733".to_string());
    calendar.sync_token = Some("sync-token-123".to_string());

    db.update_calendar(&calendar).expect("Failed to update calendar");

    // Retrieve and verify
    let retrieved = db.get_calendar(id).expect("Failed to get calendar")
        .expect("Calendar should exist");

    assert_eq!(retrieved.name, "Updated Work Calendar");
    assert_eq!(retrieved.color, Some("#FF5733".to_string()));
    assert_eq!(retrieved.sync_token, Some("sync-token-123".to_string()));
}

#[test]
fn test_delete_calendar() {
    let db = setup_db();

    let account = Account::new(
        "test_account".to_string(),
        "https://caldav.example.com".to_string(),
        "user@example.com".to_string(),
        "password123".to_string(),
    );
    let account_id = db.insert_account(&account).expect("Failed to insert account");

    let calendar = Calendar::new(
        account_id,
        "Work Calendar".to_string(),
        "https://caldav.example.com/calendars/work".to_string(),
    );

    let id = db.insert_calendar(&calendar).expect("Failed to insert calendar");

    db.delete_calendar(id).expect("Failed to delete calendar");

    let result = db.get_calendar(id).expect("Failed to query calendar");
    assert!(result.is_none(), "Deleted calendar should not exist");
}

#[test]
fn test_calendar_foreign_key() {
    let db = setup_db();

    // Try to insert calendar with non-existent account_id
    let calendar = Calendar::new(
        999, // Non-existent account
        "Test Calendar".to_string(),
        "https://caldav.example.com/calendars/test".to_string(),
    );

    let result = db.insert_calendar(&calendar);
    assert!(result.is_err(), "Calendar with invalid account_id should fail");
}

#[test]
fn test_delete_account_cascades_to_calendars() {
    let db = setup_db();

    let account = Account::new(
        "test_account".to_string(),
        "https://caldav.example.com".to_string(),
        "user@example.com".to_string(),
        "password123".to_string(),
    );
    let account_id = db.insert_account(&account).expect("Failed to insert account");

    let calendar = Calendar::new(
        account_id,
        "Work Calendar".to_string(),
        "https://caldav.example.com/calendars/work".to_string(),
    );
    let calendar_id = db.insert_calendar(&calendar).expect("Failed to insert calendar");

    // Delete the account
    db.delete_account(account_id).expect("Failed to delete account");

    // Calendar should also be deleted
    let result = db.get_calendar(calendar_id).expect("Failed to query calendar");
    assert!(result.is_none(), "Calendar should be deleted when account is deleted");
}

// --- Event CRUD Tests ---

#[test]
fn test_upsert_event() {
    let db = setup_db();

    let account = Account::new(
        "test_account".to_string(),
        "https://caldav.example.com".to_string(),
        "user@example.com".to_string(),
        "password123".to_string(),
    );
    let account_id = db.insert_account(&account).expect("Failed to insert account");

    let calendar = Calendar::new(
        account_id,
        "Work Calendar".to_string(),
        "https://caldav.example.com/calendars/work".to_string(),
    );
    let calendar_id = db.insert_calendar(&calendar).expect("Failed to insert calendar");

    let event = Event::new(
        calendar_id,
        "event-uid-123".to_string(),
        "BEGIN:VEVENT\nEND:VEVENT".to_string(),
    );

    let id = db.upsert_event(&event).expect("Failed to upsert event");
    assert!(id > 0, "Event ID should be positive");
}

#[test]
fn test_upsert_event_updates_existing() {
    let db = setup_db();

    let account = Account::new(
        "test_account".to_string(),
        "https://caldav.example.com".to_string(),
        "user@example.com".to_string(),
        "password123".to_string(),
    );
    let account_id = db.insert_account(&account).expect("Failed to insert account");

    let calendar = Calendar::new(
        account_id,
        "Work Calendar".to_string(),
        "https://caldav.example.com/calendars/work".to_string(),
    );
    let calendar_id = db.insert_calendar(&calendar).expect("Failed to insert calendar");

    let mut event = Event::new(
        calendar_id,
        "event-uid-123".to_string(),
        "BEGIN:VEVENT\nEND:VEVENT".to_string(),
    );
    event.summary = Some("Meeting".to_string());

    let id1 = db.upsert_event(&event).expect("Failed to upsert event");

    // Update with same calendar_id and uid
    event.summary = Some("Updated Meeting".to_string());
    event.etag = Some("etag-456".to_string());

    let id2 = db.upsert_event(&event).expect("Failed to upsert event again");

    // Should return the same ID (update, not insert)
    assert_eq!(id1, id2, "Upsert should update existing event");

    let retrieved = db.get_event(id1).expect("Failed to get event")
        .expect("Event should exist");

    assert_eq!(retrieved.summary, Some("Updated Meeting".to_string()));
    assert_eq!(retrieved.etag, Some("etag-456".to_string()));
}

#[test]
fn test_get_event() {
    let db = setup_db();

    let account = Account::new(
        "test_account".to_string(),
        "https://caldav.example.com".to_string(),
        "user@example.com".to_string(),
        "password123".to_string(),
    );
    let account_id = db.insert_account(&account).expect("Failed to insert account");

    let calendar = Calendar::new(
        account_id,
        "Work Calendar".to_string(),
        "https://caldav.example.com/calendars/work".to_string(),
    );
    let calendar_id = db.insert_calendar(&calendar).expect("Failed to insert calendar");

    let mut event = Event::new(
        calendar_id,
        "event-uid-123".to_string(),
        "BEGIN:VEVENT\nEND:VEVENT".to_string(),
    );
    event.summary = Some("Team Meeting".to_string());
    event.description = Some("Quarterly review".to_string());

    let id = db.upsert_event(&event).expect("Failed to upsert event");

    let retrieved = db.get_event(id).expect("Failed to get event")
        .expect("Event should exist");

    assert_eq!(retrieved.id, id);
    assert_eq!(retrieved.calendar_id, calendar_id);
    assert_eq!(retrieved.uid, "event-uid-123");
    assert_eq!(retrieved.summary, Some("Team Meeting".to_string()));
    assert_eq!(retrieved.description, Some("Quarterly review".to_string()));
}

#[test]
fn test_get_event_by_uid() {
    let db = setup_db();

    let account = Account::new(
        "test_account".to_string(),
        "https://caldav.example.com".to_string(),
        "user@example.com".to_string(),
        "password123".to_string(),
    );
    let account_id = db.insert_account(&account).expect("Failed to insert account");

    let calendar = Calendar::new(
        account_id,
        "Work Calendar".to_string(),
        "https://caldav.example.com/calendars/work".to_string(),
    );
    let calendar_id = db.insert_calendar(&calendar).expect("Failed to insert calendar");

    let event = Event::new(
        calendar_id,
        "event-uid-456".to_string(),
        "BEGIN:VEVENT\nEND:VEVENT".to_string(),
    );

    db.upsert_event(&event).expect("Failed to upsert event");

    let retrieved = db.get_event_by_uid(calendar_id, "event-uid-456")
        .expect("Failed to get event by UID")
        .expect("Event should exist");

    assert_eq!(retrieved.uid, "event-uid-456");
    assert_eq!(retrieved.calendar_id, calendar_id);
}

#[test]
fn test_list_events_by_time_range() {
    let db = setup_db();

    let account = Account::new(
        "test_account".to_string(),
        "https://caldav.example.com".to_string(),
        "user@example.com".to_string(),
        "password123".to_string(),
    );
    let account_id = db.insert_account(&account).expect("Failed to insert account");

    let calendar = Calendar::new(
        account_id,
        "Work Calendar".to_string(),
        "https://caldav.example.com/calendars/work".to_string(),
    );
    let calendar_id = db.insert_calendar(&calendar).expect("Failed to insert calendar");

    // Create events at different times
    let now = Utc::now();
    let yesterday = now - chrono::Duration::days(1);
    let tomorrow = now + chrono::Duration::days(1);
    let next_week = now + chrono::Duration::days(7);

    let mut event1 = Event::new(
        calendar_id,
        "event-yesterday".to_string(),
        "BEGIN:VEVENT\nEND:VEVENT".to_string(),
    );
    event1.start_time = yesterday;

    let mut event2 = Event::new(
        calendar_id,
        "event-tomorrow".to_string(),
        "BEGIN:VEVENT\nEND:VEVENT".to_string(),
    );
    event2.start_time = tomorrow;

    let mut event3 = Event::new(
        calendar_id,
        "event-next-week".to_string(),
        "BEGIN:VEVENT\nEND:VEVENT".to_string(),
    );
    event3.start_time = next_week;

    db.upsert_event(&event1).expect("Failed to insert event1");
    db.upsert_event(&event2).expect("Failed to insert event2");
    db.upsert_event(&event3).expect("Failed to insert event3");

    // Query events in the next 5 days
    let events = db.list_events(
        Some(calendar_id),
        now,
        now + chrono::Duration::days(5),
    ).expect("Failed to list events");

    // Should only get event2 (tomorrow), not event1 (past) or event3 (too far future)
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].uid, "event-tomorrow");
}

#[test]
fn test_list_events_all_calendars() {
    let db = setup_db();

    let account = Account::new(
        "test_account".to_string(),
        "https://caldav.example.com".to_string(),
        "user@example.com".to_string(),
        "password123".to_string(),
    );
    let account_id = db.insert_account(&account).expect("Failed to insert account");

    let cal1 = Calendar::new(
        account_id,
        "Work".to_string(),
        "https://caldav.example.com/calendars/work".to_string(),
    );
    let cal1_id = db.insert_calendar(&cal1).expect("Failed to insert calendar 1");

    let cal2 = Calendar::new(
        account_id,
        "Personal".to_string(),
        "https://caldav.example.com/calendars/personal".to_string(),
    );
    let cal2_id = db.insert_calendar(&cal2).expect("Failed to insert calendar 2");

    let now = Utc::now();

    let mut event1 = Event::new(
        cal1_id,
        "work-event".to_string(),
        "BEGIN:VEVENT\nEND:VEVENT".to_string(),
    );
    event1.start_time = now;

    let mut event2 = Event::new(
        cal2_id,
        "personal-event".to_string(),
        "BEGIN:VEVENT\nEND:VEVENT".to_string(),
    );
    event2.start_time = now;

    db.upsert_event(&event1).expect("Failed to insert event1");
    db.upsert_event(&event2).expect("Failed to insert event2");

    // Query all events across all calendars
    let events = db.list_events(
        None, // No calendar filter
        now - chrono::Duration::hours(1),
        now + chrono::Duration::hours(1),
    ).expect("Failed to list events");

    assert_eq!(events.len(), 2);
}

#[test]
fn test_delete_event() {
    let db = setup_db();

    let account = Account::new(
        "test_account".to_string(),
        "https://caldav.example.com".to_string(),
        "user@example.com".to_string(),
        "password123".to_string(),
    );
    let account_id = db.insert_account(&account).expect("Failed to insert account");

    let calendar = Calendar::new(
        account_id,
        "Work Calendar".to_string(),
        "https://caldav.example.com/calendars/work".to_string(),
    );
    let calendar_id = db.insert_calendar(&calendar).expect("Failed to insert calendar");

    let event = Event::new(
        calendar_id,
        "event-uid-789".to_string(),
        "BEGIN:VEVENT\nEND:VEVENT".to_string(),
    );

    let id = db.upsert_event(&event).expect("Failed to upsert event");

    db.delete_event(id).expect("Failed to delete event");

    let result = db.get_event(id).expect("Failed to query event");
    assert!(result.is_none(), "Deleted event should not exist");
}

#[test]
fn test_delete_event_by_uid() {
    let db = setup_db();

    let account = Account::new(
        "test_account".to_string(),
        "https://caldav.example.com".to_string(),
        "user@example.com".to_string(),
        "password123".to_string(),
    );
    let account_id = db.insert_account(&account).expect("Failed to insert account");

    let calendar = Calendar::new(
        account_id,
        "Work Calendar".to_string(),
        "https://caldav.example.com/calendars/work".to_string(),
    );
    let calendar_id = db.insert_calendar(&calendar).expect("Failed to insert calendar");

    let event = Event::new(
        calendar_id,
        "event-uid-999".to_string(),
        "BEGIN:VEVENT\nEND:VEVENT".to_string(),
    );

    db.upsert_event(&event).expect("Failed to upsert event");

    db.delete_event_by_uid(calendar_id, "event-uid-999").expect("Failed to delete event by UID");

    let result = db.get_event_by_uid(calendar_id, "event-uid-999")
        .expect("Failed to query event");
    assert!(result.is_none(), "Deleted event should not exist");
}

#[test]
fn test_delete_calendar_cascades_to_events() {
    let db = setup_db();

    let account = Account::new(
        "test_account".to_string(),
        "https://caldav.example.com".to_string(),
        "user@example.com".to_string(),
        "password123".to_string(),
    );
    let account_id = db.insert_account(&account).expect("Failed to insert account");

    let calendar = Calendar::new(
        account_id,
        "Work Calendar".to_string(),
        "https://caldav.example.com/calendars/work".to_string(),
    );
    let calendar_id = db.insert_calendar(&calendar).expect("Failed to insert calendar");

    let event = Event::new(
        calendar_id,
        "event-uid-cascade".to_string(),
        "BEGIN:VEVENT\nEND:VEVENT".to_string(),
    );
    let event_id = db.upsert_event(&event).expect("Failed to upsert event");

    // Delete the calendar
    db.delete_calendar(calendar_id).expect("Failed to delete calendar");

    // Event should also be deleted
    let result = db.get_event(event_id).expect("Failed to query event");
    assert!(result.is_none(), "Event should be deleted when calendar is deleted");
}

#[test]
fn test_event_foreign_key() {
    let db = setup_db();

    // Try to insert event with non-existent calendar_id
    let event = Event::new(
        999, // Non-existent calendar
        "event-uid-bad".to_string(),
        "BEGIN:VEVENT\nEND:VEVENT".to_string(),
    );

    let result = db.upsert_event(&event);
    assert!(result.is_err(), "Event with invalid calendar_id should fail");
}

#[test]
fn test_calendar_unique_account_url() {
    let db = setup_db();

    let account = Account::new(
        "test_account".to_string(),
        "https://caldav.example.com".to_string(),
        "user@example.com".to_string(),
        "password123".to_string(),
    );
    let account_id = db.insert_account(&account).expect("Failed to insert account");

    let cal1 = Calendar::new(
        account_id,
        "Calendar 1".to_string(),
        "https://caldav.example.com/calendars/work".to_string(),
    );

    let cal2 = Calendar::new(
        account_id,
        "Calendar 2".to_string(),
        "https://caldav.example.com/calendars/work".to_string(), // Same URL
    );

    db.insert_calendar(&cal1).expect("Failed to insert first calendar");

    // Second insert should fail due to unique constraint on (account_id, url)
    let result = db.insert_calendar(&cal2);
    assert!(result.is_err(), "Duplicate calendar URL for same account should fail");
}
