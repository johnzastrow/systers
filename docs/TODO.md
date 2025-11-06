# TODO - Systers Improvements

This document tracks proposed improvements and enhancements for the Systers project.

## Critical Issues

### 1. Fix unsafe `.unwrap()` calls in database queries
**Location:** `src/db.rs:164`, `src/db.rs:204`

**Issue:** Timestamp parsing uses `.unwrap()` which will panic if the database contains invalid data.

**Fix:**
```rust
let timestamp = DateTime::parse_from_rfc3339(&timestamp_str)
    .context("Invalid timestamp in database")?
    .with_timezone(&Utc);
```

**Impact:** Prevents crashes when reading corrupted database entries.

---

### 2. Implement data retention policy
**Location:** New functionality needed across database and syswriter

**Issue:** Database will grow indefinitely without cleanup mechanism.

**Proposed Solution:**
- Add a cleanup function to remove data older than X days (configurable)
- Add `--cleanup` or `--max-days` flag to syswriter
- Optionally run automatic cleanup during data collection
- Add configuration for retention period (default: 30 days?)

**Impact:** Prevents unbounded disk usage growth.

---

## High Priority

### 3. Replace magic numbers with named constants
**Location:** Throughout codebase (reporter.rs, collector.rs)

**Issue:** Hardcoded thresholds (90%, 85%), limits (1000, 10), and timing (200ms) make configuration difficult.

**Proposed Solution:**
Create `src/config.rs`:
```rust
pub const CPU_WARNING_THRESHOLD: f32 = 90.0;
pub const MEMORY_WARNING_THRESHOLD: f32 = 90.0;
pub const DISK_WARNING_THRESHOLD: f32 = 85.0;
pub const LOAD_WARNING_THRESHOLD: f64 = 5.0;
pub const MAX_LOG_LINES_PER_FILE: usize = 1000;
pub const MAX_RECENT_ERRORS_DISPLAY: usize = 10;
pub const CPU_MEASUREMENT_DELAY_MS: u64 = 200;
```

Later, these could be loaded from a configuration file.

---

### 4. Improve log parsing accuracy
**Location:** `src/collector.rs:71-81`

**Issues:**
- Simple `.contains()` matching generates false positives (e.g., "error" in URLs)
- Doesn't handle structured logs (JSON, systemd journal)
- Uses collection timestamp instead of actual log entry timestamp
- Misses log format variations

**Proposed Solutions:**
- Use regex patterns for better matching
- Add systemd journal integration (`journalctl` support)
- Parse log timestamps from entries
- Make log patterns configurable
- Add support for common log formats (syslog, Apache, nginx)

---

### 5. Add test coverage
**Location:** New `tests/` directory

**Current State:** No tests exist

**Proposed Tests:**
- Unit tests for percentage calculations in `reporter.rs`
- Unit tests for metric collection with mocked sysinfo
- Integration tests with in-memory SQLite database (`:memory:`)
- Tests for log parsing edge cases (empty lines, malformed entries)
- Tests for database query functions
- End-to-end tests for both binaries

**Priority Items:**
- Database operations (CRUD, queries)
- Report generation and formatting
- Log parsing patterns

---

### 6. Eliminate database query code duplication
**Location:** `src/db.rs:192-249`

**Issue:** The `query_logs` function duplicates nearly identical code for filtered vs unfiltered queries.

**Fix:** Consolidate query logic:
```rust
pub fn query_logs(
    conn: &Connection,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
    level_filter: Option<&str>,
) -> Result<Vec<LogEntry>> {
    let (query, params): (&str, Vec<Box<dyn rusqlite::ToSql>>) = if let Some(level) = level_filter {
        (
            "SELECT timestamp, level, source, message
             FROM log_entries
             WHERE timestamp >= ?1 AND timestamp <= ?2 AND level = ?3
             ORDER BY timestamp DESC",
            vec![Box::new(start.to_rfc3339()), Box::new(end.to_rfc3339()), Box::new(level.to_string())]
        )
    } else {
        (
            "SELECT timestamp, level, source, message
             FROM log_entries
             WHERE timestamp >= ?1 AND timestamp <= ?2
             ORDER BY timestamp DESC",
            vec![Box::new(start.to_rfc3339()), Box::new(end.to_rfc3339())]
        )
    };
    // ... rest of implementation
}
```

---

## Medium Priority

### 7. Add structured logging
**Location:** Replace all `println!` and `eprintln!` calls

**Issue:** Current logging uses print statements with no level control or filtering.

**Proposed Solution:**
- Add `log` crate dependency
- Add `env_logger` for runtime configuration
- Replace print statements with `info!()`, `warn!()`, `error!()`, `debug!()`
- Allow users to control verbosity with `RUST_LOG` environment variable

---

### 8. Improve command-line argument parsing
**Location:** `src/bin/sysreport.rs:39-62`, potentially `src/bin/syswriter.rs`

**Issue:** Manual argument parsing is error-prone and provides poor UX.

**Proposed Solution:**
Add `clap` crate for:
- Automatic help generation
- Type validation
- Better error messages
- Subcommands support (e.g., `sysreport show`, `sysreport export`, `sysreport clean`)
- Shell completion generation

