//! Example: Hello Mihomo
//!
//! This example demonstrates the absolute basics of using the mihomo-rs SDK.
//! It shows how to:
//! - Connect to a running mihomo instance
//! - Get mihomo version information
//! - List available proxies
//!
//! ## Prerequisites
//! - mihomo service must be running
//! - External controller must be configured (default: http://127.0.0.1:9090)
//!
//! ## Running
//! ```bash
//! cargo run --example hello_mihomo
//! ```
//!
//! If you don't have mihomo installed yet, run the basic_workflow example first:
//! ```bash
//! cargo run --example basic_workflow
//! ```

use mihomo_rs::{ConfigManager, MihomoClient, Result};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging (optional, but useful for debugging)
    env_logger::init();

    println!("=== Hello Mihomo ===\n");

    // Step 1: Get the external controller URL from configuration
    // This will use the default mihomo-rs home directory (~/.config/mihomo-rs)
    println!("Step 1: Getting external controller URL...");
    let cm = ConfigManager::new()?;

    let url = match cm.get_external_controller().await {
        Ok(url) => {
            println!("  Found external controller at: {}\n", url);
            url
        }
        Err(e) => {
            eprintln!("Error: Could not get external controller URL");
            eprintln!("  {}", e);
            eprintln!("\nHint: Make sure mihomo is configured and running.");
            eprintln!("  You can run 'cargo run --example basic_workflow' to set everything up.");
            return Err(e);
        }
    };

    // Step 2: Create a client to connect to mihomo
    // The second parameter is an optional secret for authentication
    println!("Step 2: Creating mihomo client...");
    let client = MihomoClient::new(&url, None)?;
    println!("  Client created successfully\n");

    // Step 3: Get version information
    println!("Step 3: Getting mihomo version...");
    match client.get_version().await {
        Ok(version) => {
            println!("  Version: {}", version.version);
            println!("  Premium: {}", version.premium);
            println!("  Meta: {}\n", version.meta);
        }
        Err(e) => {
            eprintln!("Error: Could not connect to mihomo");
            eprintln!("  {}", e);
            eprintln!("\nHint: Make sure mihomo service is running.");
            eprintln!("  You can start it with 'cargo run --example service_lifecycle'");
            return Err(e);
        }
    }

    // Step 4: List all proxies
    println!("Step 4: Listing proxies...");
    match client.get_proxies().await {
        Ok(proxies) => {
            println!("  Found {} proxies/groups\n", proxies.len());

            // Show first few proxies as examples
            let mut count = 0;
            for (name, info) in proxies.iter().take(5) {
                count += 1;
                println!("  Proxy #{}: {}", count, name);
                println!("    Type: {}", info.proxy_type);

                // Show current selection if this is a group
                if let Some(current) = &info.now {
                    println!("    Currently using: {}", current);
                }

                // Show available proxies if this is a group
                if let Some(all) = &info.all {
                    println!("    Available: {} options", all.len());
                }

                println!();
            }

            if proxies.len() > 5 {
                println!("  ... and {} more", proxies.len() - 5);
                println!("\n  Run 'cargo run --example list_proxies' to see all proxies");
            }
        }
        Err(e) => {
            eprintln!("Error: Could not list proxies");
            eprintln!("  {}", e);
            return Err(e);
        }
    }

    println!("\n=== Success! ===");
    println!("You've successfully connected to mihomo and retrieved basic information.");
    println!("\nNext steps:");
    println!("  - See 'cargo run --example list_groups' to explore proxy groups");
    println!("  - See 'cargo run --example switch_proxy' to switch proxies");
    println!("  - See 'cargo run --example stream_traffic' for real-time monitoring");

    Ok(())
}
