//! Example: Current Proxy
//!
//! This example demonstrates how to get the current proxy selection for all groups.
//! It shows:
//! - Listing all proxy groups
//! - Getting current selection for each group
//! - Displaying in a formatted table
//!
//! ## Prerequisites
//! - mihomo service must be running
//! - Proxy groups configured
//!
//! ## Running
//! ```bash
//! cargo run --example current_proxy
//! ```

use mihomo_rs::{ConfigManager, MihomoClient, ProxyManager, Result};

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    println!("=== Current Proxy Selections ===\n");

    // Connect to mihomo
    let cm = ConfigManager::new()?;
    let url = cm.get_external_controller().await?;
    let client = MihomoClient::new(&url, None)?;

    println!("Connected to: {}\n", url);

    let pm = ProxyManager::new(client);

    // Get all proxy groups
    println!("Fetching proxy groups...");
    let groups = pm.list_groups().await?;

    if groups.is_empty() {
        println!("\nNo proxy groups found.");
        return Ok(());
    }

    println!("Found {} group(s)\n", groups.len());

    // Calculate column widths for nice formatting
    let max_name_len = groups
        .iter()
        .map(|g| g.name.len())
        .max()
        .unwrap_or(10)
        .max(10);
    let max_type_len = groups
        .iter()
        .map(|g| g.group_type.len())
        .max()
        .unwrap_or(10)
        .max(10);
    let max_current_len = groups
        .iter()
        .map(|g| g.now.len())
        .max()
        .unwrap_or(15)
        .max(15);

    // Print header
    println!(
        "{:<width_name$}  {:<width_type$}  {:<width_current$}  Options",
        "Group Name",
        "Type",
        "Current Selection",
        width_name = max_name_len,
        width_type = max_type_len,
        width_current = max_current_len
    );
    println!(
        "{}",
        "-".repeat(max_name_len + max_type_len + max_current_len + 20)
    );

    // Print each group
    for group in &groups {
        println!(
            "{:<width_name$}  {:<width_type$}  {:<width_current$}  {}",
            group.name,
            group.group_type,
            group.now,
            group.all.len(),
            width_name = max_name_len,
            width_type = max_type_len,
            width_current = max_current_len
        );
    }

    println!();

    // Show detailed info for a few groups
    println!("=== Detailed View (First 3 Groups) ===\n");
    for group in groups.iter().take(3) {
        println!("Group: {}", group.name);
        println!("  Type: {}", group.group_type);
        println!("  Current: {}", group.now);
        println!("  Available options: {}", group.all.len());

        // Show proxies with marker for current
        if group.all.len() <= 10 {
            for (i, proxy) in group.all.iter().enumerate() {
                let marker = if proxy == &group.now { "→" } else { " " };
                println!("    {} [{}] {}", marker, i + 1, proxy);
            }
        } else {
            // Show first few
            for (i, proxy) in group.all.iter().take(5).enumerate() {
                let marker = if proxy == &group.now { "→" } else { " " };
                println!("    {} [{}] {}", marker, i + 1, proxy);
            }
            println!("    ... and {} more", group.all.len() - 5);
        }
        println!();
    }

    if groups.len() > 3 {
        println!("  ... and {} more groups", groups.len() - 3);
        println!();
    }

    // Group type summary
    let mut type_counts = std::collections::HashMap::new();
    for group in &groups {
        *type_counts.entry(group.group_type.as_str()).or_insert(0) += 1;
    }

    println!("=== Group Type Summary ===");
    for (group_type, count) in type_counts.iter() {
        println!("  {}: {}", group_type, count);
    }

    println!("\n=== Next Steps ===");
    println!("  - Switch proxy: cargo run --example switch_proxy");
    println!("  - List all groups: cargo run --example list_groups");
    println!("  - Test delays: cargo run --example test_delay");

    Ok(())
}
