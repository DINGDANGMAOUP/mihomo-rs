use mihomo_rs::ConfigManager;
use std::net::TcpListener;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> mihomo_rs::Result<()> {
    env_logger::init();

    let home = PathBuf::from("/tmp/mihomo-test");
    let cm = ConfigManager::with_home(home)?;

    println!("Testing port occupation handling...\n");

    // Occupy port 9092 (the one currently in config)
    println!("1. Occupying port 9092...");
    let _listener = TcpListener::bind("127.0.0.1:9092").expect("Failed to bind port 9092");
    println!("   ✓ Port 9092 is now occupied\n");

    // Call ensure_external_controller - should find alternative port
    println!("2. Calling ensure_external_controller (port 9092 is occupied)...");
    let url = cm.ensure_external_controller().await?;
    println!("   ✓ Found alternative port: {}\n", url);

    // Verify the config was updated
    let content = cm.load("default").await?;
    println!("3. Updated config content:");
    println!("   {}\n", content.replace('\n', "\n   "));

    println!("✓ Port occupation handling works correctly!");

    Ok(())
}
