# davd - CalDAV/CardDAV Sync Daemon

## Project Overview

**davd** is a system daemon for Linux that provides centralized CalDAV/CardDAV account management and synchronization. It aims to solve the fragmented calendar and contacts experience on Linux by providing a unified, background-syncing service with a D-Bus API that applications can consume.

### Problem Statement

Linux lacks a unified account management layer for calendars and contacts. Unlike macOS (which has EventKit/Contacts frameworks) or even GNOME (which has Evolution Data Server, but only for GNOME apps), most Linux users must manually configure sync tools like vdirsyncer and run them periodically. This creates:

- No real-time synchronization
- Verbose per-application configuration
- No shared account credentials
- No system-level API for calendar/contact access
- Fragmented user experience

### Solution

davd provides:

- System-level daemon that runs in the background
- Automatic bidirectional sync with CalDAV/CardDAV servers
- D-Bus API for applications to query and modify calendar/contact data
- Secure credential storage via Secret Service
- Support for multiple accounts from different providers
- SQLite-backed local storage with efficient queries
- Change notifications to subscribed applications

## Technology Stack

- **Language**: Rust
- **Async Runtime**: tokio
- **HTTP Client**: reqwest
- **XML Parsing**: quick-xml or roxmltree
- **Database**: SQLite via rusqlite or sqlx
- **D-Bus**: zbus
- **Credentials**: secret-service crate
- **iCalendar/vCard**: ical, vobject, or custom parser
- **Logging**: tracing + tracing-subscriber
- **Configuration**: toml or serde-based config

## Project Phases

### Phase 1: Proof of Concept (MVP)

**Goal**: Demonstrate core functionality with minimal features

**Scope**:

- Single FastMail account support (hardcoded for testing)
- CalDAV calendar sync (read-only)
- Basic SQLite storage for events
- Simple D-Bus API for listing events
- CLI for testing/debugging
- Manual sync trigger (no automatic background sync yet)

**Deliverables**:

1. CalDAV client that can authenticate and discover calendars
1. Download events from a single calendar
1. Store events in SQLite
1. Expose basic D-Bus methods:
- `ListEvents(start_time, end_time) -> Vec<Event>`
- `GetEvent(event_id) -> Event`
1. CLI tool to trigger sync and query data
1. Basic systemd service file

**Key Decisions**:

- Choose XML parser
- Design SQLite schema for events
- Define initial D-Bus interface
- Decide on configuration format

**Timeline**: 2-4 weeks (weekends/evenings)

**Success Criteria**:

- Can sync FastMail calendar and list events via D-Bus
- Works as systemd user service
- Basic documentation for setup

-----

### Phase 2: Core Functionality

**Goal**: Make it actually usable for daily workflows

**Scope**:

- Multiple accounts (FastMail, Nextcloud, generic CalDAV)
- Two-way sync (create, update, delete events)
- CardDAV support for contacts
- Automatic periodic sync (configurable interval)
- Improved D-Bus API with full CRUD operations
- Configuration file support
- Better error handling and logging
- Conflict resolution (server wins by default)

**Deliverables**:

1. Account management:
- Add/remove accounts via config file or CLI
- Store credentials in Secret Service (gnome-keyring, KeePassXC)
- Auto-discover CalDAV/CardDAV endpoints
1. Two-way sync:
- Detect local changes
- Upload to server
- Handle ETags for conflict detection
- Basic conflict resolution strategy
1. CardDAV implementation:
- Download contacts
- Store in SQLite
- D-Bus API for contacts
1. Enhanced D-Bus API:
- `CreateEvent(event) -> event_id`
- `UpdateEvent(event_id, event)`
- `DeleteEvent(event_id)`
- `ListContacts() -> Vec<Contact>`
- `GetContact(contact_id) -> Contact`
- Signals for changes: `EventChanged`, `ContactChanged`
1. Configuration:
- TOML config file in `~/.config/davd/config.toml`
- Account definitions
- Sync interval settings
1. Automatic sync loop:
- Background task that syncs periodically
- Respects sync intervals per account
- Exponential backoff on errors
1. Improved CLI:
- `davd accounts list`
- `davd accounts add`
- `davd sync` (manual trigger)
- `davd status`

**Key Decisions**:

- Sync interval strategy (per-account vs. global)
- Conflict resolution policies
- How to handle calendar collection discovery
- Error recovery strategies

