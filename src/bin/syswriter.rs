use anyhow::{Context, Result};
use clap::Parser;
use log::{debug, info, warn};
use std::env;
use std::path::PathBuf;
use systers::collector::{collect_system_metrics, scan_system_logs_with_paths};
use systers::config::DEFAULT_RETENTION_DAYS;
use systers::db::{cleanup_old_data, init_database, insert_log_entry, insert_metrics};

/// System Data Collector
///
/// Collects system metrics (CPU, memory, disk, load) and scans system logs for issues.
/// Data is stored in a SQLite database for analysis with sysreport.
#[derive(Parser)]
#[command(name = "syswriter")]
#[command(version = systers::VERSION)]
#[command(about = "System Data Collector", long_about = None)]
struct Args {
    /// Only perform cleanup of old data, skip collection
    #[arg(long)]
    cleanup: bool,

    /// Disable automatic cleanup after collection
    #[arg(long)]
    no_cleanup: bool,

    /// Number of days to retain data (default: 30)
    #[arg(long, value_name = "DAYS")]
    retention_days: Option<i64>,

    /// Path to database file (overrides SYSTERS_DB_PATH env var)
    #[arg(long, value_name = "PATH")]
    db_path: Option<PathBuf>,

    /// Comma-separated log file paths to scan (overrides SYSTERS_LOG_PATHS env var)
    #[arg(long, value_name = "PATHS", value_delimiter = ',')]
    log_paths: Option<Vec<PathBuf>>,
}

fn get_db_path(cli_path: Option<PathBuf>) -> PathBuf {
    cli_path.unwrap_or_else(|| {
        env::var("SYSTERS_DB_PATH")
            .unwrap_or_else(|_| {
                let mut path = env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
                path.push_str("/.systers.db");
                path
            })
            .into()
    })
}

fn get_log_paths(cli_paths: Option<Vec<PathBuf>>) -> Option<Vec<PathBuf>> {
    if let Some(paths) = cli_paths {
        return Some(paths);
    }

    // Try environment variable
    if let Ok(paths_str) = env::var("SYSTERS_LOG_PATHS") {
        let paths: Vec<PathBuf> = paths_str
            .split(':')
            .map(|s| PathBuf::from(s.trim()))
            .filter(|p| !p.as_os_str().is_empty())
            .collect();
        if !paths.is_empty() {
            return Some(paths);
        }
    }

    // Return None to use defaults
    None
}

fn main() -> Result<()> {
    // Initialize logger (defaults to INFO level, configurable via RUST_LOG env var)
    env_logger::init();

    let args = Args::parse();
    let db_path = get_db_path(args.db_path);
    let log_paths = get_log_paths(args.log_paths);
    let retention_days = args.retention_days.unwrap_or(DEFAULT_RETENTION_DAYS);

    info!(
        "Systers Writer - System Data Collector v{}",
        systers::VERSION
    );
    info!("Database: {}", db_path.display());
    if let Some(ref paths) = log_paths {
        debug!("Custom log paths: {:?}", paths);
    }

    // Initialize database
    let conn = init_database(&db_path).context("Failed to initialize database")?;

    // If cleanup-only mode, run cleanup and exit
    if args.cleanup {
        info!(
            "Cleaning up old data (retention: {} days)...",
            retention_days
        );
        let (metrics_deleted, logs_deleted) =
            cleanup_old_data(&conn, retention_days).context("Failed to cleanup old data")?;
        info!(
            "Deleted {} metrics and {} log entries",
            metrics_deleted, logs_deleted
        );
        return Ok(());
    }

    info!("Collecting system metrics...");

    // Collect system metrics
    let metrics = collect_system_metrics().context("Failed to collect system metrics")?;

    debug!("CPU Usage: {:.1}%", metrics.cpu_usage);
    debug!(
        "Memory: {:.1}% used",
        (metrics.memory_used as f32 / metrics.memory_total as f32) * 100.0
    );
    debug!(
        "Disk: {:.1}% used",
        (metrics.disk_used as f32 / metrics.disk_total as f32) * 100.0
    );
    debug!("Processes: {}", metrics.process_count);
    debug!(
        "Load Average: {:.2}, {:.2}, {:.2}",
        metrics.load_avg_1min, metrics.load_avg_5min, metrics.load_avg_15min
    );

    // Store metrics
    insert_metrics(&conn, &metrics).context("Failed to insert metrics")?;

    info!("Scanning system logs for issues...");

    // Collect and store log entries
    let entries_result = if let Some(ref paths) = log_paths {
        scan_system_logs_with_paths(Some(paths.as_slice()))
    } else {
        scan_system_logs_with_paths::<PathBuf>(None)
    };

    match entries_result {
        Ok(entries) => {
            let error_count = entries.iter().filter(|e| e.level == "ERROR").count();
            let warning_count = entries.iter().filter(|e| e.level == "WARNING").count();
            let critical_count = entries.iter().filter(|e| e.level == "CRITICAL").count();

            info!(
                "Found {} critical, {} errors, {} warnings",
                critical_count, error_count, warning_count
            );

            for entry in entries {
                insert_log_entry(&conn, &entry).context("Failed to insert log entry")?;
            }
        }
        Err(e) => {
            warn!("Could not scan all logs: {}", e);
        }
    }

    info!("Data collection complete at {}", metrics.timestamp);

    // Automatic cleanup of old data
    if !args.no_cleanup {
        info!(
            "Cleaning up old data (retention: {} days)...",
            retention_days
        );
        match cleanup_old_data(&conn, retention_days) {
            Ok((metrics_deleted, logs_deleted)) => {
                if metrics_deleted > 0 || logs_deleted > 0 {
                    info!(
                        "Deleted {} metrics and {} log entries",
                        metrics_deleted, logs_deleted
                    );
                }
            }
            Err(e) => {
                warn!("Cleanup failed: {}", e);
            }
        }
    }

    info!("Use 'sysreport' to view analysis");

    Ok(())
}
