//! Example: Auto Restart
//!
//! Automatic service restart with health checking.
//!
//! ## Running
//! ```bash
//! cargo run --example auto_restart
//! ```

use mihomo_rs::{ConfigManager, MihomoClient, Result, ServiceManager, VersionManager};
use std::time::Duration;
use tokio::time::sleep;

async fn is_healthy(controller_url: &str) -> bool {
    if let Ok(client) = MihomoClient::new(controller_url, None) {
        client.get_version().await.is_ok()
    } else {
        false
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    println!("=== Auto Restart with Health Check ===\n");

    let vm = VersionManager::new()?;
    let cm = ConfigManager::new()?;

    cm.ensure_default_config().await?;
    let controller_url = cm.ensure_external_controller().await?;

    let binary = vm.get_binary_path(None).await?;
    let config = cm.get_current_path().await?;

    let sm = ServiceManager::new(binary, config);

    println!("Performing health check...");
    if is_healthy(&controller_url).await {
        println!("✓ Service is healthy\n");
    } else {
        println!("✗ Service is unhealthy");
        println!("Attempting restart...\n");

        sm.restart().await?;
        println!("✓ Service restarted");

        // Wait and check again
        sleep(Duration::from_secs(3)).await;
        if is_healthy(&controller_url).await {
            println!("✓ Service is now healthy");
        } else {
            println!("✗ Service still unhealthy after restart");
        }
    }

    println!("\n=== Health Check Logic ===");
    println!("This example demonstrates:");
    println!("  1. Check if service responds to API calls");
    println!("  2. If unhealthy, restart the service");
    println!("  3. Verify health after restart");
    println!("\nUse this pattern for production monitoring!");

    Ok(())
}
