use crate::config::{CPU_MEASUREMENT_DELAY_MS, MAX_LOG_LINES_PER_FILE};
use crate::db::{LogEntry, SystemMetrics};
use anyhow::{Context, Result};
use chrono::Utc;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
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

/// Parse system log file for errors and warnings
pub fn collect_log_entries<P: AsRef<Path>>(
    log_path: P,
    max_entries: usize,
) -> Result<Vec<LogEntry>> {
    let file = File::open(log_path.as_ref()).context("Failed to open log file")?;
    let reader = BufReader::new(file);

    let mut entries = Vec::new();
    let timestamp = Utc::now();

    // Parse log file (simplified - assumes syslog format)
    for line in reader.lines().take(max_entries) {
        let line = line?;

        // Look for error/warning patterns
        let (level, should_include) = if line.to_lowercase().contains("error") {
            ("ERROR", true)
        } else if line.to_lowercase().contains("warning") || line.to_lowercase().contains("warn") {
            ("WARNING", true)
        } else if line.to_lowercase().contains("critical") || line.to_lowercase().contains("crit") {
            ("CRITICAL", true)
        } else if line.to_lowercase().contains("fail") {
            ("ERROR", true)
        } else {
            ("INFO", false)
        };

        if should_include {
            entries.push(LogEntry {
                timestamp,
                level: level.to_string(),
                source: "syslog".to_string(),
                message: line.trim().to_string(),
            });
        }
    }

    Ok(entries)
}

/// Scan common log file locations for issues
pub fn scan_system_logs() -> Result<Vec<LogEntry>> {
    let mut all_entries = Vec::new();

    // Common log file locations on Linux
    let log_paths = vec![
        "/var/log/syslog",
        "/var/log/messages",
        "/var/log/kern.log",
        "/var/log/auth.log",
    ];

    for log_path in log_paths {
        if Path::new(log_path).exists() {
            match collect_log_entries(log_path, MAX_LOG_LINES_PER_FILE) {
                Ok(mut entries) => all_entries.append(&mut entries),
                Err(e) => eprintln!("Warning: Could not read {}: {}", log_path, e),
            }
        }
    }

    Ok(all_entries)
}
