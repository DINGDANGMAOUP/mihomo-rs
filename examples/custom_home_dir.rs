//! Example: Custom Home Directory
//!
//! Use custom home directory for mihomo data (useful for multi-user setups).
//!
//! ## Running
//! ```bash
//! CUSTOM_HOME="/tmp/mihomo-test" cargo run --example custom_home_dir
//! ```

use mihomo_rs::{ConfigManager, Result, VersionManager};
use std::env;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    println!("=== Custom Home Directory ===\n");

    // Get custom home from environment
    let custom_home = env::var("CUSTOM_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/tmp/mihomo-custom"));

    println!("Using custom home: {}\n", custom_home.display());

    // Create managers with custom home
    let vm = VersionManager::with_home(custom_home.clone())?;
    let cm = ConfigManager::with_home(custom_home.clone())?;

    println!("Version manager home: {}/versions", custom_home.display());
    println!("Config manager home: {}/configs\n", custom_home.display());

    // List versions (will be empty for new custom home)
    let versions = vm.list_installed().await?;
    println!("Installed versions: {}", versions.len());

    // List profiles
    let profiles = cm.list_profiles().await?;
    println!("Profiles: {}", profiles.len());

    println!("\n=== Use Cases ===");
    println!("Custom home directories are useful for:");
    println!("  - Multi-user systems");
    println!("  - Testing without affecting main installation");
    println!("  - Isolated environments");
    println!("  - CI/CD pipelines");

    Ok(())
}
