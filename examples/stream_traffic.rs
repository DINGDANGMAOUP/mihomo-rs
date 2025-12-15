//! Example: Stream Traffic
//!
//! This example demonstrates real-time traffic monitoring.
//! It shows:
//! - Streaming upload/download statistics
//! - Calculating transfer rates
//! - Displaying human-readable sizes
//!
//! ## Prerequisites
//! - mihomo service must be running
//! - External controller configured
//! - Some network traffic (browse web, etc.)
//!
//! ## Running
//! ```bash
//! cargo run --example stream_traffic
//! ```
//!
//! Press Ctrl+C to stop monitoring.

use mihomo_rs::{ConfigManager, MihomoClient, Result};
use std::time::Duration;
use tokio::signal;
use tokio::time::{interval, Instant};

// Helper function to format bytes in human-readable format
fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit_index = 0;

    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }

    format!("{:.2} {}", size, UNITS[unit_index])
}

// Helper function to format speed (bytes per second)
fn format_speed(bytes_per_sec: f64) -> String {
    const UNITS: &[&str] = &["B/s", "KB/s", "MB/s", "GB/s"];
    let mut speed = bytes_per_sec;
    let mut unit_index = 0;

    while speed >= 1024.0 && unit_index < UNITS.len() - 1 {
        speed /= 1024.0;
        unit_index += 1;
    }

    format!("{:.2} {}", speed, UNITS[unit_index])
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    println!("=== Stream Traffic Statistics ===\n");

    // Connect to mihomo
    let cm = ConfigManager::new()?;
    let url = cm.get_external_controller().await?;
    let client = MihomoClient::new(&url, None)?;

    println!("Connected to: {}", url);
    println!("Monitoring traffic in real-time");
    println!("Press Ctrl+C to stop\n");
    println!("{}", "=".repeat(80));

    // Start traffic streaming
    let mut rx = client.stream_traffic().await?;

    // Set up Ctrl+C handler
    let ctrl_c = signal::ctrl_c();
    tokio::pin!(ctrl_c);

    // Set up update interval for rate calculation
    let mut update_interval = interval(Duration::from_secs(1));

    // Track previous values for rate calculation
    let mut prev_up = 0u64;
    let mut prev_down = 0u64;
    let mut prev_time = Instant::now();

    // Totals
    let mut total_up = 0u64;
    let mut total_down = 0u64;
    let start_time = Instant::now();

    // Peak speeds
    let mut peak_up_speed = 0f64;
    let mut peak_down_speed = 0f64;

    loop {
        tokio::select! {
            // Receive traffic data
            Some(traffic) = rx.recv() => {
                let now = Instant::now();
                let elapsed = now.duration_since(prev_time).as_secs_f64();

                if elapsed > 0.0 {
                    // Calculate rates (bytes per second)
                    let up_diff = traffic.up.saturating_sub(prev_up);
                    let down_diff = traffic.down.saturating_sub(prev_down);

                    let up_speed = up_diff as f64 / elapsed;
                    let down_speed = down_diff as f64 / elapsed;

                    // Update peaks
                    peak_up_speed = peak_up_speed.max(up_speed);
                    peak_down_speed = peak_down_speed.max(down_speed);

                    // Update totals
                    total_up = traffic.up;
                    total_down = traffic.down;

                    // Display current stats
                    println!("\rUpload:   {} ({})  |  Download: {} ({})  ",
                        format_bytes(traffic.up),
                        format_speed(up_speed),
                        format_bytes(traffic.down),
                        format_speed(down_speed),
                    );

                    prev_up = traffic.up;
                    prev_down = traffic.down;
                    prev_time = now;
                }
            }

            // Periodic summary (every second)
            _ = update_interval.tick() => {
                // Just tick, actual update happens when we receive traffic data
            }

            // Handle Ctrl+C
            _ = &mut ctrl_c => {
                println!("\n{}", "=".repeat(80));
                println!("\nStopping traffic monitoring...");
                break;
            }

            // Handle channel closed
            else => {
                println!("\n{}", "=".repeat(80));
                println!("\nTraffic stream ended (connection closed)");
                break;
            }
        }
    }

    // Print final statistics
    let total_time = start_time.elapsed().as_secs_f64();

    println!("\n=== Session Statistics ===");
    println!("Duration: {:.1} seconds\n", total_time);

    println!("Total Transfer:");
    println!("  Upload:   {}", format_bytes(total_up));
    println!("  Download: {}", format_bytes(total_down));
    println!("  Combined: {}\n", format_bytes(total_up + total_down));

    if total_time > 0.0 {
        println!("Average Speed:");
        println!("  Upload:   {}", format_speed(total_up as f64 / total_time));
        println!(
            "  Download: {}",
            format_speed(total_down as f64 / total_time)
        );
        println!();
    }

    println!("Peak Speed:");
    println!("  Upload:   {}", format_speed(peak_up_speed));
    println!("  Download: {}", format_speed(peak_down_speed));

    println!("\n=== Next Steps ===");
    println!("  - Monitor memory: cargo run --example memory_usage");
    println!("  - View logs: cargo run --example stream_logs");
    println!("  - Advanced workflow: cargo run --example complete_workflow");

    Ok(())
}
