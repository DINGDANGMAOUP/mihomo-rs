//! Example: Stream Connections
//!
//! This example demonstrates how to monitor connections in real-time.
//! It shows:
//! - Connecting to mihomo WebSocket endpoint
//! - Streaming connection updates
//! - Displaying real-time connection changes
//! - Showing traffic statistics updates
//!
//! ## Prerequisites
//! - mihomo service must be running
//! - Configuration with external-controller enabled
//!
//! ## Running
//! ```bash
//! cargo run --example stream_connections
//! ```

use mihomo_rs::{ConfigManager, ConnectionManager, MihomoClient, Result};
use tokio::signal;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    println!("=== Stream Active Connections ===\n");

    // Connect to mihomo
    let cm = ConfigManager::new()?;
    let url = cm.get_external_controller().await?;
    let client = MihomoClient::new(&url, None)?;

    println!("Connected to: {}\n", url);

    // Use ConnectionManager to stream connections
    let conn_mgr = ConnectionManager::new(client);

    println!("Starting connection stream...");
    println!("Press Ctrl+C to stop\n");

    let mut rx = conn_mgr.stream().await?;

    // Set up Ctrl+C handler
    let ctrl_c = signal::ctrl_c();
    tokio::pin!(ctrl_c);

    let mut update_count = 0;
    let mut last_connection_count = 0;

    loop {
        tokio::select! {
            // Receive connection updates
            Some(snapshot) = rx.recv() => {
                update_count += 1;

                // Clear screen for better visualization (optional)
                // print!("\x1B[2J\x1B[1;1H");

                println!("=== Update #{} ===", update_count);
                println!(
                    "Total Download: {:.2} MB",
                    snapshot.download_total as f64 / 1024.0 / 1024.0
                );
                println!(
                    "Total Upload: {:.2} MB",
                    snapshot.upload_total as f64 / 1024.0 / 1024.0
                );
                println!("Active Connections: {}", snapshot.connections.len());

                // Show connection change
                if snapshot.connections.len() != last_connection_count {
                    let change = snapshot.connections.len() as i32 - last_connection_count as i32;
                    let change_str = if change > 0 {
                        format!("(+{})", change)
                    } else {
                        format!("({})", change)
                    };
                    println!("Connection Change: {}", change_str);
                    last_connection_count = snapshot.connections.len();
                }

                // Show top 5 connections by traffic
                if !snapshot.connections.is_empty() {
                    println!("\n=== Top Connections by Traffic ===");

                    let mut sorted_conns = snapshot.connections.clone();
                    sorted_conns.sort_by(|a, b| {
                        (b.download + b.upload).cmp(&(a.download + a.upload))
                    });

                    for (i, conn) in sorted_conns.iter().take(5).enumerate() {
                        let host = if !conn.metadata.host.is_empty() {
                            &conn.metadata.host
                        } else {
                            &conn.metadata.destination_ip
                        };

                        println!(
                            "  {}. {} - ↓ {:.2} KB / ↑ {:.2} KB",
                            i + 1,
                            host,
                            conn.download as f64 / 1024.0,
                            conn.upload as f64 / 1024.0
                        );

                        if !conn.chains.is_empty() {
                            println!("     Chain: {}", conn.chains.join(" -> "));
                        }
                    }
                }

                println!("\n{}", "=".repeat(50));
                println!();
            }

            // Handle Ctrl+C
            _ = &mut ctrl_c => {
                println!("\n{}", "=".repeat(50));
                println!("\nStopping connection stream...");
                println!("Total updates received: {}", update_count);
                break;
            }

            // Handle channel closed
            else => {
                println!("\n{}", "=".repeat(50));
                println!("\nConnection stream ended (connection closed)");
                println!("Total updates received: {}", update_count);
                break;
            }
        }
    }

    println!("\n=== Next Steps ===");
    println!("  - List connections: cargo run --example list_connections");
    println!("  - Close connections: cargo run --example close_connections");
    println!("  - Monitor traffic: cargo run --example stream_traffic");

    Ok(())
}