**Timeline**: 6-8 weeks

**Success Criteria**:

- Can manage multiple accounts
- Full two-way sync works
- Contacts sync works
- No manual intervention needed (auto-sync)
- Can be used as daily driver for calendar/contacts

-----

### Phase 3: Polish & Ecosystem Integration

**Goal**: Production-ready daemon with broad compatibility

**Scope**:

- VTODO (tasks) support
- Push notification support (if servers support it)
- Improved conflict resolution (user-configurable)
- NixOS module for easy deployment
- Integration examples (calcurse, khal, custom apps)
- Comprehensive documentation
- Test suite
- Performance optimization
- Migration tools (from vdirsyncer, Evolution, etc.)

**Deliverables**:

1. Tasks support:
- VTODO parsing and storage
- D-Bus API for tasks
- Sync with CalDAV task lists
1. Advanced sync:
- Push notifications (CalDAV-push if supported)
- Incremental sync (only fetch changes)
- Bandwidth optimization
1. Conflict resolution:
- User-configurable policies (server wins, client wins, manual)
- Interactive conflict resolution via CLI
- Logging of all conflicts
1. NixOS integration:

   ```nix
   services.davd = {
     enable = true;
     accounts = {
       fastmail = {
         url = "https://caldav.fastmail.com";
         username = "user@fastmail.com";
         passwordFile = "/run/secrets/fastmail-password";
       };
     };
     syncInterval = "15m";
   };
   ```
1. Documentation:
- Architecture overview
- API documentation
- Integration guide for app developers
- User guide for setup and configuration
- Troubleshooting guide
1. Integration examples:
- Patch for calcurse to use davd D-Bus API
- Example Python script using D-Bus
- Example Rust library for davd integration
1. Testing:
- Unit tests for core logic
- Integration tests with mock CalDAV server
- Test against real servers (FastMail, Nextcloud, Radicale)
1. Performance:
- Optimize SQLite queries
- Batch operations
- Lazy loading
- Connection pooling
1. Migration tools:
- Import from vdirsyncer configuration
- Import from Evolution Data Server
- Export to standard formats

**Key Decisions**:

- Conflict resolution UI/UX
- NixOS module options structure
- Backward compatibility guarantees
- Versioning and release strategy

**Timeline**: 8-12 weeks

**Success Criteria**:

- Used by early adopters without major issues
- NixOS users can install with one line of config
- Documentation allows developers to integrate
- Performance is acceptable (sync 1000+ events quickly)
- Ready for broader announcement (Reddit, HN, etc.)

-----

## Architecture

### Components

```
┌─────────────────────────────────────────────────────────────┐
│                    davd Daemon Process                       │
│                                                              │
│  ┌────────────────────────────────────────────────────────┐ │
│  │              Account Manager                           │ │
│  │  - Store account configurations                        │ │
│  │  - Manage credentials via Secret Service              │ │
│  │  - Auto-discover CalDAV/CardDAV endpoints             │ │
│  └────────────────────────────────────────────────────────┘ │
│                                                              │
│  ┌────────────────────────────────────────────────────────┐ │
│  │              Sync Engine                               │ │
│  │  - Background sync loop per account                    │ │
│  │  - CalDAV/CardDAV protocol implementation             │ │
│  │  - Change detection (ETags, sync-tokens)              │ │
│  │  - Conflict resolution                                 │ │
│  │  - Error handling and retry logic                     │ │
│  └────────────────────────────────────────────────────────┘ │
│                                                              │
│  ┌────────────────────────────────────────────────────────┐ │
│  │              Storage Layer (SQLite)                    │ │
│  │  - Events, Todos, Contacts                            │ │
│  │  - Sync metadata (ETags, ctags, sync-tokens)         │ │
│  │  - Change tracking (pending uploads)                  │ │
│  │  - Indexing for efficient queries                     │ │
│  └────────────────────────────────────────────────────────┘ │
│                                                              │
│  ┌────────────────────────────────────────────────────────┐ │
│  │              D-Bus Interface                           │ │
│  │  - org.davd.Calendar                                   │ │
│  │  - org.davd.Contacts                                   │ │
│  │  - org.davd.Tasks                                      │ │
│  │  - org.davd.Accounts                                   │ │
│  └────────────────────────────────────────────────────────┘ │
│                                                              │
└───────────────────────┬──────────────────────────────────────┘
                        │
                        │ D-Bus Session Bus
                        │
        ┌───────────────┼───────────────┬──────────────┐
        │               │               │              │
   ┌────▼────┐    ┌────▼────┐    ┌────▼────┐   ┌────▼────┐
   │ calcurse│    │  khal   │    │ Custom  │   │  GNOME  │
   │         │    │         │    │   App   │   │Calendar │
   └─────────┘    └─────────┘    └─────────┘   └─────────┘
```

