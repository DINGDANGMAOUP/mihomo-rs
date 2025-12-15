//! Example: Memory Usage
//!
//! This example demonstrates how to monitor mihomo's memory usage.
//! It shows:
//! - Getting current memory statistics
//! - Displaying in human-readable format
//! - Calculating usage percentage
//! - Continuous monitoring option
//!
//! ## Prerequisites
//! - mihomo service must be running
//! - External controller configured
//!
//! ## Running
//! ```bash
//! # Single check
//! cargo run --example memory_usage
//!
//! # Continuous monitoring
//! MONITOR=true cargo run --example memory_usage
//! ```
//!
//! Press Ctrl+C to stop continuous monitoring.

use mihomo_rs::{ConfigManager, MihomoClient, Result};
use std::env;
use std::time::Duration;
use tokio::signal;
use tokio::time::sleep;

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

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    // Check if continuous monitoring is requested
    let monitor_mode = env::var("MONITOR").unwrap_or_default() == "true";

    if monitor_mode {
        println!("=== Continuous Memory Monitoring ===\n");
    } else {
        println!("=== Mihomo Memory Usage ===\n");
    }

    // Connect to mihomo
    let cm = ConfigManager::new()?;
    let url = cm.get_external_controller().await?;
    let client = MihomoClient::new(&url, None)?;

    println!("Connected to: {}\n", url);

    if monitor_mode {
        println!("Monitoring memory every 2 seconds");
        println!("Press Ctrl+C to stop\n");
        println!(
            "{:<20} {:<15} {:<15} {:<10}",
            "Time", "In Use", "OS Limit", "Usage %"
        );
        println!("{}", "-".repeat(65));

        // Set up Ctrl+C handler
        let ctrl_c = signal::ctrl_c();
        tokio::pin!(ctrl_c);

        // Track min/max for statistics
        let mut min_usage = u64::MAX;
        let mut max_usage = 0u64;
        let mut sample_count = 0;
        let mut total_usage = 0u64;

        loop {
            tokio::select! {
                _ = sleep(Duration::from_secs(2)) => {
                    match client.get_memory().await {
                        Ok(memory) => {
                            let percentage = if memory.os_limit > 0 {
                                (memory.in_use as f64 / memory.os_limit as f64) * 100.0
                            } else {
                                0.0
                            };

                            // Update statistics
                            min_usage = min_usage.min(memory.in_use);
                            max_usage = max_usage.max(memory.in_use);
                            total_usage += memory.in_use;
                            sample_count += 1;

                            // Get current time
                            let now = chrono::Local::now();
                            let time_str = now.format("%H:%M:%S").to_string();

                            println!(
                                "{:<20} {:<15} {:<15} {:<10.2}",
                                time_str,
                                format_bytes(memory.in_use),
                                format_bytes(memory.os_limit),
                                percentage
                            );
                        }
                        Err(e) => {
                            eprintln!("Error fetching memory: {}", e);
                        }
                    }
                }

                _ = &mut ctrl_c => {
                    println!("\n{}", "-".repeat(65));
                    println!("\nStopping monitoring...");

                    if sample_count > 0 {
                        let avg_usage = total_usage / sample_count;
                        println!("\n=== Statistics ===");
                        println!("Samples: {}", sample_count);
                        println!("Min usage: {}", format_bytes(min_usage));
                        println!("Max usage: {}", format_bytes(max_usage));
                        println!("Avg usage: {}", format_bytes(avg_usage));
                    }

                    break;
                }
            }
        }
    } else {
        // Single check mode
        println!("Fetching memory statistics...\n");

        match client.get_memory().await {
            Ok(memory) => {
                // Calculate percentage
                let percentage = if memory.os_limit > 0 {
                    (memory.in_use as f64 / memory.os_limit as f64) * 100.0
                } else {
                    0.0
                };

                println!("Memory Usage:");
                println!(
                    "  In Use:   {} ({} bytes)",
                    format_bytes(memory.in_use),
                    memory.in_use
                );
                println!(
                    "  OS Limit: {} ({} bytes)",
                    format_bytes(memory.os_limit),
                    memory.os_limit
                );
                println!("  Usage:    {:.2}%", percentage);

                // Memory assessment
                println!("\nAssessment:");
                if percentage < 50.0 {
                    println!("  ✓ Memory usage is normal");
                } else if percentage < 80.0 {
                    println!("  ⚠ Memory usage is moderate");
                } else {
                    println!("  ✗ Memory usage is high");
                }

                // Calculate available memory
                let available = memory.os_limit.saturating_sub(memory.in_use);
                println!("  Available: {}", format_bytes(available));
            }
            Err(e) => {
                eprintln!("Error: Failed to fetch memory statistics");
                eprintln!("  {}", e);
                return Err(e);
            }
        }

        println!("\n=== Tip ===");
        println!("For continuous monitoring:");
        println!("  MONITOR=true cargo run --example memory_usage");
    }

    println!("\n=== Next Steps ===");
    println!("  - Monitor traffic: cargo run --example stream_traffic");
    println!("  - View logs: cargo run --example stream_logs");
    println!("  - Complete workflow: cargo run --example complete_workflow");

    Ok(())
}
