use anyhow::{Context, Result};
use clap::Parser;
use rusqlite::Connection;
use std::env;
use std::fs;
use std::path::PathBuf;
use systers::reporter::{export_report, generate_report, ExportFormat};

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

    /// Output file path (if not specified, prints to stdout)
    #[arg(short, long, value_name = "FILE")]
    output: Option<PathBuf>,

    /// Output format: text or json (default: text)
    #[arg(short, long, value_name = "FORMAT", default_value = "text")]
    format: String,
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

    // Parse export format
    let export_format = ExportFormat::from_str(&args.format)
        .context(format!("Invalid format '{}'. Supported: text, json", args.format))?;

    // Open database
    let conn = Connection::open(&db_path).context("Failed to open database")?;

    // Generate report
    let (metrics, logs, system_checks) =
        generate_report(&conn, args.hours).context("Failed to generate report")?;

    // Export report in the specified format
    let report_content = export_report(&metrics, &logs, &system_checks, export_format)
        .context("Failed to export report")?;

    // Write to file or stdout
    if let Some(output_path) = args.output {
        fs::write(&output_path, &report_content)
            .context(format!("Failed to write report to {}", output_path.display()))?;
        eprintln!("Report saved to: {}", output_path.display());
    } else {
        println!("{}", report_content);
    }

    Ok(())
}
