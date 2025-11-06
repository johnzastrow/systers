pub mod collector;
pub mod config;
pub mod db;
pub mod reporter;
pub mod system_checks;

/// Application version from Cargo.toml
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Application name
pub const APP_NAME: &str = env!("CARGO_PKG_NAME");
