/// Complete workflow example demonstrating all major features
use mihomo_rs::{
    ConfigManager, MihomoClient, ProxyManager, Result, ServiceManager, ServiceStatus,
    VersionManager,
};

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== mihomo-rs Complete Workflow Example ===\n");

    // 1. Version Management
    println!("1. Version Management");
    println!("   -------------------");
    let vm = VersionManager::new()?;
    let versions = vm.list_installed().await?;
    println!("   Installed versions: {}", versions.len());
    for v in &versions {
        let marker = if v.is_default { "*" } else { " " };
        println!("   {} {}", marker, v.version);
    }

    // 2. Configuration Management
    println!("\n2. Configuration Management");
    println!("   ------------------------");
    let cm = ConfigManager::new()?;
    let profiles = cm.list_profiles().await?;
    println!("   Available profiles: {}", profiles.len());
    let current_profile = cm.get_current().await?;
    println!("   Current profile: {}", current_profile);
    let controller_url = cm.get_external_controller().await?;
    println!("   External controller: {}", controller_url);

    // 3. Service Management
    println!("\n3. Service Management");
    println!("   ------------------");
    if versions.is_empty() {
        println!("   ⚠ No mihomo version installed, skipping service management");
    } else {
        let binary = vm.get_binary_path(None).await?;
        let config = cm.get_current_path().await?;
        let sm = ServiceManager::new(binary, config);

        match sm.status().await? {
            ServiceStatus::Running(pid) => {
                println!("   ✓ Service is running (PID: {})", pid);
            }
            ServiceStatus::Stopped => {
                println!("   ✗ Service is stopped");
                println!("   Tip: Run 'mihomo-rs start' to start the service");
            }
        }
    }

    // 4. Proxy Management
    println!("\n4. Proxy Management");
    println!("   ----------------");
    match MihomoClient::new(&controller_url, None) {
        Ok(client) => {
            let pm = ProxyManager::new(client);

            // List proxy nodes
            match pm.list_proxies().await {
                Ok(proxies) => {
                    println!("   Proxy nodes: {}", proxies.len());
                    for proxy in proxies.iter().take(5) {
                        let delay = proxy
                            .delay
                            .map(|d| format!("{}ms", d))
                            .unwrap_or_else(|| "-".to_string());
                        println!("     - {} ({}): {}", proxy.name, proxy.proxy_type, delay);
                    }
                    if proxies.len() > 5 {
                        println!("     ... and {} more", proxies.len() - 5);
                    }
                }
                Err(e) => {
                    println!("   ✗ Failed to list proxies: {}", e);
                }
            }

            // List proxy groups
            match pm.list_groups().await {
                Ok(groups) => {
                    println!("\n   Proxy groups: {}", groups.len());
                    for group in groups.iter().take(5) {
                        println!(
                            "     - {} ({}): {} -> {}",
                            group.name,
                            group.group_type,
                            group.now,
                            group.all.len()
                        );
                    }
                    if groups.len() > 5 {
                        println!("     ... and {} more", groups.len() - 5);
                    }
                }
                Err(e) => {
                    println!("   ✗ Failed to list groups: {}", e);
                }
            }
        }
        Err(e) => {
            println!("   ✗ Failed to connect to mihomo: {}", e);
            println!("   Tip: Make sure mihomo service is running");
        }
    }

    // 5. Summary
    println!("\n=== Summary ===");
    println!(
        "✓ Version management: {} versions installed",
        versions.len()
    );
    println!("✓ Configuration: {} profiles available", profiles.len());
    println!("✓ Current profile: {}", current_profile);
    println!("✓ Controller URL: {}", controller_url);

    println!("\nFor more examples, see:");
    println!("  - cargo run --example version_management");
    println!("  - cargo run --example config_management");
    println!("  - cargo run --example service_management");
    println!("  - cargo run --example list_proxies");
    println!("  - cargo run --example switch_proxy");
    println!("  - cargo run --example test_delay");

    Ok(())
}
