//! Example: List Proxy Groups
//!
//! This example demonstrates how to list all proxy groups and their members.
//! It shows:
//! - Retrieving proxy groups (Selector, URLTest, Fallback, etc.)
//! - Displaying group type and current selection
//! - Showing available proxies in each group
//!
//! ## Prerequisites
//! - mihomo service must be running
//! - Configuration with proxy groups defined
//!
//! ## Running
//! ```bash
//! cargo run --example list_groups
//! ```

use mihomo_rs::{ConfigManager, MihomoClient, ProxyManager, Result};

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    println!("=== List All Proxy Groups ===\n");

    // Connect to mihomo
    let cm = ConfigManager::new()?;
    let url = cm.get_external_controller().await?;
    let client = MihomoClient::new(&url, None)?;

    println!("Connected to: {}\n", url);

    // Use ProxyManager to list groups
    let pm = ProxyManager::new(client);

    println!("Fetching proxy groups...");
    let groups = pm.list_groups().await?;

    if groups.is_empty() {
        println!("\nNo proxy groups found.");
        println!("Add proxy groups to your mihomo configuration to see them here.");
        println!("\nExample group configuration:");
        println!(
            r#"
proxy-groups:
  - name: "GLOBAL"
    type: select
    proxies:
      - proxy1
      - proxy2
  - name: "AutoSelect"
    type: url-test
    proxies:
      - proxy1
      - proxy2
    url: 'http://www.gstatic.com/generate_204'
    interval: 300
"#
        );
        return Ok(());
    }

    println!("Found {} proxy group(s):\n", groups.len());

    // Display each group with details
    for (i, group) in groups.iter().enumerate() {
        println!("{}. {} ({})", i + 1, group.name, group.group_type);
        println!("   Currently using: {}", group.now);
        println!("   Available proxies: {}", group.all.len());

        // Show all available proxies
        if group.all.len() <= 10 {
            // Show all if 10 or fewer
            for (j, proxy_name) in group.all.iter().enumerate() {
                let marker = if proxy_name == &group.now { "→" } else { " " };
                println!("     {} [{}] {}", marker, j + 1, proxy_name);
            }
        } else {
            // Show first few and indicate there are more
            for (j, proxy_name) in group.all.iter().take(5).enumerate() {
                let marker = if proxy_name == &group.now { "→" } else { " " };
                println!("     {} [{}] {}", marker, j + 1, proxy_name);
            }
            println!("     ... and {} more", group.all.len() - 5);
        }

        println!();
    }

    // Group type statistics
    let mut type_counts = std::collections::HashMap::new();
    for group in &groups {
        *type_counts.entry(group.group_type.as_str()).or_insert(0) += 1;
    }

    println!("=== Group Types ===");
    for (group_type, count) in type_counts.iter() {
        println!("  {}: {}", group_type, count);
    }

    // Explain group types
    println!("\n=== Group Type Descriptions ===");
    println!("  Selector: Manually select which proxy to use");
    println!("  URLTest: Automatically select fastest proxy");
    println!("  Fallback: Use first available proxy");
    println!("  LoadBalance: Distribute requests across proxies");
    println!("  Relay: Chain proxies together");

    println!("\n=== Next Steps ===");
    println!("  - Switch proxy in a group: cargo run --example switch_proxy");
    println!("  - Get current selections: cargo run --example current_proxy");
    println!("  - List individual proxies: cargo run --example list_proxies");

    Ok(())
}