### Data Flow

**Sync Flow (Server → Local)**:

1. Sync engine makes CalDAV PROPFIND request
1. Server returns list of events with ETags
1. Compare ETags with local database
1. Download changed events
1. Parse iCalendar data
1. Update SQLite storage
1. Emit D-Bus signals for changed events

**Sync Flow (Local → Server)**:

1. Application modifies event via D-Bus
1. davd marks event as "pending upload" in SQLite
1. Sync engine detects pending changes
1. Serialize event to iCalendar format
1. PUT to CalDAV server
1. Server returns new ETag
1. Update local storage with new ETag
1. Clear "pending upload" flag

### Database Schema (Initial)

```sql
-- Accounts
CREATE TABLE accounts (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    caldav_url TEXT,
    carddav_url TEXT,
    username TEXT NOT NULL,
    last_sync TIMESTAMP,
    sync_token TEXT,
    enabled BOOLEAN DEFAULT TRUE
);

-- Calendars (collections within an account)
CREATE TABLE calendars (
    id INTEGER PRIMARY KEY,
    account_id INTEGER NOT NULL,
    url TEXT NOT NULL,
    display_name TEXT,
    color TEXT,
    sync_token TEXT,
    ctag TEXT,
    FOREIGN KEY (account_id) REFERENCES accounts(id) ON DELETE CASCADE,
    UNIQUE(account_id, url)
);

-- Events
CREATE TABLE events (
    id INTEGER PRIMARY KEY,
    calendar_id INTEGER NOT NULL,
    uid TEXT NOT NULL,
    url TEXT NOT NULL,
    etag TEXT,
    icalendar_data TEXT NOT NULL,
    summary TEXT,
    description TEXT,
    location TEXT,
    start_time TIMESTAMP,
    end_time TIMESTAMP,
    all_day BOOLEAN DEFAULT FALSE,
    recurrence_rule TEXT,
    status TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    pending_upload BOOLEAN DEFAULT FALSE,
    FOREIGN KEY (calendar_id) REFERENCES calendars(id) ON DELETE CASCADE,
    UNIQUE(calendar_id, uid)
);

CREATE INDEX idx_events_time ON events(start_time, end_time);
CREATE INDEX idx_events_calendar ON events(calendar_id);

-- Contacts (similar structure)
CREATE TABLE contacts (
    id INTEGER PRIMARY KEY,
    account_id INTEGER NOT NULL,
    uid TEXT NOT NULL,
    url TEXT NOT NULL,
    etag TEXT,
    vcard_data TEXT NOT NULL,
    full_name TEXT,
    email TEXT,
    phone TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    pending_upload BOOLEAN DEFAULT FALSE,
    FOREIGN KEY (account_id) REFERENCES accounts(id) ON DELETE CASCADE,
    UNIQUE(account_id, uid)
);

CREATE INDEX idx_contacts_name ON contacts(full_name);
CREATE INDEX idx_contacts_email ON contacts(email);
```

### D-Bus API (Initial)

**Interface: org.davd.Calendar**

```
Methods:
  ListEvents(start_time: i64, end_time: i64, calendar_ids: Vec<i32>) -> Vec<Event>
  GetEvent(event_id: i32) -> Event
  CreateEvent(calendar_id: i32, event: Event) -> i32
  UpdateEvent(event_id: i32, event: Event) -> ()
  DeleteEvent(event_id: i32) -> ()
  ListCalendars(account_id: Option<i32>) -> Vec<Calendar>

Signals:
  EventCreated(event_id: i32, calendar_id: i32)
  EventUpdated(event_id: i32, calendar_id: i32)
  EventDeleted(event_id: i32, calendar_id: i32)
  SyncStarted(account_id: i32)
  SyncCompleted(account_id: i32, success: bool)

Types:
  Event {
    id: i32,
    calendar_id: i32,
    uid: String,
    summary: String,
    description: String,
    location: String,
    start_time: i64,
    end_time: i64,
    all_day: bool,
    status: String,
    // ... more fields
  }

  Calendar {
    id: i32,
    account_id: i32,
    display_name: String,
    color: String,
    url: String,
  }
```

