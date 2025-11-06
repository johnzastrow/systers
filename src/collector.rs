use crate::config::{CPU_MEASUREMENT_DELAY_MS, MAX_LOG_LINES_PER_FILE};
use crate::db::{LogEntry, SystemMetrics};
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use log::warn;
use regex::Regex;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::sync::OnceLock;
use sysinfo::{Disks, System};

/// Collect current system metrics
pub fn collect_system_metrics() -> Result<SystemMetrics> {
    let mut sys = System::new_all();

    // Refresh to get accurate data
    // Sleep to allow sysinfo to calculate accurate CPU usage (needs one refresh cycle)
    std::thread::sleep(std::time::Duration::from_millis(CPU_MEASUREMENT_DELAY_MS));
    sys.refresh_cpu();
    sys.refresh_memory();

    // Get CPU usage (average across all cores)
    let cpu_usage = sys.global_cpu_info().cpu_usage();

    // Get memory info (in bytes)
    let memory_total = sys.total_memory();
    let memory_used = sys.used_memory();
    let memory_available = sys.available_memory();

    // Get disk info (sum across all disks)
    let disks = Disks::new_with_refreshed_list();
    let mut disk_total = 0u64;
    let mut disk_used = 0u64;

    for disk in &disks {
        disk_total += disk.total_space();
        disk_used += disk.total_space().saturating_sub(disk.available_space());
    }

    // Get process count
    let process_count = sys.processes().len();

    // Get load averages
    let load_avg = System::load_average();

    Ok(SystemMetrics {
        timestamp: Utc::now(),
        cpu_usage,
        memory_total,
        memory_used,
        memory_available,
        disk_total,
        disk_used,
        process_count,
        load_avg_1min: load_avg.one,
        load_avg_5min: load_avg.five,
        load_avg_15min: load_avg.fifteen,
    })
}

/// Get compiled regex patterns (cached using OnceLock)
fn get_log_patterns() -> &'static [(&'static str, Regex)] {
    static PATTERNS: OnceLock<Vec<(&'static str, Regex)>> = OnceLock::new();
    PATTERNS.get_or_init(|| {
        vec![
            // Critical patterns (kernel panic, segfault, critical errors, etc.)
            ("CRITICAL", Regex::new(r"(?i)\b(critical|panic|segfault|kernel:.*bug|out of memory|oom[-_]killer)\b").unwrap()),
            // Error patterns (more specific to avoid false positives)
            ("ERROR", Regex::new(r"(?i)\b(error|failed|failure|fatal|cannot)\b").unwrap()),
            // Warning patterns
            ("WARNING", Regex::new(r"(?i)\b(warn|warning|deprecated)\b").unwrap()),
        ]
    })
}

/// Try to extract timestamp from log line (supports common formats)
fn extract_timestamp(line: &str) -> Option<DateTime<Utc>> {
    static TIMESTAMP_PATTERN: OnceLock<Regex> = OnceLock::new();
    let pattern = TIMESTAMP_PATTERN.get_or_init(|| {
        // Matches ISO 8601 timestamps like 2025-11-05T20:00:01.529-0500
        Regex::new(r"(\d{4}-\d{2}-\d{2}[T ]\d{2}:\d{2}:\d{2}(?:\.\d+)?(?:[+-]\d{2}:?\d{2}|Z)?)").unwrap()
    });

    if let Some(cap) = pattern.captures(line) {
        if let Some(ts_match) = cap.get(1) {
            // Try to parse the timestamp
            if let Ok(dt) = DateTime::parse_from_rfc3339(ts_match.as_str()) {
                return Some(dt.with_timezone(&Utc));
            }
            // Try with space instead of 'T'
            let ts_str = ts_match.as_str().replace(' ', "T");
            if let Ok(dt) = DateTime::parse_from_rfc3339(&ts_str) {
                return Some(dt.with_timezone(&Utc));
            }
        }
    }
    None
}

/// Parse system log file for errors and warnings with improved pattern matching
pub fn collect_log_entries<P: AsRef<Path>>(
    log_path: P,
    max_entries: usize,
) -> Result<Vec<LogEntry>> {
    let log_path_ref = log_path.as_ref();
    let file = File::open(log_path_ref).context("Failed to open log file")?;
    let reader = BufReader::new(file);

    let mut entries = Vec::new();
    let fallback_timestamp = Utc::now();

    // Use the log file path as the source
    let source = log_path_ref.to_string_lossy().to_string();

    let patterns = get_log_patterns();

    // Parse log file using regex patterns
    for line in reader.lines().take(max_entries) {
        let line = line?;
        let line_lower = line.to_lowercase();

        // Skip empty lines and lines that are just URLs/paths
        if line.trim().is_empty() || line.starts_with("http://") || line.starts_with("https://") {
            continue;
        }

        // Check each pattern in priority order (CRITICAL, ERROR, WARNING)
        let mut matched_level = None;
        for (level, pattern) in patterns {
            if pattern.is_match(&line_lower) {
                // Additional check to reduce false positives for ERROR level
                if *level == "ERROR" {
                    // Skip if it's just mentioning error in a benign context
                    if line_lower.contains("error=0")
                        || line_lower.contains("errors=0")
                        || line_lower.contains("no error")
                        || line_lower.contains("without error") {
                        continue;
                    }
                }
                matched_level = Some(level);
                break;
            }
        }

        if let Some(level) = matched_level {
            // Try to extract timestamp from the log line
            let timestamp = extract_timestamp(&line).unwrap_or(fallback_timestamp);

            entries.push(LogEntry {
                timestamp,
                level: level.to_string(),
                source: source.clone(),
                message: line.trim().to_string(),
            });
        }
    }

    Ok(entries)
}

/// Get default log file paths for common Linux distributions
fn get_default_log_paths() -> Vec<&'static str> {
    vec![
        "/var/log/syslog",
        "/var/log/messages",
        "/var/log/kern.log",
        "/var/log/auth.log",
    ]
}

