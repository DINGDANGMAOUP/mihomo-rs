//! Example: Stream Logs
//!
//! This example demonstrates real-time log streaming from mihomo.
//! It shows:
//! - Connecting to mihomo's WebSocket log stream
//! - Receiving and displaying logs in real-time
//! - Gracefully handling disconnection
//!
//! ## Prerequisites
//! - mihomo service must be running
//! - External controller configured
//!
//! ## Running
//! ```bash
//! cargo run --example stream_logs
//! ```
//!
//! Press Ctrl+C to stop streaming.

use mihomo_rs::{ConfigManager, MihomoClient, Result};
use tokio::signal;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    println!("=== Stream Mihomo Logs ===\n");

    // Connect to mihomo
    let cm = ConfigManager::new()?;
    let url = cm.get_external_controller().await?;
    let client = MihomoClient::new(&url, None)?;

    println!("Connected to: {}", url);
    println!("Streaming all logs (no filter)");
    println!("Press Ctrl+C to stop\n");
    println!("{}", "=".repeat(80));

    // Start log streaming (no level filter = all logs)
    let mut rx = client.stream_logs(None).await?;

    // Set up Ctrl+C handler
    let ctrl_c = signal::ctrl_c();
    tokio::pin!(ctrl_c);

    let mut log_count = 0;

    loop {
        tokio::select! {
            // Receive log messages
            Some(log) = rx.recv() => {
                log_count += 1;
                println!("{}", log);
            }

            // Handle Ctrl+C
            _ = &mut ctrl_c => {
                println!("\n{}", "=".repeat(80));
                println!("\nStopping log stream...");
                println!("Total logs received: {}", log_count);
                break;
            }

            // Handle channel closed
            else => {
                println!("\n{}", "=".repeat(80));
                println!("\nLog stream ended (connection closed)");
                println!("Total logs received: {}", log_count);
                break;
            }
        }
    }

    println!("\n=== Next Steps ===");
    println!("  - Filter logs by level: cargo run --example stream_logs_filtered");
    println!("  - Monitor traffic: cargo run --example stream_traffic");
    println!("  - Check memory usage: cargo run --example memory_usage");

    Ok(())
}
