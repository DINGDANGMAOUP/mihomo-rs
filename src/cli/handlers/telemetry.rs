use crate::cli::print_info;
use crate::config::ConfigManager;
use crate::core::MihomoClient;

pub async fn handle_logs(level: Option<String>) -> anyhow::Result<()> {
    let cm = ConfigManager::new()?;
    let url = cm.get_external_controller().await?;
    let client = MihomoClient::new(&url, None)?;
    print_info("Streaming logs... (Press Ctrl+C to stop)");

    let mut rx = client.stream_logs(level.as_deref()).await?;
    while let Some(log) = rx.recv().await {
        println!("{}", log);
    }

    Ok(())
}

pub async fn handle_traffic() -> anyhow::Result<()> {
    let cm = ConfigManager::new()?;
    let url = cm.get_external_controller().await?;
    let client = MihomoClient::new(&url, None)?;
    print_info("Streaming traffic... (Press Ctrl+C to stop)");

    let mut rx = client.stream_traffic().await?;
    while let Some(traffic) = rx.recv().await {
        println!(
            "↑ {} KB/s  ↓ {} KB/s",
            traffic.up / 1024,
            traffic.down / 1024
        );
    }

    Ok(())
}

pub async fn handle_memory() -> anyhow::Result<()> {
    let cm = ConfigManager::new()?;
    let url = cm.get_external_controller().await?;
    let client = MihomoClient::new(&url, None)?;

    let memory = client.get_memory().await?;
    println!("Memory Usage:");
    println!("  In Use:   {} MB", memory.in_use / 1024 / 1024);
    println!("  OS Limit: {} MB", memory.os_limit / 1024 / 1024);

    Ok(())
}
