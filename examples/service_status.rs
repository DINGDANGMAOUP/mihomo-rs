//! Example: Service Status
//!
//! Check mihomo service status and PID information.
//!
//! ## Running
//! ```bash
//! cargo run --example service_status
//! ```

use mihomo_rs::{ConfigManager, Result, ServiceManager, ServiceStatus, VersionManager};

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    println!("=== Mihomo Service Status ===\n");

    let vm = VersionManager::new()?;
    let cm = ConfigManager::new()?;

    let binary = vm.get_binary_path(None).await?;
    let config = cm.get_current_path().await?;

    let sm = ServiceManager::new(binary.clone(), config.clone());

    println!("Service configuration:");
    println!("  Binary: {}", binary.display());
    println!("  Config: {}", config.display());
    println!();

    // Check if running
    println!("Checking service status...");
    if sm.is_running().await {
        println!("✓ Service is running\n");

        // Get detailed status
        match sm.status().await {
            Ok(ServiceStatus::Running(pid)) => {
                println!("Details:");
                println!("  PID: {}", pid);
                println!("  Status: Active");
            }
            Ok(ServiceStatus::Stopped) => {
                println!("Details:");
                println!("  Status: Stopped");
            }
            Err(e) => {
                eprintln!("Error getting status: {}", e);
            }
        }
    } else {
        println!("✗ Service is not running");
        println!("\nStart it with:");
        println!("  cargo run --example service_lifecycle");
    }

    Ok(())
}