Example:
```rust
use clap::Parser;

#[derive(Parser)]
#[command(name = "sysreport")]
#[command(about = "System Analysis Report Generator")]
struct Args {
    /// Number of hours to look back
    #[arg(long, default_value_t = 24)]
    hours: i64,

    /// Output format (text, json, html)
    #[arg(long, default_value = "text")]
    format: String,
}
```

---

### 9. Make log file paths configurable
**Location:** `src/collector.rs:101-106`

**Issue:** Hardcoded log paths don't work for all systems or use cases.

**Proposed Solutions:**
- Environment variable: `SYSTERS_LOG_PATHS` (colon-separated)
- Configuration file support
- Command-line argument to syswriter
- Support for custom application logs
- Distribution-specific defaults (RHEL vs Debian)

---

### 10. Add report export functionality
**Location:** New feature in `src/reporter.rs` and `src/bin/sysreport.rs`

**Context:** REQUIREMENTS.md mentions email delivery and file export.

**Proposed Features:**
- `--output <file>` flag to save reports to file
- `--format json` for machine-readable output
- `--format html` for web viewing
- `--format csv` for spreadsheet import
- Email delivery via SMTP (configurable)
- Webhook support for integration with monitoring systems

---

## Low Priority

### 11. Optimize database schema
**Location:** `src/db.rs:40-74`

**Current Issues:**
- Timestamps stored as TEXT (RFC3339) instead of INTEGER (Unix timestamp)
  - Slower queries
  - More storage space
  - More parsing overhead
- No hostname field for multi-host monitoring (future REQUIREMENTS.md feature)

**Proposed Changes:**
- Store timestamps as INTEGER (Unix epoch)
- Add `hostname` TEXT field to both tables
- Add database migration support for schema changes
- Bump `SCHEMA_VERSION` to 2

**Note:** Requires migration logic for existing databases.

---

### 12. Document CPU collection delay
**Location:** `src/collector.rs:13`

**Issue:** The 200ms sleep is necessary but not explained.

**Fix:** Add documentation:
```rust
// Sleep briefly to allow sysinfo to calculate accurate CPU usage.
// The sysinfo crate needs at least one refresh cycle to compute
// CPU percentage. This delay ensures we get meaningful data rather
// than 0% on the first collection.
std::thread::sleep(std::time::Duration::from_millis(200));
```

**Alternative:** Investigate using sysinfo's async capabilities or two-sample collection.

---

### 13. Improve error messages
**Location:** Throughout, especially `src/bin/syswriter.rs:64`, `src/collector.rs:112`

**Issue:** Error messages don't provide actionable guidance.

**Examples:**

Current:
```
Warning: Could not read /var/log/syslog: Permission denied
```

Better:
```
Warning: Could not read /var/log/syslog: Permission denied
  Hint: Try running with sudo, or add your user to the 'adm' group:
        sudo usermod -a -G adm $USER
```

Current:
```
Error: Database not found at /home/user/.systers.db
Run 'syswriter' first to collect system data
```

Better:
```
Error: Database not found at /home/user/.systers.db
  Cause: No data has been collected yet
  Solution: Run 'syswriter' to start collecting system metrics

  You can also specify a different database location:
    SYSTERS_DB_PATH=/path/to/db.db sysreport
```

---

### 14. Security considerations
**Location:** Documentation and potentially `src/collector.rs`

**Issues:**
- Log messages in database might contain sensitive information (passwords in error messages, API tokens, etc.)
- Database file permissions not explicitly set
- No sanitization of collected data

**Proposed Actions:**
- Add warning in README about sensitive data in logs
- Consider adding optional log message sanitization (regex patterns for common secrets)
- Set restrictive permissions on database file (0600)
- Add security section to documentation
- Optionally support encrypted database (SQLCipher)

**Documentation needed:**
```markdown
## Security Considerations

- **Sensitive Data**: Log entries may contain passwords, tokens, or other
  sensitive information from application error messages. The database file
  should be protected with appropriate file permissions.

- **Database Permissions**: Ensure `~/.systers.db` has restrictive permissions:
  ```bash
  chmod 600 ~/.systers.db
  ```

- **Log Access**: syswriter requires read access to system logs, which may
  contain sensitive information. Run with minimum necessary privileges.
```

---

## Future Enhancements

Items from REQUIREMENTS.md not yet implemented:

- [ ] Web dashboard for viewing reports
- [ ] Integration with monitoring systems (Prometheus, Grafana)
- [ ] Custom plugin support for additional checks
- [ ] Multi-host monitoring from central location
- [ ] Configuration file support (TOML/YAML)
- [ ] Configurable log rotation
- [ ] Real-time alerting capabilities
- [ ] Container and Kubernetes monitoring
- [ ] Cloud platform integration (AWS, Azure, GCP)

---

## Completed Items

(This section will track completed improvements)

---

## Notes

- Priority levels are subjective and can be adjusted based on user needs
- Some items (like #11 database schema changes) require careful migration planning
- Consider creating GitHub issues for tracking individual items