/// Scan log files for issues
/// If custom_paths is None, uses default Linux log locations
pub fn scan_system_logs_with_paths<P: AsRef<Path>>(
    custom_paths: Option<&[P]>,
) -> Result<Vec<LogEntry>> {
    let mut all_entries = Vec::new();

    if let Some(paths) = custom_paths {
        // Use custom paths
        for log_path in paths {
            let path_ref = log_path.as_ref();
            if path_ref.exists() {
                match collect_log_entries(path_ref, MAX_LOG_LINES_PER_FILE) {
                    Ok(mut entries) => all_entries.append(&mut entries),
                    Err(e) => {
                        if e.to_string().contains("Permission denied") {
                            warn!(
                                "Permission denied reading {}: Try running with sudo or add your user to the 'adm' group",
                                path_ref.display()
                            );
                        } else {
                            warn!("Could not read {}: {}", path_ref.display(), e);
                        }
                    }
                }
            } else {
                warn!("Log file does not exist: {}", path_ref.display());
            }
        }
    } else {
        // Use default paths
        let default_paths = get_default_log_paths();
        for log_path in default_paths {
            if Path::new(log_path).exists() {
                match collect_log_entries(log_path, MAX_LOG_LINES_PER_FILE) {
                    Ok(mut entries) => all_entries.append(&mut entries),
                    Err(e) => {
                        if e.to_string().contains("Permission denied") {
                            warn!(
                                "Permission denied reading {}: Try running with sudo or add your user to the 'adm' group",
                                log_path
                            );
                        } else {
                            warn!("Could not read {}: {}", log_path, e);
                        }
                    }
                }
            }
        }
    }

    Ok(all_entries)
}

/// Scan common log file locations for issues (backward compatibility)
pub fn scan_system_logs() -> Result<Vec<LogEntry>> {
    scan_system_logs_with_paths::<&str>(None)
}
