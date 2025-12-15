// Common test utilities
#![allow(dead_code)]

use std::path::PathBuf;
use tempfile::TempDir;

/// Create a temporary home directory for testing
pub fn setup_temp_home() -> TempDir {
    tempfile::tempdir().expect("Failed to create temp dir")
}

/// Get the temp home path as PathBuf
pub fn get_temp_home_path(temp_dir: &TempDir) -> PathBuf {
    temp_dir.path().to_path_buf()
}

/// Create a test config content
pub fn create_test_config() -> String {
    r#"port: 7890
socks-port: 7891
allow-lan: false
mode: rule
log-level: info
external-controller: 127.0.0.1:9090
"#
    .to_string()
}

/// Create a test config with custom port
pub fn create_test_config_with_port(port: u16) -> String {
    format!(
        r#"port: 7890
socks-port: 7891
allow-lan: false
mode: rule
log-level: info
external-controller: 127.0.0.1:{}
"#,
        port
    )
}
