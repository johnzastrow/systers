use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection};
use std::path::Path;

/// Database schema version
pub const SCHEMA_VERSION: i32 = 1;

/// System metrics record
#[derive(Debug, Clone)]
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
#[derive(Debug, Clone)]
pub struct LogEntry {
    pub timestamp: DateTime<Utc>,
    pub level: String,
    pub source: String,
    pub message: String,
}

/// Initialize the database with required schema
pub fn init_database<P: AsRef<Path>>(db_path: P) -> Result<Connection> {
    let path_ref = db_path.as_ref();
    let conn = Connection::open(path_ref).context("Failed to open database")?;

    // Create schema
    conn.execute(
        "CREATE TABLE IF NOT EXISTS schema_version (
            version INTEGER PRIMARY KEY,
            app_version TEXT
        )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS system_metrics (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            timestamp TEXT NOT NULL,
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
            timestamp TEXT NOT NULL,
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
            metrics.timestamp.to_rfc3339(),
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
            entry.timestamp.to_rfc3339(),
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

    let metrics_iter = stmt.query_map(params![start.to_rfc3339(), end.to_rfc3339()], |row| {
        let timestamp_str: String = row.get(0)?;
        let timestamp = DateTime::parse_from_rfc3339(&timestamp_str)
            .map_err(|e| {
                rusqlite::Error::FromSqlConversionFailure(
                    0,
                    rusqlite::types::Type::Text,
                    Box::new(e),
                )
            })?
            .with_timezone(&Utc);

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
    let mut results = Vec::new();

    let parse_row = |row: &rusqlite::Row| -> rusqlite::Result<LogEntry> {
        let timestamp_str: String = row.get(0)?;
        let timestamp = DateTime::parse_from_rfc3339(&timestamp_str)
            .map_err(|e| {
                rusqlite::Error::FromSqlConversionFailure(
                    0,
                    rusqlite::types::Type::Text,
                    Box::new(e),
                )
            })?
            .with_timezone(&Utc);

        Ok(LogEntry {
            timestamp,
            level: row.get(1)?,
            source: row.get(2)?,
            message: row.get(3)?,
        })
    };

    if let Some(level) = level_filter {
        let mut stmt = conn.prepare(
            "SELECT timestamp, level, source, message
             FROM log_entries 
             WHERE timestamp >= ?1 AND timestamp <= ?2 AND level = ?3
             ORDER BY timestamp DESC",
        )?;

        let logs_iter = stmt.query_map(
            params![start.to_rfc3339(), end.to_rfc3339(), level],
            parse_row,
        )?;

        for log in logs_iter {
            results.push(log?);
        }
    } else {
        let mut stmt = conn.prepare(
            "SELECT timestamp, level, source, message
             FROM log_entries 
             WHERE timestamp >= ?1 AND timestamp <= ?2
             ORDER BY timestamp DESC",
        )?;

        let logs_iter = stmt.query_map(params![start.to_rfc3339(), end.to_rfc3339()], parse_row)?;

        for log in logs_iter {
            results.push(log?);
        }
    }

    Ok(results)
}

/// Delete old data beyond the retention period
/// Returns tuple of (metrics_deleted, logs_deleted)
pub fn cleanup_old_data(conn: &Connection, retention_days: i64) -> Result<(usize, usize)> {
    let cutoff_date = chrono::Utc::now() - chrono::Duration::days(retention_days);
    let cutoff_str = cutoff_date.to_rfc3339();

    // Delete old metrics
    let metrics_deleted = conn.execute(
        "DELETE FROM system_metrics WHERE timestamp < ?1",
        params![cutoff_str],
    )?;

    // Delete old log entries
    let logs_deleted = conn.execute(
        "DELETE FROM log_entries WHERE timestamp < ?1",
        params![cutoff_str],
    )?;

    // Vacuum to reclaim space
    conn.execute("VACUUM", [])?;

    Ok((metrics_deleted, logs_deleted))
}
