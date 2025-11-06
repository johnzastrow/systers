# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.3.0] - 2025-11-06

### Added
- Structured logging with `log` and `env_logger` crates
- Regex-based log pattern matching for better accuracy
- Timestamp extraction from log entries (ISO 8601 format support)
- False positive reduction for error detection (skips "error=0", "no error", etc.)
- Configurable log file paths via `--log-paths` CLI flag or `SYSTERS_LOG_PATHS` env var
- Database schema v2 with INTEGER timestamps (Unix epoch) for better performance
- Automatic migration from schema v1 (TEXT) to v2 (INTEGER timestamps)
- Improved error messages with actionable hints and guidance

### Changed
- Replaced `println!`/`eprintln!` with structured logging macros (`info!`, `warn!`, `debug!`, `error!`)
- Log parsing now uses compiled regex patterns (cached with `OnceLock`)
- Consolidated database query logic to eliminate code duplication in `query_logs()`
- Log timestamps now extracted from log lines when available
- Improved pattern matching: checks for word boundaries to reduce false positives
- Database schema optimized: timestamps now stored as INTEGER instead of TEXT
  - Faster queries (numeric comparison vs string parsing)
  - Less storage space (8 bytes vs ~24 bytes per timestamp)
  - Better index performance
- Error messages now provide step-by-step guidance
- Permission denied errors now suggest running with sudo or adding user to 'adm' group

### Dependencies
- log (0.4) - Structured logging facade
- env_logger (0.11) - Configurable logging implementation
- regex (1.12) - Regular expression support for log parsing

## [0.2.0] - 2025-11-06

### Added
- Project documentation improvements (CLAUDE.md, AI_INSTRUCTIONS.md, SECURITY_EVALUATION.md)
- TODO.md for tracking proposed improvements
- This CHANGELOG.md file
- Configuration module (`src/config.rs`) with constants for all thresholds and limits
- Automatic data retention/cleanup functionality
- `cleanup_old_data()` function to delete data older than retention period
- Database file permission enforcement (Unix: 0600 - owner read/write only)
- Comprehensive test suite (22 tests across db, collector, and reporter)
- Mermaid diagrams in README (architecture, data flow, database schema)
- Automatic database schema migration for existing databases
- Professional CLI with `clap` crate for both binaries
  - Better help messages with `-h/--help`
  - Version display with `-V/--version`
  - Type validation for all arguments
  - `--cleanup` flag to syswriter for manual cleanup
  - `--no-cleanup` flag to syswriter to disable automatic cleanup
  - `--retention-days <DAYS>` to customize retention period
  - `--db-path <PATH>` to override database location
  - `--hours <N>` for sysreport to specify time range

### Changed
- Reorganized documentation into `docs/` directory
- Replaced all magic numbers with named constants from `config` module
- Fixed unsafe `.unwrap()` calls in database query functions (CRITICAL security fix)
- Database schema now includes `app_version` field in `schema_version` table
- Version number now displayed in all binaries and reports
- Automatic cleanup runs after each syswriter execution (30-day retention by default)
- Improved error handling in timestamp parsing (no longer panics on malformed data)
- All timestamps now displayed in local time instead of UTC
- Log entries now show the source log file path (e.g., `/var/log/syslog`)
- Report output now includes timestamp and source for each log entry

### Fixed
- Database queries no longer panic on malformed timestamp data
- Database file created with restrictive permissions to protect collected data

### Security
- Implemented proper error handling to prevent crashes from corrupted database data
- Database files now created with 0600 permissions (Unix) to prevent unauthorized access
- Added comprehensive security evaluation document

### Dependencies
- clap (4.5) - Command-line argument parsing with derive macros

## [0.1.0] - 2025-11-05

### Added
- Initial implementation of `syswriter` binary for system metrics collection
- Initial implementation of `sysreport` binary for generating analysis reports
- SQLite database backend for storing metrics and log entries
- System metrics collection: CPU usage, memory usage, disk usage, load averages, process counts
- Log scanning and analysis from common Linux system logs
- Issue detection for high resource usage
- Actionable recommendations in reports
- Database schema with versioning support
- Time-range queries for metrics and logs
- Formatted terminal output for reports
- REQUIREMENTS.md documenting project requirements
- Support for `SYSTERS_DB_PATH` environment variable
- Command-line argument support for `sysreport` (`--hours` flag)

### Dependencies
- rusqlite (0.31) - SQLite database interface
- chrono (0.4) - Date and time handling
- sysinfo (0.30) - System information gathering
- anyhow (1.0) - Error handling

## [0.0.0] - Initial Planning

### Added
- Initial project structure
- Planning documentation

---

## Version History Format

### Types of Changes
- **Added** for new features
- **Changed** for changes in existing functionality
- **Deprecated** for soon-to-be removed features
- **Removed** for now removed features
- **Fixed** for any bug fixes
- **Security** for vulnerability fixes

[Unreleased]: https://github.com/johnzastrow/systers/compare/v0.3.0...HEAD
[0.3.0]: https://github.com/johnzastrow/systers/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/johnzastrow/systers/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/johnzastrow/systers/releases/tag/v0.1.0
