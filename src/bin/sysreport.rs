use anyhow::{Context, Result};
use clap::Parser;
use rusqlite::Connection;
use std::env;
use std::path::PathBuf;
use systers::reporter::{format_report, generate_report};

/// System Analysis Report Generator
///
/// Generates comprehensive reports from collected system metrics and logs.
/// By default, analyzes data from the last 24 hours.
#[derive(Parser)]
#[command(name = "sysreport")]
#[command(version = systers::VERSION)]
#[command(about = "System Analysis Report Generator", long_about = None)]
struct Args {
    /// Number of hours to look back for analysis
    #[arg(long, default_value_t = 24, value_name = "N")]
    hours: i64,

    /// Path to database file (overrides SYSTERS_DB_PATH env var)
    #[arg(long, value_name = "PATH")]
    db_path: Option<PathBuf>,
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

fn main() -> Result<()> {
    // Initialize logger (defaults to WARN level for reports, configurable via RUST_LOG)
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();

    let args = Args::parse();
    let db_path = get_db_path(args.db_path);

    if !db_path.exists() {
        eprintln!("Error: Database not found at {}", db_path.display());
        eprintln!();
        eprintln!("The database file does not exist. You need to collect data first.");
        eprintln!();
        eprintln!("To fix this:");
        eprintln!("  1. Run 'syswriter' to start collecting system metrics");
        eprintln!("  2. Wait a few moments for data collection to complete");
        eprintln!("  3. Run 'sysreport' again to view the analysis");
        eprintln!();
        eprintln!("You can also specify a different database location:");
        eprintln!("  sysreport --db-path /path/to/database.db");
        eprintln!("  or set: SYSTERS_DB_PATH=/path/to/database.db");
        std::process::exit(1);
    }

    // Open database
    let conn = Connection::open(&db_path).context("Failed to open database")?;

    // Generate report
    let (metrics, logs) =
        generate_report(&conn, args.hours).context("Failed to generate report")?;

    // Format and display report
    let report = format_report(&metrics, &logs);
    println!("{}", report);

    Ok(())
}
