//! Example: External Controller Setup
//!
//! Configure and verify external controller for API access.
//!
//! ## Running
//! ```bash
//! cargo run --example external_controller
//! ```

use mihomo_rs::{ConfigManager, Result};

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    println!("=== External Controller Setup ===\n");

    let cm = ConfigManager::new()?;

    // Ensure external controller is configured
    println!("Ensuring external controller is configured...");
    let url = cm.ensure_external_controller().await?;
    println!("✓ External controller: {}\n", url);

    // Get controller from config
    match cm.get_external_controller().await {
        Ok(controller_url) => {
            println!("Current configuration:");
            println!("  URL: {}", controller_url);

            // Parse URL to show details
            if let Ok(parsed) = url::Url::parse(&controller_url) {
                if let Some(host) = parsed.host_str() {
                    println!("  Host: {}", host);
                }
                if let Some(port) = parsed.port() {
                    println!("  Port: {}", port);
                }
            }
        }
        Err(e) => {
            eprintln!("Error: {}", e);
        }
    }

    println!("\n=== Tips ===");
    println!("The external-controller allows you to:");
    println!("  - Query proxy status via HTTP API");
    println!("  - Switch proxies programmatically");
    println!("  - Monitor traffic and logs");
    println!("  - Reload configuration without restart");

    Ok(())
}
