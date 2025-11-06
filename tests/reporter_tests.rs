use anyhow::Result;
use chrono::{Duration, Utc};
use systers::db::{init_database, insert_log_entry, insert_metrics};
use systers::db::{LogEntry, SystemMetrics};
use systers::reporter::{format_report, generate_report};

/// Test report generation with no data
#[test]
fn test_generate_report_no_data() -> Result<()> {
    let conn = init_database(":memory:")?;
    let (metrics, logs, system_checks) = generate_report(&conn, 24)?;

    assert_eq!(metrics.avg_cpu_usage, 0.0);
    assert_eq!(metrics.max_cpu_usage, 0.0);
    assert_eq!(logs.total_errors, 0);
    assert_eq!(logs.total_warnings, 0);
    assert_eq!(system_checks.total_checks, 0);
    assert!(!metrics.issues.is_empty()); // Should have "No data available" message

    Ok(())
}

/// Test report generation with sample data
#[test]
fn test_generate_report_with_data() -> Result<()> {
    let conn = init_database(":memory:")?;
    let now = Utc::now();

    // Insert sample metrics
    for i in 0..10 {
        let metrics = SystemMetrics {
            timestamp: now - Duration::hours(i),
            cpu_usage: 30.0 + (i as f32 * 2.0),
            memory_total: 16_000_000_000,
            memory_used: 8_000_000_000 + (i as u64 * 100_000_000),
            memory_available: 8_000_000_000 - (i as u64 * 100_000_000),
            disk_total: 500_000_000_000,
            disk_used: 250_000_000_000,
            process_count: 150,
            load_avg_1min: 1.5,
            load_avg_5min: 1.2,
            load_avg_15min: 1.0,
        };
        insert_metrics(&conn, &metrics)?;
    }

    // Insert sample logs
    for i in 0..5 {
        let error = LogEntry {
            timestamp: now - Duration::hours(i),
            level: "ERROR".to_string(),
            source: "test".to_string(),
            message: format!("Test error {}", i),
        };
        insert_log_entry(&conn, &error)?;
    }

    let (metrics, logs, _system_checks) = generate_report(&conn, 24)?;

    assert!(metrics.avg_cpu_usage > 0.0);
    assert!(metrics.max_cpu_usage > 0.0);
    assert_eq!(logs.total_errors, 5);
    assert_eq!(logs.recent_errors.len(), 5);

    Ok(())
}

/// Test issue detection for high CPU
#[test]
fn test_issue_detection_high_cpu() -> Result<()> {
    let conn = init_database(":memory:")?;
    let now = Utc::now();

    // Insert metrics with high CPU
    let metrics = SystemMetrics {
        timestamp: now,
        cpu_usage: 95.0, // High CPU!
        memory_total: 16_000_000_000,
        memory_used: 8_000_000_000,
        memory_available: 8_000_000_000,
        disk_total: 500_000_000_000,
        disk_used: 250_000_000_000,
        process_count: 150,
        load_avg_1min: 1.5,
        load_avg_5min: 1.2,
        load_avg_15min: 1.0,
    };
    insert_metrics(&conn, &metrics)?;

    let (report, _, _) = generate_report(&conn, 1)?;

    // Should detect high CPU issue
    assert!(!report.issues.is_empty());
    assert!(report.issues.iter().any(|i| i.contains("CPU")));

    Ok(())
}

/// Test issue detection for high memory
#[test]
fn test_issue_detection_high_memory() -> Result<()> {
    let conn = init_database(":memory:")?;
    let now = Utc::now();

    // Insert metrics with high memory usage
    let metrics = SystemMetrics {
        timestamp: now,
        cpu_usage: 30.0,
        memory_total: 16_000_000_000,
        memory_used: 15_000_000_000, // 93.75% used - high!
        memory_available: 1_000_000_000,
        disk_total: 500_000_000_000,
        disk_used: 250_000_000_000,
        process_count: 150,
        load_avg_1min: 1.5,
        load_avg_5min: 1.2,
        load_avg_15min: 1.0,
    };
    insert_metrics(&conn, &metrics)?;

    let (report, _, _) = generate_report(&conn, 1)?;

    // Should detect high memory issue
    assert!(!report.issues.is_empty());
    assert!(report.issues.iter().any(|i| i.contains("MEMORY")));

    Ok(())
}

