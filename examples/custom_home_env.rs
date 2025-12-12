/// Example demonstrating custom home directory usage
use mihomo_rs::{ConfigManager, VersionManager, Result};

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== Custom Home Directory Example ===\n");

    // The home directory is determined by:
    // 1. MIHOMO_HOME environment variable (if set)
    // 2. Default: ~/.config/mihomo-rs

    // Check current home directory
    let home = mihomo_rs::core::get_home_dir()?;
    println!("Current home directory: {}", home.display());

    // Check if using custom home
    if let Ok(custom_home) = std::env::var("MIHOMO_HOME") {
        println!("✓ Using custom MIHOMO_HOME: {}", custom_home);
    } else {
        println!("Using default home directory");
        println!("\nTo use a custom directory, set MIHOMO_HOME:");
        println!("  export MIHOMO_HOME=/path/to/custom/dir");
        println!("  cargo run --example custom_home");
    }

    // List versions from the current home
    println!("\n=== Versions in current home ===");
    let vm = VersionManager::new()?;
    let versions = vm.list_installed().await?;
    if versions.is_empty() {
        println!("No versions installed in this home directory");
    } else {
        for v in versions {
            let marker = if v.is_default { "*" } else { " " };
            println!("{} {}", marker, v.version);
        }
    }

    // List configs from the current home
    println!("\n=== Configs in current home ===");
    let cm = ConfigManager::new()?;
    let profiles = cm.list_profiles().await?;
    if profiles.is_empty() {
        println!("No profiles found in this home directory");
    } else {
        for p in profiles {
            let marker = if p.active { "*" } else { " " };
            println!("{} {}", marker, p.name);
        }
    }

    println!("\n=== Usage Examples ===");
    println!("# Run with custom home:");
    println!("MIHOMO_HOME=/tmp/mihomo-test cargo run --example custom_home");
    println!("\n# Install to custom home:");
    println!("MIHOMO_HOME=/tmp/mihomo-test mihomo-rs install");
    println!("\n# Run service from custom home:");
    println!("MIHOMO_HOME=/tmp/mihomo-test mihomo-rs start");

    Ok(())
}
