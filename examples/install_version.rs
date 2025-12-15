//! Example: Install Specific Version
//!
//! Demonstrates installing a specific mihomo version from GitHub releases.
//!
//! ## Running
//! ```bash
//! # Install specific version
//! VERSION="v1.18.9" cargo run --example install_version
//!
//! # Or use default (latest stable)
//! cargo run --example install_version
//! ```

use mihomo_rs::{Result, VersionManager};
use std::env;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    println!("=== Install Mihomo Version ===\n");

    let vm = VersionManager::new()?;

    // Get version from environment or use default
    let version = env::var("VERSION").unwrap_or_else(|_| "v1.18.9".to_string());

    println!("Installing mihomo version: {}", version);
    println!("This may take a few minutes...\n");

    match vm.install(&version).await {
        Ok(_) => {
            println!("✓ Successfully installed mihomo {}", version);

            // Get binary path
            let binary_path = vm.get_binary_path(Some(&version)).await?;
            println!("✓ Binary location: {}", binary_path.display());

            // List all installed versions
            println!("\nInstalled versions:");
            let versions = vm.list_installed().await?;
            for v in versions {
                let marker = if v.is_default { "→" } else { " " };
                println!("  {} {}", marker, v.version);
            }
        }
        Err(e) => {
            eprintln!("✗ Failed to install: {}", e);
            return Err(e);
        }
    }

    println!("\nNext: Set as default with");
    println!("  cargo run --example manage_versions");

    Ok(())
}