/// Test issue detection for high disk usage
#[test]
fn test_issue_detection_high_disk() -> Result<()> {
    let conn = init_database(":memory:")?;
    let now = Utc::now();

    // Insert metrics with high disk usage
    let metrics = SystemMetrics {
        timestamp: now,
        cpu_usage: 30.0,
        memory_total: 16_000_000_000,
        memory_used: 8_000_000_000,
        memory_available: 8_000_000_000,
        disk_total: 500_000_000_000,
        disk_used: 450_000_000_000, // 90% used - high!
        process_count: 150,
        load_avg_1min: 1.5,
        load_avg_5min: 1.2,
        load_avg_15min: 1.0,
    };
    insert_metrics(&conn, &metrics)?;

    let (report, _, _) = generate_report(&conn, 1)?;

    // Should detect high disk issue
    assert!(!report.issues.is_empty());
    assert!(report.issues.iter().any(|i| i.contains("DISK")));

    Ok(())
}

/// Test report formatting
#[test]
fn test_format_report() -> Result<()> {
    let conn = init_database(":memory:")?;
    let now = Utc::now();

    // Insert some data
    let metrics = SystemMetrics {
        timestamp: now,
        cpu_usage: 45.0,
        memory_total: 16_000_000_000,
        memory_used: 8_000_000_000,
        memory_available: 8_000_000_000,
        disk_total: 500_000_000_000,
        disk_used: 250_000_000_000,
        process_count: 150,
        load_avg_1min: 1.5,
        load_avg_5min: 1.2,
        load_avg_15min: 1.0,
    };
    insert_metrics(&conn, &metrics)?;

    let (metrics_report, logs_report, system_checks_report) = generate_report(&conn, 1)?;
    let report = format_report(&metrics_report, &logs_report, &system_checks_report);

    // Verify report contains expected sections
    assert!(report.contains("SYSTERS"));
    assert!(report.contains("SYSTEM ANALYSIS REPORT"));
    assert!(report.contains("SYSTEM METRICS"));
    assert!(report.contains("CPU Usage:"));
    assert!(report.contains("Memory Usage:"));
    assert!(report.contains("Disk Usage:"));
    assert!(report.contains("LOG ANALYSIS"));
    assert!(report.contains("RECOMMENDATIONS"));

    // Verify version is displayed
    assert!(report.contains(env!("CARGO_PKG_VERSION")));

    Ok(())
}

/// Test statistics calculations
#[test]
fn test_statistics_calculations() -> Result<()> {
    let conn = init_database(":memory:")?;
    let now = Utc::now();

    // Insert metrics with known values
    let cpu_values = vec![10.0, 20.0, 30.0, 40.0, 50.0];
    for (i, cpu) in cpu_values.iter().enumerate() {
        let metrics = SystemMetrics {
            timestamp: now - Duration::hours(i as i64),
            cpu_usage: *cpu,
            memory_total: 100_000_000,
            memory_used: 50_000_000,
            memory_available: 50_000_000,
            disk_total: 1_000_000_000,
            disk_used: 500_000_000,
            process_count: 100,
            load_avg_1min: 1.0,
            load_avg_5min: 1.0,
            load_avg_15min: 1.0,
        };
        insert_metrics(&conn, &metrics)?;
    }

    let (report, _, _) = generate_report(&conn, 24)?;

    // Average CPU should be 30.0 (10+20+30+40+50)/5
    assert_eq!(report.avg_cpu_usage, 30.0);

    // Max CPU should be 50.0
    assert_eq!(report.max_cpu_usage, 50.0);

    // Memory should be 50% (50M/100M)
    assert_eq!(report.avg_memory_used_percent, 50.0);

    Ok(())
}
