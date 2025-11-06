use crate::config::{
    CPU_WARNING_THRESHOLD, DISK_WARNING_THRESHOLD, ERROR_COUNT_THRESHOLD,
    LOAD_WARNING_THRESHOLD, MAX_RECENT_ERRORS_DISPLAY, MEMORY_WARNING_THRESHOLD,
};
use crate::db::{query_logs, query_metrics, LogEntry};
use anyhow::Result;
use chrono::{DateTime, Duration, Utc};
use rusqlite::Connection;

/// Report statistics for system metrics
#[derive(Debug)]
pub struct MetricsReport {
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
    pub avg_cpu_usage: f32,
    pub max_cpu_usage: f32,
    pub avg_memory_used_percent: f32,
    pub max_memory_used_percent: f32,
    pub avg_disk_used_percent: f32,
    pub max_disk_used_percent: f32,
    pub avg_process_count: usize,
    pub max_load_avg_1min: f64,
    pub issues: Vec<String>,
}

/// Report statistics for log entries
#[derive(Debug)]
pub struct LogReport {
    pub total_errors: usize,
    pub total_warnings: usize,
    pub total_critical: usize,
    pub recent_errors: Vec<LogEntry>,
}

/// Generate a comprehensive system report
pub fn generate_report(conn: &Connection, hours_back: i64) -> Result<(MetricsReport, LogReport)> {
    let end = Utc::now();
    let start = end - Duration::hours(hours_back);

    // Query metrics
    let metrics = query_metrics(conn, start, end)?;

    let metrics_report = if metrics.is_empty() {
        MetricsReport {
            period_start: start,
            period_end: end,
            avg_cpu_usage: 0.0,
            max_cpu_usage: 0.0,
            avg_memory_used_percent: 0.0,
            max_memory_used_percent: 0.0,
            avg_disk_used_percent: 0.0,
            max_disk_used_percent: 0.0,
            avg_process_count: 0,
            max_load_avg_1min: 0.0,
            issues: vec!["No data available for the specified time period".to_string()],
        }
    } else {
        // Calculate statistics
        let count = metrics.len() as f32;
        let avg_cpu = metrics.iter().map(|m| m.cpu_usage).sum::<f32>() / count;
        let max_cpu = metrics.iter().map(|m| m.cpu_usage).fold(0.0f32, f32::max);

        let avg_mem_pct = metrics
            .iter()
            .map(|m| (m.memory_used as f32 / m.memory_total as f32) * 100.0)
            .sum::<f32>()
            / count;
        let max_mem_pct = metrics
            .iter()
            .map(|m| (m.memory_used as f32 / m.memory_total as f32) * 100.0)
            .fold(0.0f32, f32::max);

        let avg_disk_pct = metrics
            .iter()
            .map(|m| (m.disk_used as f32 / m.disk_total as f32) * 100.0)
            .sum::<f32>()
            / count;
        let max_disk_pct = metrics
            .iter()
            .map(|m| (m.disk_used as f32 / m.disk_total as f32) * 100.0)
            .fold(0.0f32, f32::max);

        let avg_proc =
            (metrics.iter().map(|m| m.process_count).sum::<usize>() as f32 / count) as usize;
        let max_load = metrics
            .iter()
            .map(|m| m.load_avg_1min)
            .fold(0.0f64, f64::max);

        // Identify issues
        let mut issues = Vec::new();

        if max_cpu > CPU_WARNING_THRESHOLD {
            issues.push(format!(
                "⚠️  HIGH CPU USAGE: Peak CPU usage reached {:.1}%",
                max_cpu
            ));
        }
        if max_mem_pct > MEMORY_WARNING_THRESHOLD {
            issues.push(format!(
                "⚠️  HIGH MEMORY USAGE: Peak memory usage reached {:.1}%",
                max_mem_pct
            ));
        }
        if max_disk_pct > DISK_WARNING_THRESHOLD {
            issues.push(format!(
                "⚠️  HIGH DISK USAGE: Disk usage reached {:.1}%",
                max_disk_pct
            ));
        }
        if max_load > LOAD_WARNING_THRESHOLD {
            issues.push(format!(
                "⚠️  HIGH LOAD: System load average reached {:.2}",
                max_load
            ));
        }

        MetricsReport {
            period_start: start,
            period_end: end,
            avg_cpu_usage: avg_cpu,
            max_cpu_usage: max_cpu,
            avg_memory_used_percent: avg_mem_pct,
            max_memory_used_percent: max_mem_pct,
            avg_disk_used_percent: avg_disk_pct,
            max_disk_used_percent: max_disk_pct,
            avg_process_count: avg_proc,
            max_load_avg_1min: max_load,
            issues,
        }
    };

    // Query logs
    let all_logs = query_logs(conn, start, end, None)?;

    let total_errors = all_logs.iter().filter(|l| l.level == "ERROR").count();
    let total_warnings = all_logs.iter().filter(|l| l.level == "WARNING").count();
    let total_critical = all_logs.iter().filter(|l| l.level == "CRITICAL").count();

    let recent_errors: Vec<LogEntry> = all_logs
        .iter()
        .filter(|l| l.level == "ERROR" || l.level == "CRITICAL")
        .take(MAX_RECENT_ERRORS_DISPLAY)
        .cloned()
        .collect();

    let log_report = LogReport {
        total_errors,
        total_warnings,
        total_critical,
        recent_errors,
    };

    Ok((metrics_report, log_report))
}

