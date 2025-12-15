use mihomo_rs::{ConfigManager, MihomoClient, Result};

#[tokio::main]
async fn main() -> Result<()> {
    let config_manager = ConfigManager::new()?;
    let url = config_manager.get_external_controller().await?;
    let client = MihomoClient::new(&url, None)?;

    println!("Streaming traffic statistics... (Press Ctrl+C to stop)");

    let mut rx = client.stream_traffic().await?;

    while let Some(traffic) = rx.recv().await {
        println!(
            "Upload: {} KB/s | Download: {} KB/s",
            traffic.up / 1024,
            traffic.down / 1024
        );
    }

    Ok(())
}
