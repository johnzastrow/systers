# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Systers is a system monitoring and analysis tool for Linux hosts. It consists of two complementary Rust binaries:

1. **syswriter** - Collects system metrics (CPU, memory, disk, load averages, process counts) and scans system logs, storing data in SQLite
2. **sysreport** - Analyzes collected data and generates formatted terminal reports with issue detection and recommendations

Data is stored in a SQLite database (default: `~/.systers.db`, override with `SYSTERS_DB_PATH` environment variable).

## Development Commands

### Building
```bash
# Debug build
cargo build

# Release build (recommended for production)
cargo build --release

# Binaries output to target/{debug,release}/syswriter and target/{debug,release}/sysreport
```

### Testing
```bash
# Run all tests
cargo test

# Run tests with output
cargo test -- --nocapture
```

### Linting and Formatting
```bash
# Run clippy for linting
cargo clippy

# Format code
cargo fmt

# Check formatting without modifying files
cargo fmt -- --check
```

### Running During Development
```bash
# Run syswriter directly
cargo run --bin syswriter

# Run sysreport with arguments
cargo run --bin sysreport -- --hours 48

# Run with custom database path
SYSTERS_DB_PATH=/tmp/test.db cargo run --bin syswriter
```

## Architecture

### Module Structure

- **src/lib.rs** - Library root that exports three modules
- **src/db.rs** - Database layer (schema, CRUD operations, queries)
- **src/collector.rs** - System metrics and log collection using `sysinfo` crate
- **src/reporter.rs** - Report generation and formatting logic
- **src/bin/syswriter.rs** - Data collection binary entry point
- **src/bin/sysreport.rs** - Report generation binary entry point

### Data Flow

1. **syswriter** execution:
   - Collects system metrics via `collector::collect_system_metrics()` (uses `sysinfo` crate)
   - Scans logs via `collector::scan_system_logs()` (reads `/var/log/{syslog,messages,kern.log,auth.log}`)
   - Stores data via `db::insert_metrics()` and `db::insert_log_entry()`

2. **sysreport** execution:
   - Queries data via `db::query_metrics()` and `db::query_logs()`
   - Analyzes via `reporter::generate_report()` (calculates statistics, identifies issues)
   - Formats output via `reporter::format_report()`

### Database Schema

The SQLite database contains three tables:
- **schema_version** - Tracks schema version (current: 1)
- **system_metrics** - Stores timestamped metrics (CPU, memory, disk, load, process count)
- **log_entries** - Stores notable log entries (ERROR, WARNING, CRITICAL levels only)

Indices exist on `timestamp` fields and `log_entries.level` for query performance.

### Key Types

- `SystemMetrics` (db.rs:11) - Struct representing a snapshot of system state
- `LogEntry` (db.rs:27) - Struct representing a log entry with level, source, message
- `MetricsReport` (reporter.rs:8) - Aggregated statistics and issues for a time period
- `LogReport` (reporter.rs:24) - Log entry counts and recent errors

## Permissions

- **syswriter** typically needs root/sudo to read system log files in `/var/log/`
- Normal user permissions sufficient for system metrics collection
- Database file needs read/write permissions for both binaries

## Dependencies

Core dependencies (see Cargo.toml):
- `rusqlite` - SQLite database interface (with bundled feature)
- `chrono` - Date/time handling
- `sysinfo` - Cross-platform system information gathering
- `anyhow` - Error handling

## Testing Strategy

No test suite currently exists. When adding tests:
- Unit tests for individual functions in `db`, `collector`, `reporter` modules
- Integration tests for end-to-end binary execution
- Consider using an in-memory SQLite database (`:memory:`) for test isolation
- Mock system metrics collection for deterministic tests
