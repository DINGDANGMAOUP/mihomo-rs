//! Example: List Proxies
//!
//! This example demonstrates how to list all available proxy nodes.
//! It shows:
//! - Connecting to mihomo
//! - Retrieving all proxy nodes (excluding groups)
//! - Displaying proxy information (type, status, delay)
//!
//! ## Prerequisites
//! - mihomo service must be running
//! - Configuration with proxy nodes defined
//!
//! ## Running
//! ```bash
//! cargo run --example list_proxies
//! ```

use mihomo_rs::{ConfigManager, MihomoClient, ProxyManager, Result};

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    println!("=== List All Proxy Nodes ===\n");

    // Connect to mihomo
    let cm = ConfigManager::new()?;
    let url = cm.get_external_controller().await?;
    let client = MihomoClient::new(&url, None)?;

    println!("Connected to: {}\n", url);

    // Use ProxyManager for high-level operations
    let pm = ProxyManager::new(client);

    // List all proxy nodes (filters out groups)
    println!("Fetching proxy nodes...");
    let proxies = pm.list_proxies().await?;

    if proxies.is_empty() {
        println!("\nNo proxy nodes found.");
        println!("Your configuration might only have proxy groups without individual proxies.");
        return Ok(());
    }

    println!("Found {} proxy node(s):\n", proxies.len());

    // Display each proxy with details
    for (i, proxy) in proxies.iter().enumerate() {
        println!("{}. {}", i + 1, proxy.name);
        println!("   Type: {}", proxy.proxy_type);

        // Show status
        let status = if proxy.alive { "✓ Alive" } else { "✗ Down" };
        println!("   Status: {}", status);

        // Show delay if available
        if let Some(delay) = proxy.delay {
            println!("   Delay: {} ms", delay);
        } else {
            println!("   Delay: Not tested");
        }

        println!();
    }

    // Statistics
    let alive_count = proxies.iter().filter(|p| p.alive).count();
    let down_count = proxies.len() - alive_count;

    println!("=== Summary ===");
    println!("Total proxies: {}", proxies.len());
    println!("Alive: {}", alive_count);
    println!("Down: {}", down_count);

    // Show fastest proxy if we have delay data
    let fastest = proxies
        .iter()
        .filter(|p| p.alive && p.delay.is_some())
        .min_by_key(|p| p.delay.unwrap());

    if let Some(proxy) = fastest {
        println!(
            "\nFastest proxy: {} ({} ms)",
            proxy.name,
            proxy.delay.unwrap()
        );
    }

    println!("\n=== Next Steps ===");
    println!("  - List groups: cargo run --example list_groups");
    println!("  - Test delays: cargo run --example test_delay");
    println!("  - Switch proxy: cargo run --example switch_proxy");

    Ok(())
}
