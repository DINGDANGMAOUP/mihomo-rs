use mihomo_rs::{ConfigManager, MihomoClient, Result};

#[tokio::main]
async fn main() -> Result<()> {
    // Get the external controller URL from config
    let config_manager = ConfigManager::new()?;
    let url = config_manager.get_external_controller().await?;

    // Create a client
    let client = MihomoClient::new(&url, None)?;

    // Get all proxies
    let proxies = client.get_proxies().await?;

    println!("Testing proxy delays...\n");

    // Test delay for each proxy (excluding groups)
    for (name, info) in proxies {
        // Skip group types
        let is_group = matches!(
            info.proxy_type.as_str(),
            "Selector"
                | "URLTest"
                | "Fallback"
                | "LoadBalance"
                | "Relay"
                | "Direct"
                | "Reject"
                | "Pass"
                | "Compatible"
                | "RejectDrop"
        );

        if !is_group {
            match client
                .test_delay(&name, "http://www.gstatic.com/generate_204", 5000)
                .await
            {
                Ok(delay) => {
                    println!("{:<30} {}ms", name, delay);
                }
                Err(e) => {
                    println!("{:<30} Error: {}", name, e);
                }
            }
        }
    }

    Ok(())
}
