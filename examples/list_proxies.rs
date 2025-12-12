use mihomo_rs::{ConfigManager, MihomoClient, ProxyManager, Result};

#[tokio::main]
async fn main() -> Result<()> {
    // Get the external controller URL from config
    let config_manager = ConfigManager::new()?;
    let url = config_manager.get_external_controller().await?;
    println!("Connecting to mihomo at: {}", url);

    // Create a client
    let client = MihomoClient::new(&url, None)?;
    let proxy_manager = ProxyManager::new(client);

    // List all proxy nodes
    println!("\n=== Proxy Nodes ===");
    let proxies = proxy_manager.list_proxies().await?;
    for proxy in proxies {
        let delay_str = proxy
            .delay
            .map(|d| format!("{}ms", d))
            .unwrap_or_else(|| "-".to_string());
        println!("{:<30} {:<15} {}", proxy.name, proxy.proxy_type, delay_str);
    }

    // List all proxy groups
    println!("\n=== Proxy Groups ===");
    let groups = proxy_manager.list_groups().await?;
    for group in groups {
        println!(
            "{:<30} {:<15} Current: {}",
            group.name, group.group_type, group.now
        );
    }

    Ok(())
}
