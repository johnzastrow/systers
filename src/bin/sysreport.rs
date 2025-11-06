use anyhow::{Context, Result};
use rusqlite::Connection;
use std::env;
use std::path::PathBuf;
use systers::reporter::{format_report, generate_report};

fn get_db_path() -> PathBuf {
    env::var("SYSTERS_DB_PATH")
        .unwrap_or_else(|_| {
            let mut path = env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
            path.push_str("/.systers.db");
            path
        })
        .into()
}

fn print_usage() {
    println!(
        "Systers Report - System Analysis Report Generator v{}",
        systers::VERSION
    );
    println!();
    println!("USAGE:");
    println!("  sysreport [OPTIONS]");
    println!();
    println!("OPTIONS:");
    println!("  --hours <N>    Look back N hours (default: 24)");
    println!("  --help         Show this help message");
    println!();
    println!("ENVIRONMENT:");
    println!("  SYSTERS_DB_PATH    Path to database file (default: ~/.systers.db)");
}

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();

    // Parse command line arguments
    let mut hours_back = 24i64;
    let mut i = 1;

    while i < args.len() {
        match args[i].as_str() {
            "--help" | "-h" => {
                print_usage();
                return Ok(());
            }
            "--hours" => {
                i += 1;
                if i >= args.len() {
                    eprintln!("Error: --hours requires a value");
                    print_usage();
                    std::process::exit(1);
                }
                hours_back = args[i].parse().context("Invalid value for --hours")?;
            }
            _ => {
                eprintln!("Error: Unknown argument '{}'", args[i]);
                print_usage();
                std::process::exit(1);
            }
        }
        i += 1;
    }

    let db_path = get_db_path();

    if !db_path.exists() {
        eprintln!("Error: Database not found at {}", db_path.display());
        eprintln!("Run 'syswriter' first to collect system data");
        std::process::exit(1);
    }

    // Open database
    let conn = Connection::open(&db_path).context("Failed to open database")?;

    // Generate report
    let (metrics, logs) =
        generate_report(&conn, hours_back).context("Failed to generate report")?;

    // Format and display report
    let report = format_report(&metrics, &logs);
    println!("{}", report);

    Ok(())
}