**Interface: org.davd.Contacts**

```
Methods:
  ListContacts(account_id: Option<i32>) -> Vec<Contact>
  GetContact(contact_id: i32) -> Contact
  CreateContact(account_id: i32, contact: Contact) -> i32
  UpdateContact(contact_id: i32, contact: Contact) -> ()
  DeleteContact(contact_id: i32) -> ()
  SearchContacts(query: String) -> Vec<Contact>

Signals:
  ContactCreated(contact_id: i32)
  ContactUpdated(contact_id: i32)
  ContactDeleted(contact_id: i32)
```

**Interface: org.davd.Accounts**

```
Methods:
  ListAccounts() -> Vec<Account>
  GetAccount(account_id: i32) -> Account
  AddAccount(account: AccountConfig) -> i32
  RemoveAccount(account_id: i32) -> ()
  SyncAccount(account_id: i32) -> ()
  SyncAllAccounts() -> ()

Signals:
  AccountAdded(account_id: i32)
  AccountRemoved(account_id: i32)

Types:
  Account {
    id: i32,
    name: String,
    username: String,
    caldav_url: String,
    carddav_url: String,
    last_sync: i64,
    enabled: bool,
  }
```

### Configuration Format

```toml
# ~/.config/davd/config.toml

[daemon]
log_level = "info"
sync_interval = "15m"  # Global default
database_path = "~/.local/share/davd/davd.db"

[[accounts]]
name = "fastmail"
username = "user@fastmail.com"
# Password stored in Secret Service, not in config
caldav_url = "https://caldav.fastmail.com"
carddav_url = "https://carddav.fastmail.com"
sync_interval = "10m"  # Override global
enabled = true

[[accounts]]
name = "work"
username = "user@company.com"
caldav_url = "https://caldav.company.com"
enabled = true

[sync]
conflict_resolution = "server_wins"  # server_wins, client_wins, manual
retry_attempts = 3
retry_backoff = "exponential"  # exponential, linear
max_concurrent_syncs = 3

[security]
# Uses system keyring via Secret Service
keyring_service = "davd"
```

## Implementation Priorities

### Phase 1 Focus Areas

1. **CalDAV Client Implementation**
- Start with basic PROPFIND and GET requests
- Focus on FastMail compatibility first
- Implement calendar discovery (well-known URLs)
- Parse iCalendar VEVENT components
1. **SQLite Storage**
- Create initial schema
- Basic CRUD operations
- No optimization yet (premature optimization)
1. **D-Bus Service**
- Implement read-only methods first
- Get familiar with zbus
- Test with busctl or d-feet
1. **Configuration**
- Hardcode for testing initially
- Move to config file in Phase 2

### Phase 2 Focus Areas

1. **Two-way Sync**
- Implement change tracking
- ETag handling
- Conflict detection
- Upload changes to server
1. **Multiple Accounts**
- Account abstraction
- Per-account sync loops
- Credential management via Secret Service
1. **CardDAV**
- Similar to CalDAV but with vCard
- Contacts storage schema
- D-Bus API for contacts

### Phase 3 Focus Areas

1. **Tasks (VTODO)**
- Extend to handle VTODO components
- Task-specific queries and filters
1. **NixOS Module**
- Write clean module with good options
- Integration tests
1. **Documentation**
- Comprehensive docs
- Integration examples

## Development Guidelines

### Code Organization

```
davd/
├── Cargo.toml
├── README.md
├── LICENSE
├── src/
│   ├── main.rs              # Entry point, daemon setup
│   ├── lib.rs               # Public library interface
│   ├── config.rs            # Configuration parsing
│   ├── accounts/
│   │   ├── mod.rs
│   │   └── manager.rs       # Account management
│   ├── sync/
│   │   ├── mod.rs
│   │   ├── engine.rs        # Sync orchestration
│   │   ├── caldav.rs        # CalDAV protocol
│   │   ├── carddav.rs       # CardDAV protocol
│   │   └── conflict.rs      # Conflict resolution
│   ├── storage/
│   │   ├── mod.rs
│   │   ├── db.rs            # SQLite operations
│   │   ├── models.rs        # Data models
│   │   └── migrations.rs    # Schema migrations
│   ├── dbus/
│   │   ├── mod.rs
│   │   ├── calendar.rs      # Calendar D-Bus interface
│   │   ├── contacts.rs      # Contacts D-Bus interface
│   │   └── accounts.rs      # Accounts D-Bus interface
│   ├── ical/
│   │   ├── mod.rs
│   │   ├── parser.rs        # iCalendar parsing
│   │   └── serializer.rs    # iCalendar generation
│   ├── vcard/
│   │   ├── mod.rs
│   │   └── parser.rs        # vCard parsing
│   └── cli/
│       ├── mod.rs
│       └── commands.rs      # CLI subcommands
├── tests/
│   ├── integration/
│   └── fixtures/
└── systemd/
    └── davd.service
```

