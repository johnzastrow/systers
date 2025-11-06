use crate::config::{
    CPU_WARNING_THRESHOLD, DISK_WARNING_THRESHOLD, ERROR_COUNT_THRESHOLD, LOAD_WARNING_THRESHOLD,
    MAX_RECENT_ERRORS_DISPLAY, MEMORY_WARNING_THRESHOLD,
};
use crate::db::{query_logs, query_metrics, query_system_checks, LogEntry, SystemCheckResult};
use anyhow::{Context, Result};
use chrono::{DateTime, Duration, Local, Utc};
use rusqlite::Connection;
use serde::Serialize;

/// Report statistics for system metrics
#[derive(Debug, Serialize)]
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
#[derive(Debug, Serialize)]
pub struct LogReport {
    pub total_errors: usize,
    pub total_warnings: usize,
    pub total_critical: usize,
    pub recent_errors: Vec<LogEntry>,
}

/// Report for system checks
#[derive(Debug, Serialize, Clone)]
pub struct SystemChecksReport {
    pub checks: Vec<SystemCheckResult>,
    pub total_checks: usize,
    pub checks_ok: usize,
    pub checks_warning: usize,
    pub checks_critical: usize,
    pub checks_error: usize,
}

/// Generate a comprehensive system report
pub fn generate_report(
    conn: &Connection,
    hours_back: i64,
) -> Result<(MetricsReport, LogReport, SystemChecksReport)> {
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

    // Query system checks
    let system_checks = query_system_checks(conn, start, end).unwrap_or_else(|_| Vec::new());

    let checks_ok = system_checks.iter().filter(|c| c.status == "ok").count();
    let checks_warning = system_checks
        .iter()
        .filter(|c| c.status == "warning")
        .count();
    let checks_critical = system_checks
        .iter()
        .filter(|c| c.status == "critical")
        .count();
    let checks_error = system_checks
        .iter()
        .filter(|c| c.status == "error")
        .count();

    let system_checks_report = SystemChecksReport {
        total_checks: system_checks.len(),
        checks_ok,
        checks_warning,
        checks_critical,
        checks_error,
        checks: system_checks,
    };

    Ok((metrics_report, log_report, system_checks_report))
}

/// Format report for terminal display
pub fn format_report(
    metrics: &MetricsReport,
    logs: &LogReport,
    system_checks: &SystemChecksReport,
) -> String {
    let mut output = String::new();

    output.push_str("╔════════════════════════════════════════════════════════════════╗\n");
    output.push_str(&format!(
        "║         SYSTERS v{:<6} - SYSTEM ANALYSIS REPORT          ║\n",
        crate::VERSION
    ));
    output.push_str("╚════════════════════════════════════════════════════════════════╝\n\n");

    // Convert to local time for display
    let local_start: DateTime<Local> = metrics.period_start.into();
    let local_end: DateTime<Local> = metrics.period_end.into();

    output.push_str(&format!(
        "Report Period: {} to {}\n\n",
        local_start.format("%Y-%m-%d %H:%M:%S %Z"),
        local_end.format("%Y-%m-%d %H:%M:%S %Z")
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
            let local_time: DateTime<Local> = entry.timestamp.into();
            output.push_str(&format!(
                "  {}. [{}] {} ({})\n      {}\n",
                i + 1,
                entry.level,
                local_time.format("%Y-%m-%d %H:%M:%S"),
                entry.source,
                entry.message.chars().take(100).collect::<String>()
            ));
        }
        output.push('\n');
    }

    // System checks section
    if system_checks.total_checks > 0 {
        output.push_str("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
        output.push_str("  ENHANCED SYSTEM CHECKS\n");
        output.push_str("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n\n");

        output.push_str(&format!(
            "Total Checks: {} (✓ {} OK, ⚠️  {} Warning, 🔴 {} Critical, ✗ {} Error)\n\n",
            system_checks.total_checks,
            system_checks.checks_ok,
            system_checks.checks_warning,
            system_checks.checks_critical,
            system_checks.checks_error
        ));

        // Group checks by type
        let mut check_types: std::collections::HashMap<String, Vec<&SystemCheckResult>> =
            std::collections::HashMap::new();
        for check in &system_checks.checks {
            check_types
                .entry(check.check_name.clone())
                .or_insert_with(Vec::new)
                .push(check);
        }

        for (check_name, checks) in check_types.iter() {
            // Show most recent check for each type
            if let Some(latest_check) = checks.last() {
                let status_icon = match latest_check.status.as_str() {
                    "ok" => "✓",
                    "warning" => "⚠️",
                    "critical" => "🔴",
                    "error" => "✗",
                    _ => "•",
                };

                let local_time: DateTime<Local> = latest_check.timestamp.into();
                output.push_str(&format!(
                    "{} {} [{}] ({})\n",
                    status_icon,
                    check_name,
                    latest_check.status.to_uppercase(),
                    local_time.format("%Y-%m-%d %H:%M:%S")
                ));
                output.push_str(&format!("   {}\n", latest_check.message));

                // If there are warnings/critical/errors, add them to issues
                if latest_check.status == "warning"
                    || latest_check.status == "critical"
                    || latest_check.status == "error"
                {
                    output.push('\n');
                }
            }
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

/// Combined report structure for export
#[derive(Debug, Serialize)]
pub struct FullReport {
    pub version: String,
    pub metrics: MetricsReport,
    pub logs: LogReport,
    pub system_checks: SystemChecksReport,
}

/// Export format for reports
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ExportFormat {
    Json,
    Text,
}

impl ExportFormat {
    pub fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "json" => Ok(ExportFormat::Json),
            "text" | "txt" => Ok(ExportFormat::Text),
            _ => Err(anyhow::anyhow!("Unsupported format: {}", s)),
        }
    }
}

/// Export report in the specified format
pub fn export_report(
    metrics: &MetricsReport,
    logs: &LogReport,
    system_checks: &SystemChecksReport,
    format: ExportFormat,
) -> Result<String> {
    match format {
        ExportFormat::Json => {
            let full_report = FullReport {
                version: crate::VERSION.to_string(),
                metrics: metrics.clone(),
                logs: logs.clone(),
                system_checks: system_checks.clone(),
            };
            serde_json::to_string_pretty(&full_report)
                .context("Failed to serialize report to JSON")
        }
        ExportFormat::Text => Ok(format_report(metrics, logs, system_checks)),
    }
}

impl Clone for MetricsReport {
    fn clone(&self) -> Self {
        MetricsReport {
            period_start: self.period_start,
            period_end: self.period_end,
            avg_cpu_usage: self.avg_cpu_usage,
            max_cpu_usage: self.max_cpu_usage,
            avg_memory_used_percent: self.avg_memory_used_percent,
            max_memory_used_percent: self.max_memory_used_percent,
            avg_disk_used_percent: self.avg_disk_used_percent,
            max_disk_used_percent: self.max_disk_used_percent,
            avg_process_count: self.avg_process_count,
            max_load_avg_1min: self.max_load_avg_1min,
            issues: self.issues.clone(),
        }
    }
}

impl Clone for LogReport {
    fn clone(&self) -> Self {
        LogReport {
            total_errors: self.total_errors,
            total_warnings: self.total_warnings,
            total_critical: self.total_critical,
            recent_errors: self.recent_errors.clone(),
        }
    }
}
