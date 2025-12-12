use mihomo_rs::{ConfigManager, Result, ServiceManager, ServiceStatus, VersionManager};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<()> {
    // Get the binary and config paths
    let version_manager = VersionManager::new()?;
    let config_manager = ConfigManager::new()?;

    let binary_path = version_manager.get_binary_path(None).await?;
    let config_path = config_manager.get_current_path().await?;

    println!("Binary: {}", binary_path.display());
    println!("Config: {}", config_path.display());

    // Create service manager
    let service_manager = ServiceManager::new(binary_path, config_path);

    // Check current status
    println!("\nChecking service status...");
    match service_manager.status().await? {
        ServiceStatus::Running(pid) => {
            println!("✓ Service is running (PID: {})", pid);
        }
        ServiceStatus::Stopped => {
            println!("✗ Service is stopped");
            println!("\nStarting service...");
            service_manager.start().await?;
            println!("✓ Service started successfully");

            // Wait a bit
            tokio::time::sleep(Duration::from_secs(2)).await;

            // Check status again
            match service_manager.status().await? {
                ServiceStatus::Running(pid) => {
                    println!("✓ Service is now running (PID: {})", pid);
                }
                ServiceStatus::Stopped => {
                    println!("✗ Service failed to start");
                }
            }
        }
    }

    Ok(())
}
