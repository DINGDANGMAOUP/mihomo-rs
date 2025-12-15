//! Example: Manage Versions
//!
//! Complete version lifecycle: install, switch, uninstall.
//!
//! ## Running
//! ```bash
//! cargo run --example manage_versions
//! ```

use mihomo_rs::{Result, VersionManager};

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    println!("=== Manage Mihomo Versions ===\n");

    let vm = VersionManager::new()?;

    // List current versions
    println!("Current installations:");
    let versions = vm.list_installed().await?;
    for v in &versions {
        let marker = if v.is_default { "→" } else { " " };
        println!("  {} {}", marker, v.version);
    }
    println!();

    // Get default version
    if let Ok(default) = vm.get_default().await {
        println!("Default version: {}\n", default);
    }

    // Switch default (example)
    if versions.len() > 1 {
        let new_default = &versions[0].version;
        println!("Switching default to: {}", new_default);
        vm.set_default(new_default).await?;
        println!("✓ Default changed\n");
    }

    println!("=== Version Management Commands ===");
    println!("  - Install: vm.install(\"v1.18.9\").await?");
    println!("  - Set default: vm.set_default(\"v1.18.9\").await?");
    println!("  - Uninstall: vm.uninstall(\"v1.18.9\").await?");
    println!("  - List: vm.list_installed().await?");

    Ok(())
}
