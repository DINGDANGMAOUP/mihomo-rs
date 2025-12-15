use mihomo_rs::{ConfigManager, MihomoClient, Result};

#[tokio::main]
async fn main() -> Result<()> {
    // Get external controller URL from config
    let config_manager = ConfigManager::new()?;
    let url = config_manager.get_external_controller().await?;

    // Create client
    let client = MihomoClient::new(&url, None)?;

    println!("Streaming logs from mihomo... (Press Ctrl+C to stop)");

    // Get log receiver
    let mut rx = client.stream_logs(None).await?;

    // Process logs as they arrive
    while let Some(log) = rx.recv().await {
        println!("{}", log);
    }

    Ok(())
}
