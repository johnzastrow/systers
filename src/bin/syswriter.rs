use anyhow::{Context, Result};
use chrono::Utc;
use clap::Parser;
use log::{debug, info, warn};
use std::env;
use std::path::PathBuf;
use systers::collector::{collect_system_metrics, scan_system_logs_with_paths};
use systers::config::DEFAULT_RETENTION_DAYS;
use systers::db::{
    cleanup_old_data, init_database, insert_log_entry, insert_metrics, insert_system_check,
    SystemCheckResult,
};

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

    /// Generate default configuration file and exit
    #[arg(long, value_name = "PATH")]
    generate_config: Option<PathBuf>,

    /// Enable enhanced system checks (requires external tools)
    #[arg(long)]
    system_checks: bool,

    /// Show available and missing external tools for system checks
    #[arg(long)]
    show_tools: bool,
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

    // Handle show-tools first
    if args.show_tools {
        use systers::system_checks::{detect_available_tools, EXTERNAL_TOOLS};

        println!("External Tools for Enhanced System Checks");
        println!("=========================================\n");

        let available_tools = detect_available_tools();
        let available_names: Vec<&str> = available_tools.iter().map(|t| t.name).collect();

        println!("✓ Available Tools:");
        for tool in &available_tools {
            println!("  • {} - {}", tool.name, tool.description);
        }

        let missing: Vec<_> = EXTERNAL_TOOLS
            .iter()
            .filter(|t| !available_names.contains(&t.name))
            .collect();

        if !missing.is_empty() {
            println!("\n✗ Missing Tools (optional):");
            for tool in &missing {
                println!("  • {} - {}", tool.name, tool.description);
                println!("    Install: {}", tool.install_hint.lines().next().unwrap_or(""));
            }

            println!("\nNote: Missing tools are optional. Basic monitoring works without them.");
            println!("Run with --system-checks to enable enhanced checks using available tools.");
        } else {
            println!("\nAll external tools are available!");
        }

        return Ok(());
    }

    // Handle config generation
    if let Some(config_path) = args.generate_config {
        use systers::config::Config;
        let config = Config::default();
        config.save_to_file(&config_path)
            .context("Failed to generate configuration file")?;
        println!("Configuration file generated at: {}", config_path.display());
        println!();
        println!("You can now:");
        println!("  1. Edit the file to customize your settings");
        println!("  2. Move it to one of these locations:");
        println!("     - ./systers.yaml (current directory)");
        println!("     - ~/.config/systers/config.yaml (user config)");
        println!("     - /etc/systers/config.yaml (system config)");
        println!();
        println!("See config.example.yaml for more examples and documentation.");
        return Ok(());
    }

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
        let (metrics_deleted, logs_deleted, checks_deleted) =
            cleanup_old_data(&conn, retention_days).context("Failed to cleanup old data")?;
        info!(
            "Deleted {} metrics, {} log entries, and {} system checks",
            metrics_deleted, logs_deleted, checks_deleted
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

    // Run enhanced system checks if enabled
    if args.system_checks {
        use systers::system_checks::*;

        println!("\n╔════════════════════════════════════════════════════════════════╗");
        println!("║           ENHANCED SYSTEM CHECKS                               ║");
        println!("╚════════════════════════════════════════════════════════════════╝\n");

        // Track which checks can run
        let mut active_checks = Vec::new();
        let mut unavailable_checks = Vec::new();

        // Check for package updates
        if is_command_available("apt") || is_command_available("dnf") {
            active_checks.push("Package Updates");
            println!("✓ Running: Package Update Check");
            match check_package_updates() {
                Ok(updates) => {
                    let message = if updates.updates_available > 0 {
                        format!(
                            "{} updates available ({} security) [{}]",
                            updates.updates_available, updates.security_updates, updates.package_manager
                        )
                    } else {
                        format!("System is up to date [{}]", updates.package_manager)
                    };
                    println!("  → {}", message);

                    // Store in database
                    let check_result = SystemCheckResult {
                        timestamp: Utc::now(),
                        check_name: "Package Updates".to_string(),
                        check_type: "package_manager".to_string(),
                        status: if updates.updates_available > 0 {
                            "warning"
                        } else {
                            "ok"
                        }
                        .to_string(),
                        value: Some(updates.updates_available.to_string()),
                        message,
                    };
                    if let Err(e) = insert_system_check(&conn, &check_result) {
                        warn!("Failed to store package update check result: {}", e);
                    }
                }
                Err(e) => {
                    println!("  → Failed: {} (may need sudo)", e);
                    let check_result = SystemCheckResult {
                        timestamp: Utc::now(),
                        check_name: "Package Updates".to_string(),
                        check_type: "package_manager".to_string(),
                        status: "error".to_string(),
                        value: None,
                        message: format!("Check failed: {}", e),
                    };
                    if let Err(e) = insert_system_check(&conn, &check_result) {
                        warn!("Failed to store package update check result: {}", e);
                    }
                }
            }
            println!();
        } else {
            unavailable_checks.push(("Package Updates", "apt or dnf", "Pre-installed on most systems"));
        }

        // Check systemd status
        if is_command_available("systemctl") {
            active_checks.push("Systemd Services");
            println!("✓ Running: Systemd Service Status");
            match check_systemd_status() {
                Ok(status) => {
                    println!("  → Total services: {}", status.total_services);
                    println!("  → Active services: {}", status.active_services);
                    if status.failed_services > 0 {
                        println!("  → ⚠️  Failed services: {}", status.failed_services);
                        for service in &status.failed_service_names {
                            println!("     - {}", service);
                        }
                    } else {
                        println!("  → Failed services: 0");
                    }

                    // Store in database
                    let message = if status.failed_services > 0 {
                        format!(
                            "Total: {}, Active: {}, Failed: {} ({})",
                            status.total_services,
                            status.active_services,
                            status.failed_services,
                            status.failed_service_names.join(", ")
                        )
                    } else {
                        format!(
                            "Total: {}, Active: {}, Failed: 0",
                            status.total_services, status.active_services
                        )
                    };

                    let check_result = SystemCheckResult {
                        timestamp: Utc::now(),
                        check_name: "Systemd Services".to_string(),
                        check_type: "systemd".to_string(),
                        status: if status.failed_services > 0 {
                            "warning"
                        } else {
                            "ok"
                        }
                        .to_string(),
                        value: Some(status.failed_services.to_string()),
                        message,
                    };
                    if let Err(e) = insert_system_check(&conn, &check_result) {
                        warn!("Failed to store systemd check result: {}", e);
                    }
                }
                Err(e) => {
                    println!("  → Failed: {}", e);
                    let check_result = SystemCheckResult {
                        timestamp: Utc::now(),
                        check_name: "Systemd Services".to_string(),
                        check_type: "systemd".to_string(),
                        status: "error".to_string(),
                        value: None,
                        message: format!("Check failed: {}", e),
                    };
                    if let Err(e) = insert_system_check(&conn, &check_result) {
                        warn!("Failed to store systemd check result: {}", e);
                    }
                }
            }
            println!();
        } else {
            unavailable_checks.push(("Systemd Services", "systemctl", "Pre-installed on systemd systems"));
        }

        // Check SMART disk health
        if is_command_available("smartctl") {
            active_checks.push("SMART Disk Health");
            println!("✓ Running: SMART Disk Health Check");
            match check_disk_health() {
                Ok(disks) => {
                    if disks.is_empty() {
                        println!("  → No disks found or unable to access (may need sudo)");
                        let check_result = SystemCheckResult {
                            timestamp: Utc::now(),
                            check_name: "SMART Disk Health".to_string(),
                            check_type: "disk_health".to_string(),
                            status: "warning".to_string(),
                            value: Some("0".to_string()),
                            message: "No disks found or unable to access".to_string(),
                        };
                        if let Err(e) = insert_system_check(&conn, &check_result) {
                            warn!("Failed to store disk health check result: {}", e);
                        }
                    } else {
                        for disk in &disks {
                            let status_icon = if disk.health_status == "PASSED" { "✓" } else { "⚠️" };
                            println!("  → {} {}: {}", status_icon, disk.device, disk.health_status);
                        }

                        // Store aggregated disk health in database
                        let failed_disks: Vec<_> = disks
                            .iter()
                            .filter(|d| d.health_status != "PASSED")
                            .collect();
                        let message = if failed_disks.is_empty() {
                            format!("All {} disk(s) healthy", disks.len())
                        } else {
                            format!(
                                "{} of {} disk(s) have issues: {}",
                                failed_disks.len(),
                                disks.len(),
                                failed_disks
                                    .iter()
                                    .map(|d| format!("{} ({})", d.device, d.health_status))
                                    .collect::<Vec<_>>()
                                    .join(", ")
                            )
                        };

                        let check_result = SystemCheckResult {
                            timestamp: Utc::now(),
                            check_name: "SMART Disk Health".to_string(),
                            check_type: "disk_health".to_string(),
                            status: if failed_disks.is_empty() {
                                "ok"
                            } else {
                                "critical"
                            }
                            .to_string(),
                            value: Some(failed_disks.len().to_string()),
                            message,
                        };
                        if let Err(e) = insert_system_check(&conn, &check_result) {
                            warn!("Failed to store disk health check result: {}", e);
                        }
                    }
                }
                Err(e) => {
                    println!("  → Failed: {} (requires sudo)", e);
                    let check_result = SystemCheckResult {
                        timestamp: Utc::now(),
                        check_name: "SMART Disk Health".to_string(),
                        check_type: "disk_health".to_string(),
                        status: "error".to_string(),
                        value: None,
                        message: format!("Check failed: {}", e),
                    };
                    if let Err(e) = insert_system_check(&conn, &check_result) {
                        warn!("Failed to store disk health check result: {}", e);
                    }
                }
            }
            println!();
        } else {
            unavailable_checks.push(("SMART Disk Health", "smartctl", "sudo apt install smartmontools"));
        }

        // Find large directories
        if is_command_available("du") {
            active_checks.push("Directory Sizes");
            println!("✓ Running: Top Directories by Size");
            match find_large_directories("/", 2, 10) {
                Ok(dirs) => {
                    for (i, dir) in dirs.iter().take(5).enumerate() {
                        println!("  → {}. {} - {}", i + 1, dir.path, dir.size_human);
                    }

                    // Store in database
                    let top_dirs = dirs
                        .iter()
                        .take(5)
                        .map(|d| format!("{}: {}", d.path, d.size_human))
                        .collect::<Vec<_>>()
                        .join(", ");

                    let check_result = SystemCheckResult {
                        timestamp: Utc::now(),
                        check_name: "Directory Sizes".to_string(),
                        check_type: "disk_usage".to_string(),
                        status: "info".to_string(),
                        value: Some(dirs.len().to_string()),
                        message: format!("Top directories: {}", top_dirs),
                    };
                    if let Err(e) = insert_system_check(&conn, &check_result) {
                        warn!("Failed to store directory size check result: {}", e);
                    }
                }
                Err(e) => {
                    println!("  → Failed: {}", e);
                    let check_result = SystemCheckResult {
                        timestamp: Utc::now(),
                        check_name: "Directory Sizes".to_string(),
                        check_type: "disk_usage".to_string(),
                        status: "error".to_string(),
                        value: None,
                        message: format!("Check failed: {}", e),
                    };
                    if let Err(e) = insert_system_check(&conn, &check_result) {
                        warn!("Failed to store directory size check result: {}", e);
                    }
                }
            }
            println!();
        } else {
            unavailable_checks.push(("Directory Sizes", "du", "Pre-installed (coreutils)"));
        }

        // Summary
        println!("─────────────────────────────────────────────────────────────────");
        println!("SUMMARY:");
        println!("  Active Checks: {}", active_checks.len());
        for check in &active_checks {
            println!("    ✓ {}", check);
        }

        if !unavailable_checks.is_empty() {
            println!("\n  Available Checks (not enabled - missing tools):");
            for (check, tool, install) in &unavailable_checks {
                println!("    ✗ {} (requires: {})", check, tool);
                println!("      Install: {}", install);
            }
            println!("\n  Tip: Run 'syswriter --show-tools' for more details");
        }
        println!("═════════════════════════════════════════════════════════════════\n");

        info!("Enhanced system checks complete");
    }

    // Automatic cleanup of old data
    if !args.no_cleanup {
        info!(
            "Cleaning up old data (retention: {} days)...",
            retention_days
        );
        match cleanup_old_data(&conn, retention_days) {
            Ok((metrics_deleted, logs_deleted, checks_deleted)) => {
                if metrics_deleted > 0 || logs_deleted > 0 || checks_deleted > 0 {
                    info!(
                        "Deleted {} metrics, {} log entries, and {} system checks",
                        metrics_deleted, logs_deleted, checks_deleted
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
