/// Complete workflow example using SDK with custom home
use mihomo_rs::{ConfigManager, MihomoClient, ProxyManager, Result, VersionManager};
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== SDK Complete Workflow Example ===\n");

    // Step 1: Define custom home
    let home = PathBuf::from("/tmp/mihomo-workflow");
    println!("Using custom home: {}\n", home.display());

    // Step 2: Initialize managers with custom home
    let vm = VersionManager::with_home(home.clone())?;
    let cm = ConfigManager::with_home(home.clone())?;

    println!("Step 1: Check installed versions");
    let versions = vm.list_installed().await?;
    println!("  Found {} versions", versions.len());

    println!("\nStep 2: Check configuration profiles");
    let profiles = cm.list_profiles().await?;
    println!("  Found {} profiles", profiles.len());

    if profiles.is_empty() {
        println!("  ⚠ No profiles found. Please add a config file:");
        println!("    mkdir -p {}/configs", home.display());
        println!(
            "    cp your-config.yaml {}/configs/default.yaml",
            home.display()
        );
        return Ok(());
    }

    println!("\nStep 3: Get external controller URL");
    let url = cm.get_external_controller().await?;
    println!("  Controller URL: {}", url);

    println!("\nStep 4: Connect to mihomo API");
    let client = MihomoClient::new(&url, None)?;
    let pm = ProxyManager::new(client);
    println!("  ✓ Connected");

    println!("\nStep 5: List proxy groups");
    let groups = pm.list_groups().await?;
    println!("  Found {} groups:", groups.len());
    for group in groups.iter().take(5) {
        println!("    - {} (current: {})", group.name, group.now);
    }

    println!("\nStep 6: List proxy nodes");
    let proxies = pm.list_proxies().await?;
    println!("  Found {} proxies:", proxies.len());
    for proxy in proxies.iter().take(5) {
        let delay = proxy
            .delay
            .map(|d| format!("{}ms", d))
            .unwrap_or_else(|| "-".to_string());
        println!("    - {}: {}", proxy.name, delay);
    }

    println!("\n✅ Workflow completed successfully!");
    println!("\nAll operations used custom home: {}", home.display());

    Ok(())
}
