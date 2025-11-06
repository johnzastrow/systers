use anyhow::Result;
use chrono::{Duration, Utc};
use systers::db::{
    cleanup_old_data, init_database, insert_log_entry, insert_metrics, query_logs, query_metrics,
};
use systers::db::{LogEntry, SystemMetrics};

/// Test database initialization with in-memory database
#[test]
fn test_init_database() -> Result<()> {
    let conn = init_database(":memory:")?;

    // Verify tables were created
    let mut stmt = conn.prepare("SELECT name FROM sqlite_master WHERE type='table'")?;
    let tables: Vec<String> = stmt
        .query_map([], |row| row.get(0))?
        .collect::<Result<Vec<_>, _>>()?;

    assert!(tables.contains(&"schema_version".to_string()));
    assert!(tables.contains(&"system_metrics".to_string()));
    assert!(tables.contains(&"log_entries".to_string()));

    // Verify schema version is set
    let version: i32 =
        conn.query_row("SELECT version FROM schema_version", [], |row| row.get(0))?;
    assert_eq!(version, 1);

    // Verify app version is set
    let app_version: String =
        conn.query_row("SELECT app_version FROM schema_version", [], |row| {
            row.get(0)
        })?;
    assert_eq!(app_version, env!("CARGO_PKG_VERSION"));

    Ok(())
}

/// Test inserting and querying system metrics
#[test]
fn test_metrics_insert_and_query() -> Result<()> {
    let conn = init_database(":memory:")?;
    let now = Utc::now();

    // Create test metrics
    let metrics = SystemMetrics {
        timestamp: now,
        cpu_usage: 45.5,
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

    // Insert metrics
    insert_metrics(&conn, &metrics)?;

    // Query back
    let start = now - Duration::hours(1);
    let end = now + Duration::hours(1);
    let results = query_metrics(&conn, start, end)?;

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].cpu_usage, 45.5);
    assert_eq!(results[0].process_count, 150);

    Ok(())
}

/// Test querying metrics with no results
#[test]
fn test_query_metrics_empty() -> Result<()> {
    let conn = init_database(":memory:")?;
    let now = Utc::now();

    // Query with no data
    let start = now - Duration::hours(1);
    let end = now + Duration::hours(1);
    let results = query_metrics(&conn, start, end)?;

    assert_eq!(results.len(), 0);

    Ok(())
}

/// Test inserting and querying log entries
#[test]
fn test_logs_insert_and_query() -> Result<()> {
    let conn = init_database(":memory:")?;
    let now = Utc::now();

    // Create test log entries
    let error_entry = LogEntry {
        timestamp: now,
        level: "ERROR".to_string(),
        source: "test".to_string(),
        message: "Test error message".to_string(),
    };

    let warning_entry = LogEntry {
        timestamp: now,
        level: "WARNING".to_string(),
        source: "test".to_string(),
        message: "Test warning message".to_string(),
    };

    // Insert logs
    insert_log_entry(&conn, &error_entry)?;
    insert_log_entry(&conn, &warning_entry)?;

    // Query all logs
    let start = now - Duration::hours(1);
    let end = now + Duration::hours(1);
    let all_logs = query_logs(&conn, start, end, None)?;
    assert_eq!(all_logs.len(), 2);

    // Query only errors
    let error_logs = query_logs(&conn, start, end, Some("ERROR"))?;
    assert_eq!(error_logs.len(), 1);
    assert_eq!(error_logs[0].level, "ERROR");

    // Query only warnings
    let warning_logs = query_logs(&conn, start, end, Some("WARNING"))?;
    assert_eq!(warning_logs.len(), 1);
    assert_eq!(warning_logs[0].level, "WARNING");

    Ok(())
}

/// Test querying with multiple metrics over time range
#[test]
fn test_query_metrics_time_range() -> Result<()> {
    let conn = init_database(":memory:")?;
    let now = Utc::now();

    // Insert metrics at different times
    for i in 0..5 {
        let metrics = SystemMetrics {
            timestamp: now - Duration::hours(i),
            cpu_usage: 10.0 + (i as f32 * 5.0),
            memory_total: 16_000_000_000,
            memory_used: 8_000_000_000,
            memory_available: 8_000_000_000,
            disk_total: 500_000_000_000,
            disk_used: 250_000_000_000,
            process_count: 100 + ((i * 10) as usize),
            load_avg_1min: 1.0,
            load_avg_5min: 1.0,
            load_avg_15min: 1.0,
        };
        insert_metrics(&conn, &metrics)?;
    }

    // Query all
    let start = now - Duration::hours(6);
    let end = now + Duration::hours(1);
    let all_results = query_metrics(&conn, start, end)?;
    assert_eq!(all_results.len(), 5);

    // Query limited time range
    let start = now - Duration::hours(2);
    let end = now + Duration::hours(1);
    let limited_results = query_metrics(&conn, start, end)?;
    assert_eq!(limited_results.len(), 3);

    Ok(())
}

/// Test that indices are created properly
#[test]
fn test_indices_exist() -> Result<()> {
    let conn = init_database(":memory:")?;

    // Check for indices
    let mut stmt = conn.prepare("SELECT name FROM sqlite_master WHERE type='index'")?;
    let indices: Vec<String> = stmt
        .query_map([], |row| row.get(0))?
        .collect::<Result<Vec<_>, _>>()?;

    assert!(indices.contains(&"idx_metrics_timestamp".to_string()));
    assert!(indices.contains(&"idx_logs_timestamp".to_string()));
    assert!(indices.contains(&"idx_logs_level".to_string()));

    Ok(())
}

/// Test cleanup of old data
#[test]
fn test_cleanup_old_data() -> Result<()> {
    let conn = init_database(":memory:")?;
    let now = Utc::now();

    // Insert recent data (last 2 days)
    for i in 0..5 {
        let metrics = SystemMetrics {
            timestamp: now - Duration::days(i),
            cpu_usage: 30.0,
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
    }

    // Insert old data (31+ days ago)
    for i in 31..35 {
        let metrics = SystemMetrics {
            timestamp: now - Duration::days(i),
            cpu_usage: 30.0,
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
    }

    // Insert old log entries
    for i in 31..33 {
        let log = LogEntry {
            timestamp: now - Duration::days(i),
            level: "ERROR".to_string(),
            source: "test".to_string(),
            message: "Old error".to_string(),
        };
        insert_log_entry(&conn, &log)?;
    }

    // Cleanup data older than 30 days
    let (metrics_deleted, logs_deleted) = cleanup_old_data(&conn, 30)?;

    // Should have deleted 4 old metrics and 2 old logs
    assert_eq!(metrics_deleted, 4);
    assert_eq!(logs_deleted, 2);

    // Verify remaining data
    let start = now - Duration::days(100);
    let end = now + Duration::days(1);
    let remaining_metrics = query_metrics(&conn, start, end)?;

    // Should only have 5 recent metrics left
    assert_eq!(remaining_metrics.len(), 5);

    Ok(())
}
