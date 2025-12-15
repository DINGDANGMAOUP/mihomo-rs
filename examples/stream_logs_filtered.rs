use mihomo_rs::{ConfigManager, MihomoClient, Result};

#[tokio::main]
async fn main() -> Result<()> {
    // Get external controller URL from config
    let config_manager = ConfigManager::new()?;
    let url = config_manager.get_external_controller().await?;

    // Create client
    let client = MihomoClient::new(&url, None)?;

    println!("Streaming error logs from mihomo... (Press Ctrl+C to stop)");

    // Get log receiver with error level filter
    let mut rx = client.stream_logs(Some("error")).await?;

    // Process only error logs
    while let Some(log) = rx.recv().await {
        eprintln!("ERROR: {}", log);
    }

    Ok(())
}
