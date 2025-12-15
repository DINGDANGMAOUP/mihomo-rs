//! Example: Switch Proxy
//!
//! This example demonstrates how to switch the active proxy in a group.
//! It shows:
//! - Listing available groups
//! - Showing current selection
//! - Switching to a different proxy
//! - Verifying the switch
//!
//! ## Prerequisites
//! - mihomo service must be running
//! - Configuration with Selector-type proxy groups
//!
//! ## Running
//! ```bash
//! cargo run --example switch_proxy
//! ```
//!
//! You can also specify the group and proxy directly:
//! ```bash
//! PROXY_GROUP="GLOBAL" PROXY_NAME="proxy1" cargo run --example switch_proxy
//! ```

use mihomo_rs::{ConfigManager, MihomoClient, ProxyManager, Result};
use std::env;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    println!("=== Switch Proxy ===\n");

    // Connect to mihomo
    let cm = ConfigManager::new()?;
    let url = cm.get_external_controller().await?;
    let client = MihomoClient::new(&url, None)?;

    println!("Connected to: {}\n", url);

    let pm = ProxyManager::new(client);

    // Check if group and proxy are specified via environment variables
    let group_name = env::var("PROXY_GROUP").ok();
    let proxy_name = env::var("PROXY_NAME").ok();

    if let (Some(group), Some(proxy)) = (group_name, proxy_name) {
        // Direct switch mode
        println!("Switching group '{}' to proxy '{}'...", group, proxy);
        match pm.switch(&group, &proxy).await {
            Ok(_) => {
                println!("✓ Successfully switched!");

                // Verify the switch
                let current = pm.get_current(&group).await?;
                println!("✓ Verified: group '{}' is now using '{}'", group, current);
            }
            Err(e) => {
                eprintln!("✗ Failed to switch proxy: {}", e);
                eprintln!("\nHint: Make sure the group and proxy names are correct.");
                return Err(e);
            }
        }
    } else {
        // Interactive mode - show available options
        println!("Fetching proxy groups...");
        let groups = pm.list_groups().await?;

        if groups.is_empty() {
            println!("\nNo proxy groups found.");
            println!("Add Selector-type groups to your configuration to switch proxies.");
            return Ok(());
        }

        // Filter to only Selector groups (which can be manually switched)
        let selector_groups: Vec<_> = groups
            .iter()
            .filter(|g| {
                g.group_type.to_lowercase() == "selector" || g.group_type.to_lowercase() == "select"
            })
            .collect();

        if selector_groups.is_empty() {
            println!("\nNo Selector-type groups found.");
            println!("Only Selector groups can be manually switched.");
            println!("\nFound groups:");
            for group in groups {
                println!("  - {} ({})", group.name, group.group_type);
            }
            return Ok(());
        }

        println!("\nAvailable groups you can switch:\n");
        for (i, group) in selector_groups.iter().enumerate() {
            println!("{}. {}", i + 1, group.name);
            println!("   Currently using: {}", group.now);
            println!("   Available options: {}", group.all.len());
            println!();
        }

        // Example demonstration with first group
        println!("=== Example: Switching first group ===");
        let first_group = selector_groups[0];
        println!("\nGroup: {}", first_group.name);
        println!("Current: {}", first_group.now);

        if first_group.all.len() < 2 {
            println!("This group only has one proxy option, cannot demonstrate switching.");
        } else {
            // Find a different proxy to switch to
            let new_proxy = first_group
                .all
                .iter()
                .find(|name| *name != &first_group.now)
                .unwrap();

            println!("\nAvailable proxies:");
            for (i, proxy) in first_group.all.iter().enumerate() {
                let marker = if proxy == &first_group.now {
                    "→ (current)"
                } else if proxy == new_proxy {
                    "  (will switch to this)"
                } else {
                    " "
                };
                println!("  {} [{}] {}", marker, i + 1, proxy);
            }

            println!("\nSwitching '{}' to '{}'...", first_group.name, new_proxy);

            match pm.switch(&first_group.name, new_proxy).await {
                Ok(_) => {
                    println!("✓ Successfully switched!");

                    // Verify the switch
                    let current = pm.get_current(&first_group.name).await?;
                    println!("✓ Verified: now using '{}'", current);

                    // Switch back to original
                    println!(
                        "\nSwitching back to original proxy '{}'...",
                        first_group.now
                    );
                    pm.switch(&first_group.name, &first_group.now).await?;
                    println!("✓ Switched back successfully");
                }
                Err(e) => {
                    eprintln!("✗ Failed to switch: {}", e);
                    return Err(e);
                }
            }
        }

        println!("\n=== How to switch directly ===");
        println!("Set environment variables:");
        println!(
            "  PROXY_GROUP=\"{}\" PROXY_NAME=\"your_proxy\" cargo run --example switch_proxy",
            first_group.name
        );
        println!("\nOr use the high-level function:");
        println!("  mihomo_rs::switch_proxy(\"group\", \"proxy\")");
    }

    println!("\n=== Next Steps ===");
    println!("  - View current selections: cargo run --example current_proxy");
    println!("  - Test proxy delays: cargo run --example test_delay");
    println!("  - List all groups: cargo run --example list_groups");

    Ok(())
}
