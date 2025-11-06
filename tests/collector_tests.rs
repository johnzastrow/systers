use anyhow::Result;
use std::fs::File;
use std::io::Write;
use systers::collector::collect_log_entries;
use tempfile::TempDir;

/// Test log entry collection from a test file
#[test]
fn test_collect_log_entries_with_errors() -> Result<()> {
    // Create a temporary directory and log file
    let temp_dir = TempDir::new()?;
    let log_file_path = temp_dir.path().join("test.log");
    let mut log_file = File::create(&log_file_path)?;

    // Write test log content
    writeln!(log_file, "This is a normal log line")?;
    writeln!(log_file, "This line contains an error message")?;
    writeln!(log_file, "Another normal line")?;
    writeln!(log_file, "WARNING: This is a warning")?;
    writeln!(log_file, "CRITICAL: System failure detected")?;
    writeln!(log_file, "Connection failed to database")?;
    log_file.sync_all()?;

    // Collect log entries
    let entries = collect_log_entries(&log_file_path, 100)?;

    // Should find error, warning, critical, and fail entries
    assert!(entries.len() >= 4);

    // Verify levels are correctly identified
    let has_error = entries.iter().any(|e| e.level == "ERROR");
    let has_warning = entries.iter().any(|e| e.level == "WARNING");
    let has_critical = entries.iter().any(|e| e.level == "CRITICAL");

    assert!(has_error);
    assert!(has_warning);
    assert!(has_critical);

    Ok(())
}

/// Test that normal log lines are filtered out
#[test]
fn test_collect_log_entries_filters_normal() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let log_file_path = temp_dir.path().join("test.log");
    let mut log_file = File::create(&log_file_path)?;

    // Write only normal log content
    writeln!(log_file, "Application started successfully")?;
    writeln!(log_file, "Processing user request")?;
    writeln!(log_file, "Database connection established")?;
    log_file.sync_all()?;

    // Collect log entries - should be empty since no errors/warnings
    let entries = collect_log_entries(&log_file_path, 100)?;

    assert_eq!(entries.len(), 0);

    Ok(())
}

/// Test max_entries limit
#[test]
fn test_collect_log_entries_respects_limit() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let log_file_path = temp_dir.path().join("test.log");
    let mut log_file = File::create(&log_file_path)?;

    // Write many error lines
    for i in 0..100 {
        writeln!(log_file, "Error message {}", i)?;
    }
    log_file.sync_all()?;

    // Collect with limit of 10
    let entries = collect_log_entries(&log_file_path, 10)?;

    // Should respect the limit
    assert_eq!(entries.len(), 10);

    Ok(())
}

/// Test log entry source is set correctly to the log file path
#[test]
fn test_collect_log_entries_source() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let log_file_path = temp_dir.path().join("test.log");
    let mut log_file = File::create(&log_file_path)?;

    writeln!(log_file, "Error: test error")?;
    log_file.sync_all()?;

    let entries = collect_log_entries(&log_file_path, 100)?;

    assert_eq!(entries.len(), 1);
    // Source should now be the actual log file path
    assert_eq!(
        entries[0].source,
        log_file_path.to_string_lossy().to_string()
    );

    Ok(())
}

/// Test case-insensitive error detection
#[test]
fn test_collect_log_entries_case_insensitive() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let log_file_path = temp_dir.path().join("test.log");
    let mut log_file = File::create(&log_file_path)?;

    writeln!(log_file, "ERROR in uppercase")?;
    writeln!(log_file, "error in lowercase")?;
    writeln!(log_file, "ErRoR in mixed case")?;
    writeln!(log_file, "WARNING in uppercase")?;
    writeln!(log_file, "warning in lowercase")?;
    log_file.sync_all()?;

    let entries = collect_log_entries(&log_file_path, 100)?;

    // Should detect all variations
    assert_eq!(entries.len(), 5);

    Ok(())
}

/// Test empty log file
#[test]
fn test_collect_log_entries_empty_file() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let log_file_path = temp_dir.path().join("test.log");
    File::create(&log_file_path)?;

    let entries = collect_log_entries(&log_file_path, 100)?;

    assert_eq!(entries.len(), 0);

    Ok(())
}

/// Test that collect_system_metrics returns valid data
#[test]
fn test_collect_system_metrics() -> Result<()> {
    let metrics = systers::collector::collect_system_metrics()?;

    // Verify fields are populated with reasonable values
    assert!(metrics.cpu_usage >= 0.0 && metrics.cpu_usage <= 100.0);
    assert!(metrics.memory_total > 0);
    assert!(metrics.memory_used <= metrics.memory_total);
    assert!(metrics.process_count > 0);
    assert!(metrics.disk_total >= metrics.disk_used);

    // Load averages can be 0 or positive
    assert!(metrics.load_avg_1min >= 0.0);
    assert!(metrics.load_avg_5min >= 0.0);
    assert!(metrics.load_avg_15min >= 0.0);

    Ok(())
}
