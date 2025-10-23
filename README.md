# davd - CalDAV/CardDAV Sync Daemon

A system daemon for Linux that provides centralized CalDAV/CardDAV account management and synchronization. Built in Rust for performance, reliability, and security.

## Status: Phase 1 (MVP) - Complete ✓

Phase 1 provides read-only CalDAV sync with D-Bus API access. See [Roadmap](#roadmap) for future features.

## Features

### Current (Phase 1)
- ✓ CalDAV protocol implementation (RFC 4791)
- ✓ Automatic calendar discovery
- ✓ Event synchronization (read-only)
- ✓ SQLite-backed local storage
- ✓ D-Bus API for calendar access
- ✓ CLI for manual sync and queries
- ✓ Efficient ETag-based change detection
- ✓ Systemd service integration

### Coming Soon (Phase 2+)
- Two-way sync (create, update, delete events)
- CardDAV support for contacts
- Multiple account management
- Automatic periodic sync
- Secret Service credential storage
- Configuration file support
- VTODO (tasks) support

## Installation

### Prerequisites

- Rust 1.70+ (for building)
- SQLite 3.x
- D-Bus

### Build from Source

```bash
git clone https://github.com/EdgarPost/davd.git
cd davd
cargo build --release
```

The binary will be at `target/release/davd`.

### Install

```bash
# Copy binary to PATH
sudo cp target/release/davd /usr/local/bin/

# Install systemd service (optional)
mkdir -p ~/.config/systemd/user
cp systemd/davd.service ~/.config/systemd/user/
systemctl --user enable davd
systemctl --user start davd
```

## Quick Start

### Phase 1 Testing (Hardcoded Credentials)

**⚠ Important**: Phase 1 uses hardcoded credentials for testing. Edit `src/main.rs` before building:

```rust
// Around line 20-22 in src/main.rs
const SERVER_URL: &str = "https://caldav.fastmail.com";
const USERNAME: &str = "your-email@fastmail.com";
const PASSWORD: &str = "your-app-specific-password";
```

Rebuild after editing:
```bash
cargo build --release
```

### Manual Sync

Trigger a one-time sync:

```bash
davd sync
```

Output:
```
2025-10-23T21:30:00Z  INFO davd: Starting sync
2025-10-23T21:30:02Z  INFO davd: Synced 2 calendars, 47 events
```

### Query Events

List events in a date range:

```bash
davd list-events --start 2025-10-01T00:00:00Z --end 2025-10-31T23:59:59Z
```

Output:
```
ID   Start Time           Summary              Location
42   2025-10-15 14:00    Team Meeting         Conference Room A
43   2025-10-20 09:00    Doctor Appointment   Medical Center
```

### Run as Daemon

Start the D-Bus service:

```bash
davd daemon
```

Or use systemd:
```bash
systemctl --user start davd
```

Check status:
```bash
systemctl --user status davd
```

## D-Bus API

davd exposes a D-Bus interface at `org.davd.Calendar` on the session bus.

### Methods

#### `ListEvents(start: String, end: String) -> Vec<Event>`

Query events within a time range.

```bash
# Using busctl
busctl --user call org.davd.Calendar /org/davd/Calendar org.davd.Calendar \
  ListEvents ss "2025-10-01T00:00:00Z" "2025-10-31T23:59:59Z"
```

#### `GetEvent(id: i64) -> Event`

Get a single event by ID.

#### `GetEventIcal(id: i64) -> String`

Get raw iCalendar data for an event.

### Event Structure

```
Event {
  id: i64,
  calendar_id: i64,
  uid: String,
  summary: String,
  description: String,
  location: String,
  start_time: String,  // ISO 8601
  end_time: String,    // ISO 8601
  all_day: bool,
  icalendar_data: String,
}
```

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│                    davd Daemon                          │
│                                                         │
│  CalDAV Client  →  iCalendar Parser  →  SQLite Storage │
│                                             ↓           │
│                                        D-Bus API        │
└────────────────────────────────┬────────────────────────┘
                                 │
                    ┌────────────┼────────────┐
                    │            │            │
              Application    Application  Application
              (calcurse)      (khal)      (custom)
```

### Components

- **CalDAV Client**: HTTP client implementing CalDAV protocol (PROPFIND, REPORT)
- **iCalendar Parser**: RFC 5545 parser for VEVENT components
- **Storage Layer**: SQLite database with schema migrations
- **Sync Engine**: Orchestrates CalDAV → Parser → Storage flow
- **D-Bus Interface**: Session bus API for calendar access
- **CLI**: Command-line interface for manual operations

## Data Storage

davd stores data in `~/.local/share/davd/`:

```
~/.local/share/davd/
└── davd.db          # SQLite database (calendars, events, sync metadata)
```

Database schema includes:
- `accounts` - CalDAV server credentials
- `calendars` - Calendar collections
- `events` - Calendar events with iCalendar data
- Indexes for efficient time-based queries

## Development

### Project Structure

```
davd/
├── src/
│   ├── main.rs              # Entry point, daemon setup
│   ├── lib.rs               # Public library interface
│   ├── cli/                 # CLI commands
│   ├── dbus/                # D-Bus interface
│   ├── ical/                # iCalendar parsing
│   ├── storage/             # SQLite database
│   │   ├── db.rs           # CRUD operations
│   │   ├── models.rs       # Data types
│   │   └── migrations.rs   # Schema versioning
│   └── sync/                # CalDAV synchronization
│       ├── caldav.rs       # CalDAV protocol client
│       └── engine.rs       # Sync orchestration
├── tests/                   # Integration tests
├── systemd/                 # Systemd service files
└── specs/                   # Design documentation
```

### Testing

Run all tests:
```bash
cargo test
```

Run with logging:
```bash
RUST_LOG=davd=debug cargo test -- --nocapture
```

Current test coverage:
- Storage layer: 29 tests
- iCalendar parser: 12 tests
- CalDAV client: 4 tests
- Sync engine: 4 tests
- D-Bus interface: 7 tests
- **Total: 56 tests, 100% passing**

### Design Principles

- **KISS**: Keep It Simple - no premature optimization
- **SOLID**: Single responsibility, proper abstractions
- **TDD**: Test-driven development - tests written first
- **Security**: Credentials in Secret Service (Phase 2), no logging of secrets
- **Documentation**: Explain WHY not HOW

### Building

```bash
# Debug build
cargo build

# Release build (optimized)
cargo build --release

# Check without building
cargo check

# Run clippy linter
cargo clippy
```

## Troubleshooting

### Database locked

If you get "database is locked" errors:

```bash
# Stop daemon
systemctl --user stop davd

# Or kill process
pkill davd
```

### D-Bus service not found

Verify the service is running:

```bash
busctl --user list | grep davd
```

### Check logs

```bash
# If running as systemd service
journalctl --user -u davd -f

# If running manually
RUST_LOG=davd=debug davd daemon
```

## Roadmap

### Phase 1: MVP (Complete) ✓
- [x] Single account support (hardcoded)
- [x] CalDAV calendar sync (read-only)
- [x] Basic SQLite storage
- [x] Simple D-Bus API
- [x] CLI for testing
- [x] Manual sync trigger

### Phase 2: Core Functionality (4-6 weeks)
- [ ] Multiple accounts
- [ ] Two-way sync (create, update, delete)
- [ ] CardDAV support
- [ ] Automatic periodic sync
- [ ] Configuration file
- [ ] Secret Service integration
- [ ] Conflict resolution

### Phase 3: Polish (6-8 weeks)
- [ ] VTODO (tasks) support
- [ ] Push notifications
- [ ] NixOS module
- [ ] Integration examples (calcurse, khal)
- [ ] Performance optimization
- [ ] Migration tools
- [ ] Comprehensive documentation

## Contributing

Contributions are welcome! Please:

1. Fork the repository
2. Create a feature branch
3. Write tests for new functionality
4. Ensure `cargo test` passes
5. Run `cargo clippy` and `cargo fmt`
6. Submit a pull request

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Acknowledgments

- FastMail for CalDAV testing and documentation
- The Rust community for excellent async and HTTP libraries
- RFC 4791 (CalDAV) and RFC 5545 (iCalendar) authors

## Links

- [Design Documentation](specs/design.md)
- [CalDAV RFC 4791](https://tools.ietf.org/html/rfc4791)
- [iCalendar RFC 5545](https://tools.ietf.org/html/rfc5545)
- [FastMail CalDAV](https://www.fastmail.help/hc/en-us/articles/1500000278342)

---

**Status**: Phase 1 MVP complete and ready for testing!

**Maintainer**: davd contributors
**Version**: 0.1.0 (Phase 1)
