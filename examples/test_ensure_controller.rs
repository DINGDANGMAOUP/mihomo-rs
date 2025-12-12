use mihomo_rs::ConfigManager;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> mihomo_rs::Result<()> {
    env_logger::init();

    let home = PathBuf::from("/tmp/mihomo-test");
    let cm = ConfigManager::with_home(home)?;

    println!("Testing ensure_external_controller...\n");

    // First call - should add external-controller
    println!("1. Calling ensure_external_controller (config has no external-controller)...");
    let url = cm.ensure_external_controller().await?;
    println!("   ✓ External controller configured: {}\n", url);

    // Read the config to verify
    let content = cm.load("default").await?;
    println!("2. Updated config content:");
    println!("   {}\n", content.replace('\n', "\n   "));

    // Second call - should use existing config
    println!("3. Calling ensure_external_controller again (config now has external-controller)...");
    let url2 = cm.ensure_external_controller().await?;
    println!("   ✓ Second call returned: {}\n", url2);

    println!("✓ All tests passed!");

    Ok(())
}
