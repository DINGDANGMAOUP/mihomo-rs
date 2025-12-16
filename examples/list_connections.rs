//! Example: List Connections
//!
//! This example demonstrates how to list all active connections.
//! It shows:
//! - Connecting to mihomo
//! - Retrieving all active connections
//! - Displaying connection details (source, destination, proxy chain, traffic)
//! - Filtering connections by host, process, or rule
//! - Getting connection statistics
//!
//! ## Prerequisites
//! - mihomo service must be running
//! - Configuration with external-controller enabled
//!
//! ## Running
//! ```bash
//! cargo run --example list_connections
//! ```

use mihomo_rs::{ConfigManager, ConnectionManager, MihomoClient, Result};

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    println!("=== List Active Connections ===\n");

    // Connect to mihomo
    let cm = ConfigManager::new()?;
    let url = cm.get_external_controller().await?;
    let client = MihomoClient::new(&url, None)?;

    println!("Connected to: {}\n", url);

    // Use ConnectionManager for high-level operations
    let conn_mgr = ConnectionManager::new(client);

    // Get connection statistics
    println!("Fetching connection statistics...");
    let (download_total, upload_total, count) = conn_mgr.get_statistics().await?;

    println!("=== Statistics ===");
    println!("Total connections: {}", count);
    println!(
        "Total download: {:.2} MB",
        download_total as f64 / 1024.0 / 1024.0
    );
    println!(
        "Total upload: {:.2} MB\n",
        upload_total as f64 / 1024.0 / 1024.0
    );

    // List all connections
    println!("Fetching active connections...");
    let connections = conn_mgr.list().await?;

    if connections.is_empty() {
        println!("\nNo active connections found.");
        return Ok(());
    }

    println!("Found {} active connection(s):\n", connections.len());

    // Display each connection with details
    for (i, conn) in connections.iter().enumerate() {
        println!("{}. Connection ID: {}", i + 1, conn.id);

        // Source and destination
        if !conn.metadata.source_ip.is_empty() {
            println!(
                "   Source: {}:{}",
                conn.metadata.source_ip, conn.metadata.source_port
            );
        }

        if !conn.metadata.host.is_empty() {
            println!("   Host: {}", conn.metadata.host);
        } else if !conn.metadata.destination_ip.is_empty() {
            println!(
                "   Destination: {}:{}",
                conn.metadata.destination_ip, conn.metadata.destination_port
            );
        }

        // Network and protocol
        if !conn.metadata.network.is_empty() {
            println!(
                "   Network: {} ({})",
                conn.metadata.network, conn.metadata.connection_type
            );
        }

        // Proxy chains
        if !conn.chains.is_empty() {
            println!("   Proxy Chain: {}", conn.chains.join(" -> "));
        }

        // Rule matching
        if !conn.rule.is_empty() {
            println!("   Rule: {}", conn.rule);
        }

        // Process information
        if !conn.metadata.process_path.is_empty() {
            println!("   Process: {}", conn.metadata.process_path);
        }

        // Traffic statistics
        println!(
            "   Traffic: ↓ {:.2} KB / ↑ {:.2} KB",
            conn.download as f64 / 1024.0,
            conn.upload as f64 / 1024.0
        );

        println!();
    }

    // Example: Filter by host
    println!("=== Filter Examples ===");
    println!("Filtering connections by common hosts...\n");

    let common_hosts = vec!["github.com", "google.com", "cloudflare.com"];
    for host in common_hosts {
        let filtered = conn_mgr.filter_by_host(host).await?;
        if !filtered.is_empty() {
            println!("  Connections to '{}': {}", host, filtered.len());
        }
    }

    println!("\n=== Next Steps ===");
    println!("  - Stream connections: cargo run --example stream_connections");
    println!("  - Close connections: cargo run --example close_connections");
    println!("  - Monitor traffic: cargo run --example stream_traffic");

    Ok(())
}
