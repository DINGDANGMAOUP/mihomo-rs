//! Example: Install by Channel
//!
//! Demonstrates installing the latest version from a channel (Stable/Beta/Nightly).
//!
//! ## Running
//! ```bash
//! CHANNEL=stable cargo run --example install_by_channel
//! CHANNEL=beta cargo run --example install_by_channel
//! CHANNEL=nightly cargo run --example install_by_channel
//! ```

use mihomo_rs::{Channel, Result, VersionManager};
use std::env;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    println!("=== Install Mihomo by Channel ===\n");

    let vm = VersionManager::new()?;

    // Parse channel from environment
    let channel_str = env::var("CHANNEL").unwrap_or_else(|_| "stable".to_string());
    let channel = match channel_str.to_lowercase().as_str() {
        "stable" => Channel::Stable,
        "beta" => Channel::Beta,
        "nightly" => Channel::Nightly,
        _ => {
            eprintln!("Invalid channel: {}", channel_str);
            eprintln!("Use: stable, beta, or nightly");
            return Ok(());
        }
    };

    println!("Channel: {:?}", channel);
    println!("Fetching latest version...\n");

    match vm.install_channel(channel).await {
        Ok(version) => {
            println!("✓ Installed mihomo {}", version);
            println!("✓ Set as default version");

            let binary = vm.get_binary_path(None).await?;
            println!("✓ Binary: {}", binary.display());
        }
        Err(e) => {
            eprintln!("✗ Installation failed: {}", e);
            return Err(e);
        }
    }

    Ok(())
}
