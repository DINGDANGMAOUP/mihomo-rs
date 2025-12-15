//! Example: Custom Configuration
//!
//! Create and customize mihomo YAML configuration.
//!
//! ## Running
//! ```bash
//! cargo run --example custom_config
//! ```

use mihomo_rs::{ConfigManager, Result};

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    println!("=== Custom Mihomo Configuration ===\n");

    let cm = ConfigManager::new()?;

    // Ensure default config exists
    cm.ensure_default_config().await?;
    println!("✓ Default configuration ensured");

    // Load current config
    let current_profile = cm.get_current().await?;
    println!("Current profile: {}", current_profile);

    let config_content = cm.load(&current_profile).await?;
    println!("\nCurrent configuration preview:");
    println!(
        "{}",
        config_content
            .lines()
            .take(10)
            .collect::<Vec<_>>()
            .join("\n")
    );
    if config_content.lines().count() > 10 {
        println!("... ({} more lines)", config_content.lines().count() - 10);
    }

    println!("\n=== Configuration Options ===");
    println!("port: HTTP proxy port (default: 7890)");
    println!("socks-port: SOCKS5 port (default: 7891)");
    println!("external-controller: API address (e.g., 127.0.0.1:9090)");
    println!("mode: rule|global|direct");
    println!("log-level: info|warning|error|debug|silent");

    Ok(())
}
