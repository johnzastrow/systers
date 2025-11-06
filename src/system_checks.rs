use anyhow::{Context, Result};
use log::{debug, info, warn};
use std::process::Command;

/// Information about an external tool
#[derive(Debug, Clone)]
pub struct ExternalTool {
    pub name: &'static str,
    pub description: &'static str,
    pub command: &'static str,
    pub install_hint: &'static str,
}

/// Available external tools for enhanced system checks
pub const EXTERNAL_TOOLS: &[ExternalTool] = &[
    ExternalTool {
        name: "smartctl",
        description: "SMART disk health monitoring",
        command: "smartctl",
        install_hint: "sudo apt install smartmontools  # Debian/Ubuntu\nsudo dnf install smartmontools  # RHEL/Fedora",
    },
    ExternalTool {
        name: "journalctl",
        description: "Systemd journal log access",
        command: "journalctl",
        install_hint: "Usually pre-installed with systemd",
    },
    ExternalTool {
        name: "apt",
        description: "Package update checks (Debian/Ubuntu)",
        command: "apt",
        install_hint: "Pre-installed on Debian/Ubuntu systems",
    },
    ExternalTool {
        name: "dnf",
        description: "Package update checks (RHEL/Fedora)",
        command: "dnf",
        install_hint: "Pre-installed on RHEL/Fedora systems",
    },
    ExternalTool {
        name: "df",
        description: "Disk space usage",
        command: "df",
        install_hint: "Pre-installed (coreutils)",
    },
    ExternalTool {
        name: "du",
        description: "Directory size analysis",
        command: "du",
        install_hint: "Pre-installed (coreutils)",
    },
];

/// Check if a command is available in PATH
pub fn is_command_available(command: &str) -> bool {
    Command::new("which")
        .arg(command)
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

/// Detect which external tools are available on the system
pub fn detect_available_tools() -> Vec<&'static ExternalTool> {
    EXTERNAL_TOOLS
        .iter()
        .filter(|tool| is_command_available(tool.command))
        .collect()
}

/// Print information about missing tools that could enhance monitoring
pub fn print_missing_tools_info() {
    let available: Vec<&str> = detect_available_tools()
        .iter()
        .map(|t| t.name)
        .collect();

    let missing: Vec<&ExternalTool> = EXTERNAL_TOOLS
        .iter()
        .filter(|tool| !available.contains(&tool.name))
        .collect();

    if !missing.is_empty() {
        info!("Optional tools available for enhanced monitoring:");
        for tool in &missing {
            info!("  - {}: {}", tool.name, tool.description);
            debug!("    Install: {}", tool.install_hint);
        }
    }
}

/// Result of a package update check
#[derive(Debug, Clone)]
pub struct PackageUpdateInfo {
    pub total_packages: usize,
    pub updates_available: usize,
    pub security_updates: usize,
    pub package_manager: String,
}

/// Check for available package updates (Debian/Ubuntu - apt)
pub fn check_apt_updates() -> Result<PackageUpdateInfo> {
    if !is_command_available("apt") {
        return Err(anyhow::anyhow!("apt command not available"));
    }

    // Update package lists quietly
    debug!("Updating apt package lists...");
    let update_result = Command::new("apt")
        .args(&["update", "-qq"])
        .output()
        .context("Failed to run apt update")?;

    if !update_result.status.success() {
        warn!("apt update failed, may need sudo privileges");
    }

    // Check for upgradable packages
    let output = Command::new("apt")
        .args(&["list", "--upgradable"])
        .output()
        .context("Failed to run apt list --upgradable")?;

    if !output.status.success() {
        return Err(anyhow::anyhow!("apt list command failed"));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.lines().collect();

    // First line is usually "Listing..." so we skip it
    let updates_available = lines.len().saturating_sub(1);

    // Check for security updates (this is a simplified check)
    let security_updates = lines
        .iter()
        .filter(|line| line.contains("security"))
        .count();

    Ok(PackageUpdateInfo {
        total_packages: 0, // Would need dpkg -l | wc -l for this
        updates_available,
        security_updates,
        package_manager: "apt".to_string(),
    })
}

/// Check for available package updates (RHEL/Fedora - dnf)
pub fn check_dnf_updates() -> Result<PackageUpdateInfo> {
    if !is_command_available("dnf") {
        return Err(anyhow::anyhow!("dnf command not available"));
    }

    let output = Command::new("dnf")
        .args(&["check-update", "-q"])
        .output()
        .context("Failed to run dnf check-update")?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.lines().filter(|l| !l.trim().is_empty()).collect();

    let updates_available = lines.len();

    // Check for security updates
    let security_output = Command::new("dnf")
        .args(&["updateinfo", "list", "security", "-q"])
        .output()
        .context("Failed to run dnf updateinfo")?;

    let security_stdout = String::from_utf8_lossy(&security_output.stdout);
    let security_updates = security_stdout.lines().count();

    Ok(PackageUpdateInfo {
        total_packages: 0,
        updates_available,
        security_updates,
        package_manager: "dnf".to_string(),
    })
}

/// Check for available package updates (auto-detect package manager)
pub fn check_package_updates() -> Result<PackageUpdateInfo> {
    if is_command_available("apt") {
        check_apt_updates()
    } else if is_command_available("dnf") {
        check_dnf_updates()
    } else {
        Err(anyhow::anyhow!(
            "No supported package manager found (apt or dnf)"
        ))
    }
}

/// SMART disk health status
#[derive(Debug, Clone)]
pub struct DiskHealthInfo {
    pub device: String,
    pub health_status: String,
    pub temperature: Option<i32>,
    pub power_on_hours: Option<u64>,
    pub reallocated_sectors: Option<u64>,
}

/// Check SMART disk health using smartctl
pub fn check_disk_health() -> Result<Vec<DiskHealthInfo>> {
    if !is_command_available("smartctl") {
        return Err(anyhow::anyhow!(
            "smartctl not available. Install with: sudo apt install smartmontools"
        ));
    }

    // Get list of disks
    let output = Command::new("lsblk")
        .args(&["-d", "-n", "-o", "NAME,TYPE"])
        .output()
        .context("Failed to list block devices")?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let disks: Vec<String> = stdout
        .lines()
        .filter(|line| line.contains("disk"))
        .map(|line| {
            let name = line.split_whitespace().next().unwrap_or("");
            format!("/dev/{}", name)
        })
        .collect();

    let mut results = Vec::new();

    for disk in disks {
        debug!("Checking SMART status for {}", disk);

        // This requires sudo, so it might fail
        let output = Command::new("smartctl")
            .args(&["-H", "-A", &disk])
            .output();

        if let Ok(output) = output {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);

                let health_status = if stdout.contains("PASSED") {
                    "PASSED".to_string()
                } else if stdout.contains("FAILED") {
                    "FAILED".to_string()
                } else {
                    "UNKNOWN".to_string()
                };

                results.push(DiskHealthInfo {
                    device: disk.clone(),
                    health_status,
                    temperature: None,
                    power_on_hours: None,
                    reallocated_sectors: None,
                });
            }
        } else {
            warn!("Could not check SMART status for {} (may need sudo)", disk);
        }
    }

    Ok(results)
}

