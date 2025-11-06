use anyhow::{Context, Result};
use chrono::{DateTime, TimeZone, Utc};
use log::{info, warn};
use rusqlite::{params, Connection};
use serde::Serialize;
use std::path::Path;

/// Database schema version
pub const SCHEMA_VERSION: i32 = 2;

/// System metrics record
#[derive(Debug, Clone, Serialize)]
pub struct SystemMetrics {
    pub timestamp: DateTime<Utc>,
    pub cpu_usage: f32,
    pub memory_total: u64,
    pub memory_used: u64,
    pub memory_available: u64,
    pub disk_total: u64,
    pub disk_used: u64,
    pub process_count: usize,
    pub load_avg_1min: f64,
    pub load_avg_5min: f64,
    pub load_avg_15min: f64,
}

/// Log entry record
#[derive(Debug, Clone, Serialize)]
pub struct LogEntry {
    pub timestamp: DateTime<Utc>,
    pub level: String,
    pub source: String,
    pub message: String,
}

/// Get current schema version from database
fn get_schema_version(conn: &Connection) -> Result<i32> {
    // Check if schema_version table exists
    let table_exists: bool = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='schema_version'",
            [],
            |row| {
                let count: i32 = row.get(0)?;
                Ok(count > 0)
            },
        )?;

    if !table_exists {
        return Ok(0); // Fresh database
    }

    // Try to get version
    match conn.query_row("SELECT version FROM schema_version", [], |row| row.get(0)) {
        Ok(version) => Ok(version),
        Err(_) => Ok(0),
    }
}

/// Migrate from schema v1 to v2 (TEXT timestamps to INTEGER timestamps)
fn migrate_v1_to_v2(conn: &Connection) -> Result<()> {
    info!("Migrating database from schema v1 to v2...");

    // Create new tables with INTEGER timestamps
    conn.execute(
        "CREATE TABLE IF NOT EXISTS system_metrics_v2 (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            timestamp INTEGER NOT NULL,
            cpu_usage REAL NOT NULL,
            memory_total INTEGER NOT NULL,
            memory_used INTEGER NOT NULL,
            memory_available INTEGER NOT NULL,
            disk_total INTEGER NOT NULL,
            disk_used INTEGER NOT NULL,
            process_count INTEGER NOT NULL,
            load_avg_1min REAL NOT NULL,
            load_avg_5min REAL NOT NULL,
            load_avg_15min REAL NOT NULL
        )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS log_entries_v2 (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            timestamp INTEGER NOT NULL,
            level TEXT NOT NULL,
            source TEXT NOT NULL,
            message TEXT NOT NULL
        )",
        [],
    )?;

    // Migrate system_metrics data
    // SQLite can't parse RFC3339 directly, so we need to convert via Rust
    let mut stmt = conn.prepare("SELECT * FROM system_metrics")?;
    let metrics_iter = stmt.query_map([], |row| {
        let timestamp_str: String = row.get(1)?;
        Ok((
            row.get::<_, i32>(0)?, // id
            timestamp_str,
            row.get::<_, f32>(2)?,  // cpu_usage
            row.get::<_, i64>(3)?,  // memory_total
            row.get::<_, i64>(4)?,  // memory_used
            row.get::<_, i64>(5)?,  // memory_available
            row.get::<_, i64>(6)?,  // disk_total
            row.get::<_, i64>(7)?,  // disk_used
            row.get::<_, i32>(8)?,  // process_count
            row.get::<_, f64>(9)?,  // load_avg_1min
            row.get::<_, f64>(10)?, // load_avg_5min
            row.get::<_, f64>(11)?, // load_avg_15min
        ))
    })?;

    for row in metrics_iter {
        let (id, ts_str, cpu, mem_tot, mem_used, mem_avail, disk_tot, disk_used, proc_cnt, load1, load5, load15) = row?;

        // Parse RFC3339 timestamp and convert to Unix timestamp
        let timestamp = if let Ok(dt) = DateTime::parse_from_rfc3339(&ts_str) {
            dt.timestamp()
        } else {
            warn!("Could not parse timestamp '{}', using current time", ts_str);
            Utc::now().timestamp()
        };

        conn.execute(
            "INSERT INTO system_metrics_v2 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            params![id, timestamp, cpu, mem_tot, mem_used, mem_avail, disk_tot, disk_used, proc_cnt, load1, load5, load15],
        )?;
    }

    // Migrate log_entries data
    let mut stmt = conn.prepare("SELECT * FROM log_entries")?;
    let logs_iter = stmt.query_map([], |row| {
        Ok((
            row.get::<_, i32>(0)?,     // id
            row.get::<_, String>(1)?,  // timestamp
            row.get::<_, String>(2)?,  // level
            row.get::<_, String>(3)?,  // source
            row.get::<_, String>(4)?,  // message
        ))
    })?;

    for row in logs_iter {
        let (id, ts_str, level, source, message) = row?;

        // Parse RFC3339 timestamp and convert to Unix timestamp
        let timestamp = if let Ok(dt) = DateTime::parse_from_rfc3339(&ts_str) {
            dt.timestamp()
        } else {
            warn!("Could not parse log timestamp '{}', using current time", ts_str);
            Utc::now().timestamp()
        };

        conn.execute(
            "INSERT INTO log_entries_v2 VALUES (?, ?, ?, ?, ?)",
            params![id, timestamp, level, source, message],
        )?;
    }

    // Drop old tables
    conn.execute("DROP TABLE system_metrics", [])?;
    conn.execute("DROP TABLE log_entries", [])?;

    // Rename new tables
    conn.execute("ALTER TABLE system_metrics_v2 RENAME TO system_metrics", [])?;
    conn.execute("ALTER TABLE log_entries_v2 RENAME TO log_entries", [])?;

    info!("Migration to schema v2 complete");
    Ok(())
}

