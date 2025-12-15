//! Example: List Installed Versions
//!
//! Displays all installed mihomo versions.
//!
//! ## Running
//! ```bash
//! cargo run --example list_versions
//! ```

use mihomo_rs::{Result, VersionManager};

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    println!("=== Installed Mihomo Versions ===\n");

    let vm = VersionManager::new()?;
    let versions = vm.list_installed().await?;

    if versions.is_empty() {
        println!("No versions installed.");
        println!("\nInstall one with:");
        println!("  cargo run --example install_version");
        return Ok(());
    }

    println!("Found {} installed version(s):\n", versions.len());

    for (i, v) in versions.iter().enumerate() {
        let marker = if v.is_default { "→ (default)" } else { " " };
        println!("{}. {} {}", i + 1, v.version, marker);
        println!("   Path: {}", v.path.display());
    }

    println!("\n=== Tips ===");
    println!("  - Install new version: cargo run --example install_version");
    println!("  - Switch default: cargo run --example manage_versions");

    Ok(())
}
