use mihomo_rs::{ConfigManager, MihomoClient, Result};

#[tokio::main]
async fn main() -> Result<()> {
    let config_manager = ConfigManager::new()?;
    let url = config_manager.get_external_controller().await?;
    let client = MihomoClient::new(&url, None)?;

    // Get log receiver
    let mut rx = client.stream_logs(None).await?;

    // Example: Process logs in background and do custom handling
    tokio::spawn(async move {
        while let Some(log) = rx.recv().await {
            // Third-party apps can:
            // 1. Parse and filter logs
            if log.contains("error") || log.contains("ERROR") {
                eprintln!("[ERROR] {}", log);
            }
            // 2. Send to their own logging system
            // my_logger.log(&log);

            // 3. Store in database
            // db.insert_log(&log).await;

            // 4. Send to monitoring service
            // metrics.record_log(&log);

            // 5. Trigger alerts
            if log.contains("fatal") {
                // send_alert(&log);
            }
        }
    });

    // Main application continues running
    println!("Log processing started in background...");
    tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;

    Ok(())
}
