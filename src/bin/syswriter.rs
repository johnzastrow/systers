use anyhow::{Context, Result};
use std::env;
use std::path::PathBuf;
use systers::collector::{collect_system_metrics, scan_system_logs};
use systers::config::DEFAULT_RETENTION_DAYS;
use systers::db::{cleanup_old_data, init_database, insert_log_entry, insert_metrics};

fn get_db_path() -> PathBuf {
    env::var("SYSTERS_DB_PATH")
        .unwrap_or_else(|_| {
            let mut path = env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
            path.push_str("/.systers.db");
            path
        })
        .into()
}

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    let cleanup_only = args.contains(&"--cleanup".to_string());
    let auto_cleanup = !args.contains(&"--no-cleanup".to_string());

    let db_path = get_db_path();

    println!(
        "Systers Writer - System Data Collector v{}",
        systers::VERSION
    );
    println!("Database: {}", db_path.display());

    // Initialize database
    let conn = init_database(&db_path).context("Failed to initialize database")?;

    // If cleanup-only mode, run cleanup and exit
    if cleanup_only {
        println!(
            "\nCleaning up old data (retention: {} days)...",
            DEFAULT_RETENTION_DAYS
        );
        let (metrics_deleted, logs_deleted) = cleanup_old_data(&conn, DEFAULT_RETENTION_DAYS)
            .context("Failed to cleanup old data")?;
        println!(
            "✓ Deleted {} metrics and {} log entries",
            metrics_deleted, logs_deleted
        );
        return Ok(());
    }

    println!("Collecting system metrics...");

    // Collect system metrics
    let metrics = collect_system_metrics().context("Failed to collect system metrics")?;

    println!("  CPU Usage: {:.1}%", metrics.cpu_usage);
    println!(
        "  Memory: {:.1}% used",
        (metrics.memory_used as f32 / metrics.memory_total as f32) * 100.0
    );
    println!(
        "  Disk: {:.1}% used",
        (metrics.disk_used as f32 / metrics.disk_total as f32) * 100.0
    );
    println!("  Processes: {}", metrics.process_count);
    println!(
        "  Load Average: {:.2}, {:.2}, {:.2}",
        metrics.load_avg_1min, metrics.load_avg_5min, metrics.load_avg_15min
    );

    // Store metrics
    insert_metrics(&conn, &metrics).context("Failed to insert metrics")?;

    println!("\nScanning system logs for issues...");

    // Collect and store log entries
    match scan_system_logs() {
        Ok(entries) => {
            let error_count = entries.iter().filter(|e| e.level == "ERROR").count();
            let warning_count = entries.iter().filter(|e| e.level == "WARNING").count();
            let critical_count = entries.iter().filter(|e| e.level == "CRITICAL").count();

            println!(
                "  Found {} critical, {} errors, {} warnings",
                critical_count, error_count, warning_count
            );

            for entry in entries {
                insert_log_entry(&conn, &entry).context("Failed to insert log entry")?;
            }
        }
        Err(e) => {
            eprintln!("  Warning: Could not scan all logs: {}", e);
        }
    }

    println!("\n✓ Data collection complete at {}", metrics.timestamp);

    // Automatic cleanup of old data
    if auto_cleanup {
        println!(
            "\nCleaning up old data (retention: {} days)...",
            DEFAULT_RETENTION_DAYS
        );
        match cleanup_old_data(&conn, DEFAULT_RETENTION_DAYS) {
            Ok((metrics_deleted, logs_deleted)) => {
                if metrics_deleted > 0 || logs_deleted > 0 {
                    println!(
                        "  Deleted {} metrics and {} log entries",
                        metrics_deleted, logs_deleted
                    );
                }
            }
            Err(e) => {
                eprintln!("  Warning: Cleanup failed: {}", e);
            }
        }
    }

    println!("\n  Use 'sysreport' to view analysis");

    Ok(())
}