/// Initialize the database with required schema
pub fn init_database<P: AsRef<Path>>(db_path: P) -> Result<Connection> {
    let path_ref = db_path.as_ref();
    let conn = Connection::open(path_ref).context("Failed to open database")?;

    // Create schema_version table if it doesn't exist
    conn.execute(
        "CREATE TABLE IF NOT EXISTS schema_version (
            version INTEGER PRIMARY KEY,
            app_version TEXT
        )",
        [],
    )?;

    // Get current schema version
    let current_version = get_schema_version(&conn)?;

    // Perform migrations if needed
    if current_version == 0 {
        // Fresh database - create v2 schema directly
        info!("Creating fresh database with schema v2");
    } else if current_version == 1 {
        // Migrate from v1 to v2
        migrate_v1_to_v2(&conn)?;
    } else if current_version > SCHEMA_VERSION {
        warn!(
            "Database schema version ({}) is newer than application version ({})",
            current_version, SCHEMA_VERSION
        );
    }

    // Create or recreate tables with v2 schema (INTEGER timestamps)
    conn.execute(
        "CREATE TABLE IF NOT EXISTS system_metrics (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            timestamp INTEGER NOT NULL,
            cpu_usage REAL NOT NULL,
            memory_total INTEGER NOT NULL,
            memory_used INTEGER NOT NULL,
            memory_available INTEGER NOT NULL,
            disk_total INTEGER NOT NULL,
            disk_used INTEGER NOT NULL,
            process_count INTEGER NOT NULL,
            load_avg_1min REAL NOT NULL,
            load_avg_5min REAL NOT NULL,
            load_avg_15min REAL NOT NULL
        )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS log_entries (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            timestamp INTEGER NOT NULL,
            level TEXT NOT NULL,
            source TEXT NOT NULL,
            message TEXT NOT NULL
        )",
        [],
    )?;

    // Create indices for better query performance
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_metrics_timestamp
         ON system_metrics(timestamp)",
        [],
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_logs_timestamp
         ON log_entries(timestamp)",
        [],
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_logs_level
         ON log_entries(level)",
        [],
    )?;

    // Store schema version and app version
    conn.execute(
        "INSERT OR REPLACE INTO schema_version (version, app_version) VALUES (?1, ?2)",
        params![SCHEMA_VERSION, crate::VERSION],
    )?;

    // Set restrictive permissions on database file (Unix only)
    // Skip for in-memory databases
    #[cfg(unix)]
    {
        if path_ref.to_str() != Some(":memory:") {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(path_ref)?.permissions();
            perms.set_mode(0o600); // Read/write for owner only
            std::fs::set_permissions(path_ref, perms)?;
        }
    }

    Ok(conn)
}

/// Insert system metrics into database
pub fn insert_metrics(conn: &Connection, metrics: &SystemMetrics) -> Result<()> {
    conn.execute(
        "INSERT INTO system_metrics (
            timestamp, cpu_usage, memory_total, memory_used, memory_available,
            disk_total, disk_used, process_count,
            load_avg_1min, load_avg_5min, load_avg_15min
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        params![
            metrics.timestamp.timestamp(), // Unix timestamp in seconds
            metrics.cpu_usage,
            metrics.memory_total,
            metrics.memory_used,
            metrics.memory_available,
            metrics.disk_total,
            metrics.disk_used,
            metrics.process_count,
            metrics.load_avg_1min,
            metrics.load_avg_5min,
            metrics.load_avg_15min,
        ],
    )?;
    Ok(())
}

