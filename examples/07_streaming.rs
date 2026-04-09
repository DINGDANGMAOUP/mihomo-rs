use mihomo_rs::{ConfigManager, MihomoClient, Result};

#[tokio::main]
async fn main() -> Result<()> {
    let cm = ConfigManager::new()?;
    let controller = cm.get_external_controller().await?;
    let client = MihomoClient::new(&controller, None)?;

    let mut logs = client.stream_logs(Some("info")).await?;
    let mut traffic = client.stream_traffic().await?;

    for _ in 0..3 {
        tokio::select! {
            msg = logs.recv() => {
                if let Some(line) = msg {
                    println!("log: {line}");
                }
            }
            item = traffic.recv() => {
                if let Some(t) = item {
                    println!("traffic up={} down={}", t.up, t.down);
                }
            }
        }
    }

    Ok(())
}