/// Format report for terminal display
pub fn format_report(metrics: &MetricsReport, logs: &LogReport) -> String {
    let mut output = String::new();

    output.push_str("╔════════════════════════════════════════════════════════════════╗\n");
    output.push_str(&format!(
        "║         SYSTERS v{:<6} - SYSTEM ANALYSIS REPORT          ║\n",
        crate::VERSION
    ));
    output.push_str("╚════════════════════════════════════════════════════════════════╝\n\n");

    output.push_str(&format!(
        "Report Period: {} to {}\n\n",
        metrics.period_start.format("%Y-%m-%d %H:%M:%S UTC"),
        metrics.period_end.format("%Y-%m-%d %H:%M:%S UTC")
    ));

    output.push_str("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
    output.push_str("  SYSTEM METRICS\n");
    output.push_str("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n\n");

    output.push_str("CPU Usage:\n");
    output.push_str(&format!("  Average: {:.1}%\n", metrics.avg_cpu_usage));
    output.push_str(&format!("  Peak:    {:.1}%\n\n", metrics.max_cpu_usage));

    output.push_str("Memory Usage:\n");
    output.push_str(&format!(
        "  Average: {:.1}%\n",
        metrics.avg_memory_used_percent
    ));
    output.push_str(&format!(
        "  Peak:    {:.1}%\n\n",
        metrics.max_memory_used_percent
    ));

    output.push_str("Disk Usage:\n");
    output.push_str(&format!(
        "  Average: {:.1}%\n",
        metrics.avg_disk_used_percent
    ));
    output.push_str(&format!(
        "  Peak:    {:.1}%\n\n",
        metrics.max_disk_used_percent
    ));

    output.push_str("System Load:\n");
    output.push_str(&format!(
        "  Peak (1-min avg): {:.2}\n\n",
        metrics.max_load_avg_1min
    ));

    output.push_str(&format!(
        "Average Process Count: {}\n\n",
        metrics.avg_process_count
    ));

    output.push_str("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
    output.push_str("  LOG ANALYSIS\n");
    output.push_str("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n\n");

    output.push_str(&format!("Critical Issues: {}\n", logs.total_critical));
    output.push_str(&format!("Errors:          {}\n", logs.total_errors));
    output.push_str(&format!("Warnings:        {}\n\n", logs.total_warnings));

    if !logs.recent_errors.is_empty() {
        output.push_str("Recent Critical/Error Messages (up to 10):\n");
        for (i, entry) in logs.recent_errors.iter().enumerate() {
            output.push_str(&format!(
                "  {}. [{}] {}\n",
                i + 1,
                entry.level,
                entry.message.chars().take(80).collect::<String>()
            ));
        }
        output.push('\n');
    }

    if !metrics.issues.is_empty() {
        output.push_str("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
        output.push_str("  ⚠️  ISSUES DETECTED\n");
        output.push_str("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n\n");
        for issue in &metrics.issues {
            output.push_str(&format!("{}\n", issue));
        }
        output.push('\n');
    }

    output.push_str("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
    output.push_str("  RECOMMENDATIONS\n");
    output.push_str("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n\n");

    let mut recommendations = Vec::new();

    if metrics.max_cpu_usage > CPU_WARNING_THRESHOLD {
        recommendations.push("• Investigate high CPU usage - check for runaway processes");
    }
    if metrics.max_memory_used_percent > MEMORY_WARNING_THRESHOLD {
        recommendations
            .push("• Memory usage is high - consider freeing up memory or adding more RAM");
    }
    if metrics.max_disk_used_percent > DISK_WARNING_THRESHOLD {
        recommendations.push("• Disk space is running low - clean up old files or expand storage");
    }
    if logs.total_critical > 0 {
        recommendations.push("• Critical issues found in logs - review system logs immediately");
    }
    if logs.total_errors > ERROR_COUNT_THRESHOLD {
        recommendations.push("• Multiple errors detected - review system logs for patterns");
    }

    if recommendations.is_empty() {
        output.push_str("✓ System appears healthy - no immediate action required\n\n");
    } else {
        for rec in recommendations {
            output.push_str(&format!("{}\n", rec));
        }
        output.push('\n');
    }

    output
}
