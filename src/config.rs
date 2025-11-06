use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

/// Main configuration structure for Systers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub database: DatabaseConfig,
    pub thresholds: ThresholdsConfig,
    pub collection: CollectionConfig,
    pub display: DisplayConfig,
    pub retention: RetentionConfig,
}

/// Database configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    /// Path to the SQLite database file
    /// Can be overridden by SYSTERS_DB_PATH env var or --db-path CLI flag
    pub path: PathBuf,
}

/// Issue detection thresholds
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThresholdsConfig {
    /// CPU usage percentage threshold for warning alerts (0.0-100.0)
    pub cpu_warning: f32,

    /// Memory usage percentage threshold for warning alerts (0.0-100.0)
    pub memory_warning: f32,

    /// Disk usage percentage threshold for warning alerts (0.0-100.0)
    pub disk_warning: f32,

    /// System load average threshold for warning alerts
    pub load_warning: f64,

    /// Minimum number of errors before triggering a "multiple errors" recommendation
    pub error_count: usize,
}

/// Data collection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionConfig {
    /// Log file paths to scan (Debian defaults)
    /// Can be overridden by SYSTERS_LOG_PATHS env var or --log-paths CLI flag
    pub log_paths: Vec<PathBuf>,

    /// Maximum number of log lines to read from each file
    pub max_log_lines_per_file: usize,

    /// CPU measurement delay in milliseconds
    /// The sysinfo crate needs at least one refresh cycle to compute CPU percentage
    pub cpu_measurement_delay_ms: u64,
}

/// Display configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisplayConfig {
    /// Maximum number of recent error messages to display in reports
    pub max_recent_errors: usize,
}

/// Data retention configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetentionConfig {
    /// Number of days to retain historical data
    pub days: i64,
}

impl Default for Config {
    /// Default configuration optimized for Debian/Ubuntu systems
    fn default() -> Self {
        Config {
            database: DatabaseConfig {
                path: Self::default_db_path(),
            },
            thresholds: ThresholdsConfig {
                cpu_warning: 90.0,
                memory_warning: 90.0,
                disk_warning: 85.0,
                load_warning: 5.0,
                error_count: 10,
            },
            collection: CollectionConfig {
                log_paths: vec![
                    PathBuf::from("/var/log/syslog"),
                    PathBuf::from("/var/log/messages"),
                    PathBuf::from("/var/log/kern.log"),
                    PathBuf::from("/var/log/auth.log"),
                ],
                max_log_lines_per_file: 1000,
                cpu_measurement_delay_ms: 200,
            },
            display: DisplayConfig {
                max_recent_errors: 10,
            },
            retention: RetentionConfig {
                days: 30,
            },
        }
    }
}

impl Config {
    /// Get the default database path (~/.systers.db or /tmp/.systers.db)
    fn default_db_path() -> PathBuf {
        let home = env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
        PathBuf::from(home).join(".systers.db")
    }

    /// Load configuration from a YAML file
    /// Falls back to default configuration if file doesn't exist or can't be read
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();

        if !path.exists() {
            return Ok(Self::default());
        }

        let contents = fs::read_to_string(path)
            .context(format!("Failed to read config file: {}", path.display()))?;

        let config: Config = serde_yaml::from_str(&contents)
            .context(format!("Failed to parse config file: {}", path.display()))?;

        Ok(config)
    }

    /// Load configuration from default locations in order:
    /// 1. ./systers.yaml (current directory)
    /// 2. ~/.config/systers/config.yaml (user config)
    /// 3. /etc/systers/config.yaml (system config)
    /// 4. Built-in defaults
    pub fn load() -> Result<Self> {
        // Try current directory
        if let Ok(config) = Self::load_from_file("systers.yaml") {
            return Ok(config);
        }

        // Try user config directory
        if let Ok(home) = env::var("HOME") {
            let user_config = PathBuf::from(home).join(".config/systers/config.yaml");
            if user_config.exists() {
                return Self::load_from_file(&user_config);
            }
        }

        // Try system config directory
        let system_config = PathBuf::from("/etc/systers/config.yaml");
        if system_config.exists() {
            return Self::load_from_file(&system_config);
        }

        // Use defaults
        Ok(Self::default())
    }

    /// Save configuration to a YAML file
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let path = path.as_ref();
        let yaml = serde_yaml::to_string(self)
            .context("Failed to serialize configuration")?;

        // Create parent directory if it doesn't exist
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .context(format!("Failed to create config directory: {}", parent.display()))?;
        }

        fs::write(path, yaml)
            .context(format!("Failed to write config file: {}", path.display()))?;

        Ok(())
    }
}

// Backward compatibility: export constants from config
// These can be removed in a future version once all code uses Config struct
pub const CPU_WARNING_THRESHOLD: f32 = 90.0;
pub const MEMORY_WARNING_THRESHOLD: f32 = 90.0;
pub const DISK_WARNING_THRESHOLD: f32 = 85.0;
pub const LOAD_WARNING_THRESHOLD: f64 = 5.0;
pub const MAX_LOG_LINES_PER_FILE: usize = 1000;
pub const MAX_RECENT_ERRORS_DISPLAY: usize = 10;
pub const CPU_MEASUREMENT_DELAY_MS: u64 = 200;
pub const ERROR_COUNT_THRESHOLD: usize = 10;
pub const DEFAULT_RETENTION_DAYS: i64 = 30;

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_default_config() {
        let config = Config::default();

        assert_eq!(config.thresholds.cpu_warning, 90.0);
        assert_eq!(config.thresholds.memory_warning, 90.0);
        assert_eq!(config.thresholds.disk_warning, 85.0);
        assert_eq!(config.collection.max_log_lines_per_file, 1000);
        assert_eq!(config.retention.days, 30);
    }

    #[test]
    fn test_save_and_load_config() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let config_path = temp_dir.path().join("test_config.yaml");

        // Create a custom config
        let mut config = Config::default();
        config.thresholds.cpu_warning = 95.0;
        config.retention.days = 60;

        // Save it
        config.save_to_file(&config_path)?;

        // Load it back
        let loaded_config = Config::load_from_file(&config_path)?;

        assert_eq!(loaded_config.thresholds.cpu_warning, 95.0);
        assert_eq!(loaded_config.retention.days, 60);

        Ok(())
    }

    #[test]
    fn test_load_nonexistent_file_returns_default() -> Result<()> {
        let config = Config::load_from_file("/nonexistent/path/config.yaml")?;

        // Should return default config without error
        assert_eq!(config.thresholds.cpu_warning, 90.0);

        Ok(())
    }
}
