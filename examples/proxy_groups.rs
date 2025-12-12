use mihomo_rs::{ConfigManager, MihomoClient, ProxyManager, Result};

#[tokio::main]
async fn main() -> Result<()> {
    // Get the external controller URL from config
    let config_manager = ConfigManager::new()?;
    let url = config_manager.get_external_controller().await?;

    // Create client and proxy manager
    let client = MihomoClient::new(&url, None)?;
    let proxy_manager = ProxyManager::new(client);

    // List all proxy groups with details
    println!("=== Proxy Groups ===\n");
    let groups = proxy_manager.list_groups().await?;

    for group in groups {
        println!("Group: {}", group.name);
        println!("  Type: {}", group.group_type);
        println!("  Current: {}", group.now);
        println!("  Available proxies ({}):", group.all.len());
        for (i, proxy) in group.all.iter().enumerate() {
            let marker = if proxy == &group.now { "→" } else { " " };
            println!("    {} {}", marker, proxy);
            if i >= 9 && group.all.len() > 10 {
                println!("    ... and {} more", group.all.len() - 10);
                break;
            }
        }
        println!();
    }

    // Show current proxy for each group
    println!("=== Current Proxy Selection ===\n");
    for group in proxy_manager.list_groups().await? {
        let current = proxy_manager.get_current(&group.name).await?;
        println!("{:<30} -> {}", group.name, current);
    }

    Ok(())
}
