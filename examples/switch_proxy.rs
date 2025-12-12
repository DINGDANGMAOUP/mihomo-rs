use mihomo_rs::{ConfigManager, MihomoClient, ProxyManager, Result};

#[tokio::main]
async fn main() -> Result<()> {
    // Get the external controller URL from config
    let config_manager = ConfigManager::new()?;
    let url = config_manager.get_external_controller().await?;

    // Create a client and proxy manager
    let client = MihomoClient::new(&url, None)?;
    let proxy_manager = ProxyManager::new(client);

    // Get all groups
    let groups = proxy_manager.list_groups().await?;

    if groups.is_empty() {
        println!("No proxy groups found");
        return Ok(());
    }

    // Show current status
    println!("Current proxy groups:");
    for group in &groups {
        println!("  {} -> {}", group.name, group.now);
    }

    // Example: Switch the first group to its first available proxy
    if let Some(group) = groups.first() {
        if let Some(proxy) = group.all.first() {
            println!("\nSwitching '{}' to '{}'...", group.name, proxy);
            proxy_manager.switch(&group.name, proxy).await?;
            println!("✓ Successfully switched!");

            // Verify the switch
            let current = proxy_manager.get_current(&group.name).await?;
            println!("Current proxy for '{}': {}", group.name, current);
        }
    }

    Ok(())
}
