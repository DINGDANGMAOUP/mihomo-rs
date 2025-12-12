use mihomo_rs::{ConfigManager, Result};

#[tokio::main]
async fn main() -> Result<()> {
    let cm = ConfigManager::new()?;

    // List all profiles
    println!("=== Configuration Profiles ===");
    let profiles = cm.list_profiles().await?;
    if profiles.is_empty() {
        println!("No profiles found");
    } else {
        for profile in &profiles {
            let marker = if profile.active { "*" } else { " " };
            println!("{} {} ({})", marker, profile.name, profile.path.display());
        }
    }

    // Get current profile
    println!("\n=== Current Profile ===");
    let current = cm.get_current().await?;
    println!("Active profile: {}", current);

    // Get current profile path
    let current_path = cm.get_current_path().await?;
    println!("Config path: {}", current_path.display());

    // Get external controller URL
    println!("\n=== External Controller ===");
    let url = cm.get_external_controller().await?;
    println!("Controller URL: {}", url);

    // Show current config content
    println!("\n=== Current Config Content (first 20 lines) ===");
    let content = cm.load(&current).await?;
    for (i, line) in content.lines().take(20).enumerate() {
        println!("{:3}: {}", i + 1, line);
    }
    if content.lines().count() > 20 {
        println!("... ({} more lines)", content.lines().count() - 20);
    }

    // Example: Switch to another profile (commented out)
    // if profiles.len() > 1 {
    //     let other_profile = &profiles[1].name;
    //     println!("\n=== Switching Profile ===");
    //     cm.set_current(other_profile).await?;
    //     println!("✓ Switched to profile: {}", other_profile);
    // }

    // Example: Save a new profile (commented out)
    // let new_config = r#"
    // port: 7890
    // socks-port: 7891
    // allow-lan: false
    // mode: rule
    // log-level: info
    // external-controller: :9090
    // "#;
    // cm.save("example", new_config).await?;
    // println!("✓ Saved new profile: example");

    Ok(())
}
