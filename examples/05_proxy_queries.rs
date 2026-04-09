use mihomo_rs::{ConfigManager, MihomoClient, ProxyManager, Result};

#[tokio::main]
async fn main() -> Result<()> {
    let cm = ConfigManager::new()?;
    let controller = cm.get_external_controller().await?;
    let client = MihomoClient::new(&controller, None)?;
    let pm = ProxyManager::new(client);

    let groups = pm.list_groups().await?;
    println!("groups: {}", groups.len());

    let nodes = pm.list_proxies().await?;
    println!("nodes: {}", nodes.len());

    if let Some(group) = groups.first() {
        println!("group {} current={}", group.name, pm.get_current(&group.name).await?);
    }

    Ok(())
}
