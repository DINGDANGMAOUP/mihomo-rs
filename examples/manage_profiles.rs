//! Example: Manage Profiles
//!
//! Create, list, switch, and delete configuration profiles.
//!
//! ## Running
//! ```bash
//! cargo run --example manage_profiles
//! ```

use mihomo_rs::{ConfigManager, Result};

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    println!("=== Manage Configuration Profiles ===\n");

    let cm = ConfigManager::new()?;

    // List existing profiles
    println!("Current profiles:");
    let profiles = cm.list_profiles().await?;
    for p in &profiles {
        let marker = if p.active { "→" } else { " " };
        println!("  {} {}", marker, p.name);
    }
    println!();

    // Create a new profile
    let profile_name = "test-profile";
    println!("Creating profile: {}", profile_name);

    let sample_config = r#"
port: 7890
socks-port: 7891
allow-lan: false
mode: rule
log-level: info
external-controller: 127.0.0.1:9090
"#;

    cm.save(profile_name, sample_config).await?;
    println!("✓ Profile created\n");

    // List profiles again
    let profiles = cm.list_profiles().await?;
    println!("Updated profiles:");
    for p in &profiles {
        println!("  {}", p.name);
    }

    // Clean up: delete test profile
    println!("\nCleaning up test profile...");
    cm.delete_profile(profile_name).await?;
    println!("✓ Test profile deleted");

    Ok(())
}
