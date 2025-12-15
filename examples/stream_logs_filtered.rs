//! Example: Stream Logs with Filtering
//!
//! This example demonstrates log streaming with level filtering.
//! It shows:
//! - Filtering logs by level (error, warning, info, debug)
//! - Counting messages by type
//! - Color-coded output (if terminal supports it)
//!
//! ## Prerequisites
//! - mihomo service must be running
//! - External controller configured
//!
//! ## Running
//! ```bash
//! # Stream only error logs
//! cargo run --example stream_logs_filtered
//!
//! # Or specify a level via environment variable
//! LOG_LEVEL=info cargo run --example stream_logs_filtered
//! LOG_LEVEL=debug cargo run --example stream_logs_filtered
//! LOG_LEVEL=warning cargo run --example stream_logs_filtered
//! LOG_LEVEL=error cargo run --example stream_logs_filtered
//! ```
//!
//! Press Ctrl+C to stop streaming.

use mihomo_rs::{ConfigManager, MihomoClient, Result};
use std::env;
use tokio::signal;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    println!("=== Stream Filtered Mihomo Logs ===\n");

    // Get log level from environment or default to "info"
    let log_level = env::var("LOG_LEVEL").unwrap_or_else(|_| "info".to_string());

    println!("Log Levels:");
    println!("  - debug: Most verbose, shows everything");
    println!("  - info:  General information (default)");
    println!("  - warning: Warnings and errors");
    println!("  - error: Only errors\n");

    // Connect to mihomo
    let cm = ConfigManager::new()?;
    let url = cm.get_external_controller().await?;
    let client = MihomoClient::new(&url, None)?;

    println!("Connected to: {}", url);
    println!("Filtering logs at level: {}", log_level);
    println!("Press Ctrl+C to stop\n");

    println!("To change log level:");
    println!("  LOG_LEVEL=debug cargo run --example stream_logs_filtered");
    println!("\n{}", "=".repeat(80));

    // Start log streaming with level filter
    let mut rx = client.stream_logs(Some(&log_level)).await?;

    // Set up Ctrl+C handler
    let ctrl_c = signal::ctrl_c();
    tokio::pin!(ctrl_c);

    // Counters for statistics
    let mut total_count = 0;
    let mut error_count = 0;
    let mut warning_count = 0;
    let mut info_count = 0;
    let mut debug_count = 0;

    loop {
        tokio::select! {
            // Receive log messages
            Some(log) = rx.recv() => {
                total_count += 1;

                // Categorize log (simple parsing)
                if log.to_lowercase().contains("error") {
                    error_count += 1;
                    println!("[ERROR] {}", log);
                } else if log.to_lowercase().contains("warn") {
                    warning_count += 1;
                    println!("[WARN]  {}", log);
                } else if log.to_lowercase().contains("debug") {
                    debug_count += 1;
                    println!("[DEBUG] {}", log);
                } else {
                    info_count += 1;
                    println!("[INFO]  {}", log);
                }
            }

            // Handle Ctrl+C
            _ = &mut ctrl_c => {
                println!("\n{}", "=".repeat(80));
                println!("\nStopping log stream...");
                break;
            }

            // Handle channel closed
            else => {
                println!("\n{}", "=".repeat(80));
                println!("\nLog stream ended (connection closed)");
                break;
            }
        }
    }

    // Print statistics
    println!("\n=== Statistics ===");
    println!("Total logs: {}", total_count);
    println!(
        "  Errors:   {} ({:.1}%)",
        error_count,
        (error_count as f64 / total_count.max(1) as f64) * 100.0
    );
    println!(
        "  Warnings: {} ({:.1}%)",
        warning_count,
        (warning_count as f64 / total_count.max(1) as f64) * 100.0
    );
    println!(
        "  Info:     {} ({:.1}%)",
        info_count,
        (info_count as f64 / total_count.max(1) as f64) * 100.0
    );
    println!(
        "  Debug:    {} ({:.1}%)",
        debug_count,
        (debug_count as f64 / total_count.max(1) as f64) * 100.0
    );

    println!("\n=== Next Steps ===");
    println!("  - Stream all logs: cargo run --example stream_logs");
    println!("  - Monitor traffic: cargo run --example stream_traffic");
    println!("  - Check memory: cargo run --example memory_usage");

    Ok(())
}
