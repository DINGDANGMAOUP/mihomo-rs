use mihomo_rs::{ConfigManager, MihomoClient, Result};

#[tokio::main]
async fn main() -> Result<()> {
    let config_manager = ConfigManager::new()?;
    let url = config_manager.get_external_controller().await?;
    let client = MihomoClient::new(&url, None)?;

    let memory = client.get_memory().await?;

    println!("Memory Usage:");
    println!("  In Use:   {} MB", memory.in_use / 1024 / 1024);
    println!("  OS Limit: {} MB", memory.os_limit / 1024 / 1024);
    println!(
        "  Usage:    {:.2}%",
        (memory.in_use as f64 / memory.os_limit as f64) * 100.0
    );

    Ok(())
}
