# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Project documentation improvements (CLAUDE.md, AI_INSTRUCTIONS.md)
- TODO.md for tracking proposed improvements
- This CHANGELOG.md file

### Changed
- Reorganized documentation into `docs/` directory

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