/// Systemd service status
#[derive(Debug, Clone)]
pub struct SystemdServiceStatus {
    pub total_services: usize,
    pub active_services: usize,
    pub failed_services: usize,
    pub failed_service_names: Vec<String>,
}

/// Check systemd service status
pub fn check_systemd_status() -> Result<SystemdServiceStatus> {
    if !is_command_available("systemctl") {
        return Err(anyhow::anyhow!("systemctl not available (not a systemd system?)"));
    }

    // Get failed services
    let output = Command::new("systemctl")
        .args(&["--failed", "--no-pager", "--plain", "--no-legend"])
        .output()
        .context("Failed to run systemctl --failed")?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let failed_services: Vec<String> = stdout
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            line.split_whitespace()
                .next()
                .unwrap_or("")
                .to_string()
        })
        .collect();

    let failed_count = failed_services.len();

    // Get total service count
    let list_output = Command::new("systemctl")
        .args(&["list-units", "--type=service", "--all", "--no-pager", "--plain", "--no-legend"])
        .output()
        .context("Failed to list services")?;

    let list_stdout = String::from_utf8_lossy(&list_output.stdout);
    let total_services = list_stdout.lines().count();

    let active_output = Command::new("systemctl")
        .args(&["list-units", "--type=service", "--state=active", "--no-pager", "--plain", "--no-legend"])
        .output()
        .context("Failed to list active services")?;

    let active_stdout = String::from_utf8_lossy(&active_output.stdout);
    let active_services = active_stdout.lines().count();

    Ok(SystemdServiceStatus {
        total_services,
        active_services,
        failed_services: failed_count,
        failed_service_names: failed_services,
    })
}

/// Top directories by size
#[derive(Debug, Clone)]
pub struct DirectorySizeInfo {
    pub path: String,
    pub size_bytes: u64,
    pub size_human: String,
}

/// Find top directories by size (limited depth)
pub fn find_large_directories(base_path: &str, depth: usize, limit: usize) -> Result<Vec<DirectorySizeInfo>> {
    if !is_command_available("du") {
        return Err(anyhow::anyhow!("du command not available"));
    }

    let output = Command::new("du")
        .args(&[
            "-d",
            &depth.to_string(),
            "-x", // Don't cross filesystem boundaries
            base_path,
        ])
        .output()
        .context("Failed to run du command")?;

    if !output.status.success() {
        return Err(anyhow::anyhow!("du command failed"));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut dirs: Vec<DirectorySizeInfo> = stdout
        .lines()
        .filter_map(|line| {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                let size_kb = parts[0].parse::<u64>().ok()?;
                let size_bytes = size_kb * 1024;
                let path = parts[1..].join(" ");

                Some(DirectorySizeInfo {
                    path,
                    size_bytes,
                    size_human: format_bytes(size_bytes),
                })
            } else {
                None
            }
        })
        .collect();

    // Sort by size descending
    dirs.sort_by(|a, b| b.size_bytes.cmp(&a.size_bytes));
    dirs.truncate(limit);

    Ok(dirs)
}

/// Format bytes to human-readable format
fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit_idx = 0;

    while size >= 1024.0 && unit_idx < UNITS.len() - 1 {
        size /= 1024.0;
        unit_idx += 1;
    }

    format!("{:.2} {}", size, UNITS[unit_idx])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_command_available() {
        // These should be available on most systems
        assert!(is_command_available("ls"));
        assert!(is_command_available("echo"));

        // This should definitely not exist
        assert!(!is_command_available("this_command_definitely_does_not_exist_12345"));
    }

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(500), "500.00 B");
        assert_eq!(format_bytes(1024), "1.00 KB");
        assert_eq!(format_bytes(1024 * 1024), "1.00 MB");
        assert_eq!(format_bytes(1024 * 1024 * 1024), "1.00 GB");
    }

    #[test]
    fn test_detect_available_tools() {
        let tools = detect_available_tools();
        // Should find at least some basic tools
        assert!(!tools.is_empty());
    }
}
