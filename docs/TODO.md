# TODO - Systers Improvements

This document tracks proposed improvements and enhancements for the Systers project.


## Completed Items

### ✅ 1. Fix unsafe `.unwrap()` calls in database queries
**Status:** Completed in v0.2.0
Fixed timestamp parsing to use proper error handling instead of `.unwrap()`.

### ✅ 2. Implement data retention policy
**Status:** Completed in v0.2.0
Added `cleanup_old_data()` function with configurable retention period.

### ✅ 3. Replace magic numbers with named constants
**Status:** Completed in v0.2.0
Created `src/config.rs` with all configuration constants.

### ✅ 4. Improve log parsing accuracy
**Status:** Completed in v0.3.0 (unreleased)
Implemented regex-based pattern matching with timestamp extraction and false positive reduction.

### ✅ 5. Add test coverage
**Status:** Completed in v0.2.0
Added 22 comprehensive tests across all modules.

### ✅ 6. Eliminate database query code duplication
**Status:** Completed in v0.3.0 (unreleased)
Consolidated `query_logs()` function to eliminate duplicated code.

### ✅ 7. Add structured logging
**Status:** Completed in v0.3.0 (unreleased)
Replaced print statements with `log` crate, configurable via `RUST_LOG`.

### ✅ 8. Improve command-line argument parsing
**Status:** Completed in v0.2.0
Implemented professional CLI with `clap` crate.

---

## High Priority

(All high-priority items have been completed - see "Completed Items" section above)

## Medium Priority

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

## Notes

- Priority levels are subjective and can be adjusted based on user needs
- Some items (like #11 database schema changes) require careful migration planning
- Consider creating GitHub issues for tracking individual items
