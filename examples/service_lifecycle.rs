//! Example: Service Lifecycle
//!
//! Start, stop, and restart mihomo service.
//!
//! ## Running
//! ```bash
//! cargo run --example service_lifecycle
//! ```

use mihomo_rs::{ConfigManager, Result, ServiceManager, VersionManager};
use std::time::Duration;
use tokio::time::sleep;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    println!("=== Mihomo Service Lifecycle ===\n");

    let vm = VersionManager::new()?;
    let cm = ConfigManager::new()?;

    // Ensure setup
    cm.ensure_default_config().await?;
    cm.ensure_external_controller().await?;

    let binary = vm.get_binary_path(None).await?;
    let config = cm.get_current_path().await?;

    let sm = ServiceManager::new(binary, config);

    // Check current status
    println!("Checking current status...");
    if sm.is_running().await {
        println!("✓ Mihomo is running\n");

        println!("Stopping service...");
        sm.stop().await?;
        println!("✓ Service stopped");
        sleep(Duration::from_secs(1)).await;
    } else {
        println!("✗ Mihomo is not running\n");
    }

    // Start service
    println!("\nStarting service...");
    sm.start().await?;
    println!("✓ Service started");

    sleep(Duration::from_secs(2)).await;

    // Check status again
    if let Ok(status) = sm.status().await {
        println!("\nStatus: {:?}", status);
    }

    println!("\n=== Service Commands ===");
    println!("  - Start: sm.start().await?");
    println!("  - Stop: sm.stop().await?");
    println!("  - Restart: sm.restart().await?");
    println!("  - Status: sm.status().await?");

    Ok(())
}
