//! Example: Basic Workflow
//!
//! This example demonstrates a complete beginner workflow with mihomo-rs:
//! 1. Install mihomo (if not already installed)
//! 2. Create default configuration
//! 3. Start the mihomo service
//! 4. List available proxies
//! 5. Stop the service
//!
//! This is perfect for first-time users to get everything set up and working.
//!
//! ## Prerequisites
//! - Internet connection (for downloading mihomo if needed)
//! - Sufficient permissions to run mihomo service
//!
//! ## Running
//! ```bash
//! cargo run --example basic_workflow
//! ```

use mihomo_rs::{
    Channel, ConfigManager, MihomoClient, ProxyManager, Result, ServiceManager, VersionManager,
};
use std::time::Duration;
use tokio::time::sleep;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    env_logger::init();

    println!("=== Mihomo-rs Basic Workflow ===\n");

    // Step 1: Version Management - Install mihomo
    println!("Step 1: Checking mihomo installation...");
    let vm = VersionManager::new()?;

    // Check if we have any versions installed
    let installed = vm.list_installed().await?;
    if installed.is_empty() {
        println!("  No mihomo version found. Installing latest stable version...");
        match vm.install_channel(Channel::Stable).await {
            Ok(version) => {
                println!("  Successfully installed mihomo version: {}", version);
                println!("  Set as default version");
            }
            Err(e) => {
                eprintln!("Error: Failed to install mihomo");
                eprintln!("  {}", e);
                eprintln!("\nHint: Check your internet connection and try again.");
                return Err(e);
            }
        }
    } else {
        let default_version = vm.get_default().await?;
        println!("  Mihomo is already installed");
        println!("  Default version: {}", default_version);
        println!("  Total installed versions: {}", installed.len());
    }
    println!();

    // Step 2: Configuration Management - Ensure config exists
    println!("Step 2: Setting up configuration...");
    let cm = ConfigManager::new()?;

    // Ensure default config exists
    cm.ensure_default_config().await?;
    println!("  Default configuration ensured");

    // Ensure external controller is configured
    let controller_url = cm.ensure_external_controller().await?;
    println!("  External controller: {}", controller_url);

    let config_path = cm.get_current_path().await?;
    println!("  Config file: {}", config_path.display());
    println!();

    // Step 3: Service Management - Start mihomo
    println!("Step 3: Starting mihomo service...");
    let binary_path = vm.get_binary_path(None).await?;
    let sm = ServiceManager::new(binary_path, config_path);

    // Check if already running
    if sm.is_running().await {
        println!("  Mihomo is already running");
        println!("  Restarting to ensure fresh state...");
        sm.restart().await?;
        println!("  Service restarted successfully");
    } else {
        match sm.start().await {
            Ok(_) => println!("  Service started successfully"),
            Err(e) => {
                eprintln!("Error: Failed to start mihomo service");
                eprintln!("  {}", e);
                eprintln!("\nHint: Check if the configuration is valid.");
                return Err(e);
            }
        }
    }

    // Wait a moment for service to fully initialize
    println!("  Waiting for service to initialize...");
    sleep(Duration::from_secs(2)).await;
    println!();

    // Step 4: Connect and List Proxies
    println!("Step 4: Connecting to mihomo...");
    let client = MihomoClient::new(&controller_url, None)?;

    // Verify connection by getting version
    let version = client.get_version().await?;
    println!("  Connected to mihomo {}", version.version);
    println!();

    // Use ProxyManager for high-level operations
    println!("Step 5: Listing proxy groups...");
    let pm = ProxyManager::new(client);

    let groups = pm.list_groups().await?;
    if groups.is_empty() {
        println!("  No proxy groups found in configuration");
        println!("\n  Hint: Your configuration might not have proxy groups defined yet.");
        println!(
            "  You can add proxies to your config file at: {}",
            cm.get_current_path().await?.display()
        );
    } else {
        println!("  Found {} proxy group(s):\n", groups.len());
        for (i, group) in groups.iter().enumerate().take(5) {
            println!("  Group #{}: {}", i + 1, group.name);
            println!("    Type: {}", group.group_type);
            println!("    Currently using: {}", group.now);
            println!("    Available proxies: {}", group.all.len());
            println!();
        }

        if groups.len() > 5 {
            println!("  ... and {} more groups", groups.len() - 5);
        }
    }

    // List individual proxy nodes
    println!("Step 6: Listing proxy nodes...");
    let proxies = pm.list_proxies().await?;
    if proxies.is_empty() {
        println!("  No proxy nodes found");
    } else {
        println!("  Found {} proxy node(s):\n", proxies.len());
        for (i, proxy) in proxies.iter().enumerate().take(5) {
            println!("  Proxy #{}: {}", i + 1, proxy.name);
            println!("    Type: {}", proxy.proxy_type);
            println!("    Status: {}", if proxy.alive { "Alive" } else { "Down" });
            if let Some(delay) = proxy.delay {
                println!("    Delay: {} ms", delay);
            }
            println!();
        }

        if proxies.len() > 5 {
            println!("  ... and {} more proxies", proxies.len() - 5);
        }
    }

    // Step 7: Cleanup - Stop the service
    println!("\nStep 7: Cleaning up...");
    println!("  Do you want to stop the mihomo service? (it will keep running)");
    println!("  To stop it manually later, run:");
    println!("    cargo run --example service_lifecycle");
    println!();

    // We'll keep it running for now since the user might want to explore more examples
    println!("  Keeping service running for further exploration");

    println!("\n=== Workflow Complete! ===");
    println!("\nYou now have mihomo installed and running.");
    println!("\nNext steps:");
    println!("  - Switch proxies: cargo run --example switch_proxy");
    println!("  - Monitor traffic: cargo run --example stream_traffic");
    println!("  - View logs: cargo run --example stream_logs");
    println!("  - Manage versions: cargo run --example manage_versions");
    println!("\nTo stop the service:");
    println!("  cargo run --example service_lifecycle");

    Ok(())
}
