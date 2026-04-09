use mihomo_rs::{ConfigManager, ConnectionManager, MihomoClient, Result};

#[tokio::main]
async fn main() -> Result<()> {
    let cm = ConfigManager::new()?;
    let controller = cm.get_external_controller().await?;
    let client = MihomoClient::new(&controller, None)?;
    let conn = ConnectionManager::new(client);

    let list = conn.list().await?;
    let (down, up, count) = conn.get_statistics().await?;

    println!("connections: {} (list={})", count, list.len());
    println!("traffic total down={} up={}", down, up);

    let example_host = conn.filter_by_host("example").await?;
    println!("matched host 'example': {}", example_host.len());

    Ok(())
}