/// Insert log entry into database
pub fn insert_log_entry(conn: &Connection, entry: &LogEntry) -> Result<()> {
    conn.execute(
        "INSERT INTO log_entries (timestamp, level, source, message)
         VALUES (?1, ?2, ?3, ?4)",
        params![
            entry.timestamp.timestamp(), // Unix timestamp in seconds
            entry.level,
            entry.source,
            entry.message,
        ],
    )?;
    Ok(())
}

/// Query system metrics within a time range
pub fn query_metrics(
    conn: &Connection,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
) -> Result<Vec<SystemMetrics>> {
    let mut stmt = conn.prepare(
        "SELECT timestamp, cpu_usage, memory_total, memory_used, memory_available,
                disk_total, disk_used, process_count,
                load_avg_1min, load_avg_5min, load_avg_15min
         FROM system_metrics
         WHERE timestamp >= ?1 AND timestamp <= ?2
         ORDER BY timestamp DESC",
    )?;

    let metrics_iter = stmt.query_map(params![start.timestamp(), end.timestamp()], |row| {
        let timestamp_i64: i64 = row.get(0)?;
        let timestamp = Utc.timestamp_opt(timestamp_i64, 0)
            .single()
            .ok_or_else(|| {
                rusqlite::Error::FromSqlConversionFailure(
                    0,
                    rusqlite::types::Type::Integer,
                    Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, "Invalid timestamp")),
                )
            })?;

        Ok(SystemMetrics {
            timestamp,
            cpu_usage: row.get(1)?,
            memory_total: row.get(2)?,
            memory_used: row.get(3)?,
            memory_available: row.get(4)?,
            disk_total: row.get(5)?,
            disk_used: row.get(6)?,
            process_count: row.get(7)?,
            load_avg_1min: row.get(8)?,
            load_avg_5min: row.get(9)?,
            load_avg_15min: row.get(10)?,
        })
    })?;

    let mut results = Vec::new();
    for metric in metrics_iter {
        results.push(metric?);
    }

    Ok(results)
}

/// Query log entries within a time range and optional level filter
pub fn query_logs(
    conn: &Connection,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
    level_filter: Option<&str>,
) -> Result<Vec<LogEntry>> {
    let parse_row = |row: &rusqlite::Row| -> rusqlite::Result<LogEntry> {
        let timestamp_i64: i64 = row.get(0)?;
        let timestamp = Utc.timestamp_opt(timestamp_i64, 0)
            .single()
            .ok_or_else(|| {
                rusqlite::Error::FromSqlConversionFailure(
                    0,
                    rusqlite::types::Type::Integer,
                    Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, "Invalid timestamp")),
                )
            })?;

        Ok(LogEntry {
            timestamp,
            level: row.get(1)?,
            source: row.get(2)?,
            message: row.get(3)?,
        })
    };

    // Build query and parameters based on filter
    let start_ts = start.timestamp();
    let end_ts = end.timestamp();

    let mut stmt;
    let logs_iter = if let Some(level) = level_filter {
        stmt = conn.prepare(
            "SELECT timestamp, level, source, message
             FROM log_entries
             WHERE timestamp >= ?1 AND timestamp <= ?2 AND level = ?3
             ORDER BY timestamp DESC",
        )?;
        stmt.query_map(params![start_ts, end_ts, level], parse_row)?
    } else {
        stmt = conn.prepare(
            "SELECT timestamp, level, source, message
             FROM log_entries
             WHERE timestamp >= ?1 AND timestamp <= ?2
             ORDER BY timestamp DESC",
        )?;
        stmt.query_map(params![start_ts, end_ts], parse_row)?
    };

    let mut results = Vec::new();
    for log in logs_iter {
        results.push(log?);
    }

    Ok(results)
}

/// Delete old data beyond the retention period
/// Returns tuple of (metrics_deleted, logs_deleted)
pub fn cleanup_old_data(conn: &Connection, retention_days: i64) -> Result<(usize, usize)> {
    let cutoff_date = chrono::Utc::now() - chrono::Duration::days(retention_days);
    let cutoff_ts = cutoff_date.timestamp();

    // Delete old metrics
    let metrics_deleted = conn.execute(
        "DELETE FROM system_metrics WHERE timestamp < ?1",
        params![cutoff_ts],
    )?;

    // Delete old log entries
    let logs_deleted = conn.execute(
        "DELETE FROM log_entries WHERE timestamp < ?1",
        params![cutoff_ts],
    )?;

    // Vacuum to reclaim space
    conn.execute("VACUUM", [])?;

    Ok((metrics_deleted, logs_deleted))
}
