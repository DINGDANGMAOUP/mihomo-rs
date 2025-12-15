//! Example: First Time Setup
//!
//! Complete first-time setup guide for new users.
//!
//! ## Running
//! ```bash
//! cargo run --example first_time_setup
//! ```

use mihomo_rs::{Channel, ConfigManager, Result, ServiceManager, VersionManager};

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    println!("=== First Time Setup Guide ===\n");
    println!("This will set up mihomo-rs from scratch.\n");

    // Step 1: Check for existing installation
    println!("Step 1: Checking for existing installation...");
    let vm = VersionManager::new()?;
    let versions = vm.list_installed().await?;

    if versions.is_empty() {
        println!("  No installation found.");
        println!("  Installing latest stable version...");

        match vm.install_channel(Channel::Stable).await {
            Ok(version) => {
                println!("  ✓ Installed mihomo {}", version);
            }
            Err(e) => {
                eprintln!("  ✗ Installation failed: {}", e);
                eprintln!("\nPlease check your internet connection and try again.");
                return Err(e);
            }
        }
    } else {
        println!("  ✓ Found existing installation");
        let default_version = vm.get_default().await?;
        println!("  Default version: {}", default_version);
    }
    println!();

    // Step 2: Configuration
    println!("Step 2: Setting up configuration...");
    let cm = ConfigManager::new()?;
    cm.ensure_default_config().await?;
    println!("  ✓ Default configuration created");

    let controller_url = cm.ensure_external_controller().await?;
    println!("  ✓ External controller: {}", controller_url);
    println!();

    // Step 3: Start service
    println!("Step 3: Starting service...");
    let binary = vm.get_binary_path(None).await?;
    let config = cm.get_current_path().await?;
    let sm = ServiceManager::new(binary, config);

    if sm.is_running().await {
        println!("  Service already running");
    } else {
        sm.start().await?;
        println!("  ✓ Service started");
    }
    println!();

    // Done!
    println!("=== Setup Complete! ===\n");
    println!("Mihomo is now installed and running.");
    println!("\nNext steps:");
    println!("  1. Configure your proxies in ~/.config/mihomo-rs/configs/default.yaml");
    println!("  2. List proxies: cargo run --example list_proxies");
    println!("  3. Switch proxy: cargo run --example switch_proxy");
    println!("  4. Monitor traffic: cargo run --example stream_traffic");

    Ok(())
}
