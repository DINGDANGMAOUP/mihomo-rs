use mihomo_rs::ConfigManager;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> mihomo_rs::Result<()> {
    env_logger::init();

    // Use a fresh directory to simulate first-time setup
    let home = PathBuf::from("/tmp/mihomo-first-time");

    // Clean up if exists
    if home.exists() {
        std::fs::remove_dir_all(&home).ok();
    }

    println!("Testing first-time setup scenario...\n");
    println!(
        "1. Creating ConfigManager with empty directory: {}",
        home.display()
    );

    let cm = ConfigManager::with_home(home.clone())?;

    println!("2. Calling ensure_default_config() (no config exists yet)...");
    cm.ensure_default_config().await?;
    println!("   ✓ Default config created\n");

    // Verify the config was created
    let profile = cm.get_current().await?;
    println!("3. Current profile: {}", profile);

    let config_path = cm.get_current_path().await?;
    println!("   Config path: {}\n", config_path.display());

    // Read and display the created config
    let content = cm.load(&profile).await?;
    println!("4. Created config content:");
    println!("   {}\n", content.replace('\n', "\n   "));

    // Verify external-controller is present
    let url = cm.get_external_controller().await?;
    println!("5. External controller URL: {}\n", url);

    println!("✓ First-time setup works correctly!");
    println!("\nCleanup: removing test directory...");
    std::fs::remove_dir_all(&home).ok();

    Ok(())
}