### Testing Strategy

1. **Unit Tests**
- Test each module in isolation
- Mock external dependencies
- Use cargo test
1. **Integration Tests**
- Test against mock CalDAV server
- Test D-Bus API with actual bus
- Test full sync workflows
1. **Manual Testing**
- Test against real servers: FastMail, Nextcloud, Radicale
- Test with real clients: calcurse, khal, custom scripts
- Test edge cases: network failures, conflicts, large datasets

### Performance Considerations

- Use connection pooling for HTTP requests
- Batch database operations
- Lazy load calendar data (don't load everything at startup)
- Use indexes for common queries
- Profile with `cargo flamegraph` if needed
- Benchmark sync performance with criterion

### Security Considerations

- Never log passwords or tokens
- Use Secret Service for credential storage
- Validate all user input
- Sanitize calendar/contact data before storage
- Use HTTPS for all CalDAV/CardDAV connections
- Verify SSL certificates (allow override for self-signed in config)

## Dependencies (Initial)

```toml
[dependencies]
# Async runtime
tokio = { version = "1", features = ["full"] }

# HTTP client
reqwest = { version = "0.11", features = ["rustls-tls"] }

# XML parsing
quick-xml = "0.31"

# Database
rusqlite = { version = "0.30", features = ["bundled"] }

# D-Bus
zbus = "3"

# Credentials
secret-service = "3"

# Configuration
serde = { version = "1", features = ["derive"] }
toml = "0.8"

# Logging
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }

# CLI
clap = { version = "4", features = ["derive"] }

# Date/time
chrono = "0.4"

# Error handling
anyhow = "1"
thiserror = "1"

# iCalendar (may need to evaluate crates or roll our own)
# ical = "0.8"  # Evaluate this

[dev-dependencies]
mockall = "0.12"
tempfile = "3"
```

## Milestones & Checkpoints

### Phase 1 Milestones

- [ ] M1.1: CalDAV client can authenticate and discover calendars
- [ ] M1.2: Can download events from FastMail
- [ ] M1.3: Events stored in SQLite
- [ ] M1.4: D-Bus service starts and responds to ListEvents
- [ ] M1.5: CLI can trigger sync and display events
- [ ] M1.6: Works as systemd user service
- [ ] M1.7: Basic documentation written

### Phase 2 Milestones

- [ ] M2.1: Multiple accounts configured and syncing
- [ ] M2.2: Two-way sync working (create/update/delete)
- [ ] M2.3: CardDAV implementation complete
- [ ] M2.4: Automatic periodic sync working
- [ ] M2.5: Conflict resolution implemented
- [ ] M2.6: Secret Service integration for credentials
- [ ] M2.7: Configuration file parsing working
- [ ] M2.8: Can be used as daily driver (dogfooding)

### Phase 3 Milestones

- [ ] M3.1: Tasks (VTODO) support complete
- [ ] M3.2: NixOS module working
- [ ] M3.3: Integration example with calcurse/khal
- [ ] M3.4: Comprehensive documentation published
- [ ] M3.5: Test suite with good coverage
- [ ] M3.6: Performance optimization complete
- [ ] M3.7: Migration tools from vdirsyncer/Evolution
- [ ] M3.8: Ready for public announcement

## Resources & References

### CalDAV/CardDAV Specifications

- RFC 4791: CalDAV
- RFC 6352: CardDAV
- RFC 5545: iCalendar
- RFC 6350: vCard
- RFC 6578: Collection Synchronization for WebDAV

### Existing Implementations to Study

- **vdirsyncer** (Python): https://github.com/pimutils/vdirsyncer
- **Radicale** (Python CalDAV/CardDAV server): https://github.com/Kozea/Radicale
- **Evolution Data Server** (C): https://gitlab.gnome.org/GNOME/evolution-data-server
- **DAVx⁵** (Android): https://github.com/bitfireAT/davx5-ose

### Rust Crates to Evaluate

- **ical** - iCalendar parser (evaluate if sufficient)
- **vobject** - vCard parser
- **caldav** - Existing CalDAV crate (basic, may need to fork)
- **zbus** - D-Bus library (well-maintained)
- **secret-service** - Secret Service integration

### Testing Resources

- FastMail CalDAV endpoint: https://www.fastmail.help/hc/en-us/articles/1500000278342
- Nextcloud test instances
- Radicale (easy to self-host for testing)

### Community & Discussion

- Reddit: /r/linux, /r/rust, /r/NixOS
- Matrix/IRC: #pim:matrix.org, #nixos
- GitHub Discussions once repo is created

## Success Metrics

### Phase 1 Success

- Successfully syncs one FastMail calendar
- Can list events via D-Bus
- Runs as systemd service without crashing

### Phase 2 Success

- Author uses it daily (dogfooding)
- No manual vdirsyncer invocations needed
- Handles at least 3 different accounts
- Zero data loss during testing period

### Phase 3 Success

- At least 10 early adopters using it
- Packaged in nixpkgs (or NUR)
- Positive feedback from NixOS community
- Documentation enables others to integrate
- No critical bugs reported

### Long-term Success

- Becomes default recommendation for CalDAV/CardDAV on Linux
- Other apps integrate with davd API
- Packaged in major distributions (Arch AUR, Debian, etc.)
- Active community contributions

## Contributing Guidelines (Future)

Once the project is public:

1. **Issues**: Use GitHub issues for bugs and feature requests
1. **Pull Requests**: Welcome, but discuss major changes first
1. **Code Style**: Use rustfmt and clippy
1. **Testing**: All PRs must include tests
1. **Documentation**: Update docs for any API changes

## License

**Recommendation**: MIT or Apache-2.0 (or dual-licensed)

Rationale:

- Permissive licenses encourage adoption
- Compatible with inclusion in nixpkgs
- Allows commercial use (helps adoption)
- MIT is simple and well-understood
- Apache-2.0 provides patent protection

## Next Steps

1. **Set up project structure**

   ```bash
   cargo new davd
   cd davd
   # Set up initial directory structure
   # Add dependencies to Cargo.toml
   # Create systemd service file
   ```
1. **Start with Phase 1, Milestone 1.1**
- Implement basic CalDAV authentication
- Test against FastMail
- Get calendar discovery working
1. **Document as you go**
- Keep notes on decisions
- Document pain points
- Track time spent
1. **Regular check-ins**
- Weekly review of progress
- Adjust timeline as needed
- Decide when to move between phases
1. **Prepare for public release**
- Create GitHub repo (when ready)
- Write good README
- Announce to relevant communities

-----

## Notes for AI Coding Assistants

When implementing this project:

1. **Start small**: Don't try to implement everything at once. Follow the phases.
1. **Focus on correctness first**: Don't optimize prematurely. Get it working, then make it fast.
1. **Test incrementally**: Write tests as you go, not at the end.
1. **Read the RFCs**: CalDAV/CardDAV have nuances. Don't guess—read the specs.
1. **Handle errors gracefully**: Network errors, parse errors, database errors—all should be handled properly.
1. **Log extensively**: Use tracing for debugging. It's invaluable for a daemon.
1. **Keep configuration simple**: Start with TOML. Don't over-engineer.
1. **D-Bus is your friend**: zbus makes it easy. Study the examples.
1. **SQLite is plenty**: Don't need PostgreSQL for this. SQLite is perfect.
1. **Security matters**: This daemon handles credentials. Be careful.

## Questions to Answer During Development

- Should we support OAuth in addition to basic auth?
- How to handle recurrence rules (RRULE) properly?
- Should we support calendar sharing/delegation?
- What's the strategy for schema migrations?
- How to handle timezone complexity?
- Should there be a GUI configuration tool eventually?
- What's the upgrade path between versions?
- How to handle calendar attachments?
- Should we support CalDAV scheduling (invites/responses)?

-----

**Document Version**: 1.0
**Last Updated**: 2025-10-23
**Author**: Initial specification for davd project
**Status**: Planning / Pre-development
