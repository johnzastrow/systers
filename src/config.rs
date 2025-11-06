/// Configuration constants for Systers
///
/// This module contains all configurable thresholds and limits used throughout
/// the application. These values control issue detection, data collection,
/// and system behavior.

// Issue Detection Thresholds
/// CPU usage percentage threshold for warning alerts
pub const CPU_WARNING_THRESHOLD: f32 = 90.0;

/// Memory usage percentage threshold for warning alerts
pub const MEMORY_WARNING_THRESHOLD: f32 = 90.0;

/// Disk usage percentage threshold for warning alerts
pub const DISK_WARNING_THRESHOLD: f32 = 85.0;

/// System load average threshold for warning alerts
pub const LOAD_WARNING_THRESHOLD: f64 = 5.0;

// Log Collection Limits
/// Maximum number of log lines to read from each log file
pub const MAX_LOG_LINES_PER_FILE: usize = 1000;

/// Maximum number of recent error messages to display in reports
pub const MAX_RECENT_ERRORS_DISPLAY: usize = 10;

// System Metrics Collection
/// Delay in milliseconds before collecting CPU usage to ensure accurate measurement
/// The sysinfo crate needs at least one refresh cycle to compute CPU percentage
pub const CPU_MEASUREMENT_DELAY_MS: u64 = 200;

// Report Generation
/// Minimum number of errors before triggering a "multiple errors" recommendation
pub const ERROR_COUNT_THRESHOLD: usize = 10;

// Data Retention (for future implementation)
/// Default number of days to retain historical data
pub const DEFAULT_RETENTION_DAYS: i64 = 30;
