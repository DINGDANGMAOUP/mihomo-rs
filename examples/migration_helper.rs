//! Example: Migration Helper
//!
//! Migrate from manual mihomo setup to mihomo-rs management.
//!
//! ## Running
//! ```bash
//! cargo run --example migration_helper
//! ```

use mihomo_rs::{ConfigManager, Result, VersionManager};
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    println!("=== Migration Helper ===\n");
    println!("Migrate from manual mihomo setup to mihomo-rs.\n");

    // Check for existing manual installation
    let possible_locations = vec![
        PathBuf::from("/usr/local/bin/mihomo"),
        PathBuf::from("/usr/bin/mihomo"),
        PathBuf::from(std::env::var("HOME").unwrap_or_default()).join(".local/bin/mihomo"),
    ];

    println!("Checking for existing mihomo installations:");
    for location in &possible_locations {
        if location.exists() {
            println!("  ✓ Found: {}", location.display());
        }
    }
    println!();

    // Check for existing config
    let config_locations = vec![
        PathBuf::from("/etc/mihomo/config.yaml"),
        PathBuf::from(std::env::var("HOME").unwrap_or_default()).join(".config/mihomo/config.yaml"),
    ];

    println!("Checking for existing configurations:");
    for location in &config_locations {
        if location.exists() {
            println!("  ✓ Found: {}", location.display());
        }
    }
    println!();

    // Set up mihomo-rs
    println!("Setting up mihomo-rs management:");
    let vm = VersionManager::new()?;
    let cm = ConfigManager::new()?;

    println!("  ✓ Version manager ready");
    println!("  ✓ Config manager ready");
    println!();

    println!("=== Migration Steps ===\n");
    println!("1. Import existing config:");
    println!("   Copy your config to: ~/.config/mihomo-rs/configs/");
    println!("   cp /etc/mihomo/config.yaml ~/.config/mihomo-rs/configs/default.yaml");
    println!();

    println!("2. Install mihomo via mihomo-rs:");
    println!("   cargo run --example install_by_channel");
    println!();

    println!("3. Stop old service (if running):");
    println!("   sudo systemctl stop mihomo  # or your init system");
    println!();

    println!("4. Start with mihomo-rs:");
    println!("   cargo run --example service_lifecycle");
    println!();

    println!("5. Verify:");
    println!("   cargo run --example list_proxies");

    Ok(())
}
