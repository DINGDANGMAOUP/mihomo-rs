//! Example: Complete Workflow
//!
//! Full application example: setup → run → monitor → shutdown.
//!
//! ## Running
//! ```bash
//! cargo run --example complete_workflow
//! ```

use mihomo_rs::{
    Channel, ConfigManager, MihomoClient, ProxyManager, Result, ServiceManager, VersionManager,
};
use std::time::Duration;
use tokio::time::sleep;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    println!("=== Complete Mihomo Workflow ===\n");

    // 1. Version Management
    println!("Phase 1: Version Management");
    let vm = VersionManager::new()?;
    let versions = vm.list_installed().await?;
    if versions.is_empty() {
        println!("  Installing mihomo...");
        vm.install_channel(Channel::Stable).await?;
    }
    println!("  ✓ Mihomo installed\n");

    // 2. Configuration
    println!("Phase 2: Configuration");
    let cm = ConfigManager::new()?;
    cm.ensure_default_config().await?;
    let controller_url = cm.ensure_external_controller().await?;
    println!("  ✓ Configuration ready\n");

    // 3. Start Service
    println!("Phase 3: Start Service");
    let binary = vm.get_binary_path(None).await?;
    let config = cm.get_current_path().await?;
    let sm = ServiceManager::new(binary, config);

    if !sm.is_running().await {
        sm.start().await?;
        sleep(Duration::from_secs(2)).await;
    }
    println!("  ✓ Service running\n");

    // 4. Proxy Operations
    println!("Phase 4: Proxy Operations");
    let client = MihomoClient::new(&controller_url, None)?;
    let pm = ProxyManager::new(client.clone());

    let groups = pm.list_groups().await?;
    println!("  ✓ Found {} proxy groups\n", groups.len());

    // 5. Monitoring
    println!("Phase 5: Quick Monitoring");
    if let Ok(memory) = client.get_memory().await {
        println!("  Memory usage: {} MB", memory.in_use / 1024 / 1024);
    }
    println!("  ✓ Monitoring active\n");

    println!("=== Workflow Complete ===");
    println!("Mihomo is ready for use!");

    Ok(())
}
