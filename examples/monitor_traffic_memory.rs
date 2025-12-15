use mihomo_rs::{ConfigManager, MihomoClient, Result};
use tokio::time::{interval, Duration};

#[tokio::main]
async fn main() -> Result<()> {
    let config_manager = ConfigManager::new()?;
    let url = config_manager.get_external_controller().await?;
    let client = MihomoClient::new(&url, None)?;

    println!("Monitoring traffic and memory... (Press Ctrl+C to stop)\n");

    // Stream traffic in background
    let traffic_client = client.clone();
    let traffic_handle = tokio::spawn(async move {
        if let Ok(mut rx) = traffic_client.stream_traffic().await {
            while let Some(traffic) = rx.recv().await {
                print!(
                    "\r↑ {:>6} KB/s  ↓ {:>6} KB/s",
                    traffic.up / 1024,
                    traffic.down / 1024
                );
                use std::io::Write;
                std::io::stdout().flush().ok();
            }
        }
    });

    // Query memory periodically
    let memory_client = client.clone();
    let memory_handle = tokio::spawn(async move {
        let mut ticker = interval(Duration::from_secs(5));
        loop {
            ticker.tick().await;
            if let Ok(memory) = memory_client.get_memory().await {
                println!(
                    "\nMemory: {} MB / {} MB ({:.1}%)",
                    memory.in_use / 1024 / 1024,
                    memory.os_limit / 1024 / 1024,
                    (memory.in_use as f64 / memory.os_limit as f64) * 100.0
                );
            }
        }
    });

    tokio::select! {
        _ = traffic_handle => {},
        _ = memory_handle => {},
    }

    Ok(())
}
