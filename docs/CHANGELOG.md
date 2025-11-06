# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Project documentation improvements (CLAUDE.md, AI_INSTRUCTIONS.md, SECURITY_EVALUATION.md)
- TODO.md for tracking proposed improvements
- This CHANGELOG.md file
- Configuration module (`src/config.rs`) with constants for all thresholds and limits
- Automatic data retention/cleanup functionality
- `cleanup_old_data()` function to delete data older than retention period
- `--cleanup` flag to syswriter for manual cleanup
- `--no-cleanup` flag to syswriter to disable automatic cleanup
- Database file permission enforcement (Unix: 0600 - owner read/write only)
- Comprehensive test suite (21 tests across db, collector, and reporter)
- Mermaid diagrams in README (architecture, data flow, database schema)

### Changed
- Reorganized documentation into `docs/` directory
- Replaced all magic numbers with named constants from `config` module
- Fixed unsafe `.unwrap()` calls in database query functions (CRITICAL security fix)
- Database schema now includes `app_version` field in `schema_version` table
- Version number now displayed in all binaries and reports
- Automatic cleanup runs after each syswriter execution (30-day retention by default)
- Improved error handling in timestamp parsing (no longer panics on malformed data)

### Fixed
- Database queries no longer panic on malformed timestamp data
- Database file created with restrictive permissions to protect collected data

### Security
- Implemented proper error handling to prevent crashes from corrupted database data
- Database files now created with 0600 permissions (Unix) to prevent unauthorized access
- Added comprehensive security evaluation document

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

[Unreleased]: https://github.com/johnzastrow/systers/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/johnzastrow/systers/releases/tag/v0.1.0
