//! Example: Test Proxy Delay
//!
//! This example demonstrates how to test the latency of proxy nodes.
//! It shows:
//! - Testing a single proxy's delay
//! - Testing all proxies concurrently
//! - Sorting by fastest delays
//!
//! ## Prerequisites
//! - mihomo service must be running
//! - Proxy nodes configured
//! - Internet connection
//!
//! ## Running
//! ```bash
//! cargo run --example test_delay
//! ```
//!
//! Test a specific proxy:
//! ```bash
//! PROXY_NAME="proxy1" cargo run --example test_delay
//! ```

use mihomo_rs::{ConfigManager, MihomoClient, ProxyManager, Result};
use std::env;
use tokio::time::Instant;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    println!("=== Test Proxy Delay ===\n");

    // Connect to mihomo
    let cm = ConfigManager::new()?;
    let url = cm.get_external_controller().await?;
    let client = MihomoClient::new(&url, None)?;

    println!("Connected to: {}\n", url);

    // Test URL and timeout configuration
    let test_url = "http://www.gstatic.com/generate_204";
    let timeout = 5000; // 5 seconds

    // Check if specific proxy is requested
    if let Ok(proxy_name) = env::var("PROXY_NAME") {
        // Test single proxy
        println!("Testing proxy: {}", proxy_name);
        println!("Test URL: {}", test_url);
        println!("Timeout: {} ms\n", timeout);

        let start = Instant::now();
        match client.test_delay(&proxy_name, test_url, timeout).await {
            Ok(delay) => {
                let elapsed = start.elapsed();
                println!("✓ Proxy '{}' delay: {} ms", proxy_name, delay);
                println!("  (Test completed in {:?})", elapsed);
            }
            Err(e) => {
                eprintln!("✗ Failed to test proxy '{}': {}", proxy_name, e);
                eprintln!("\nPossible reasons:");
                eprintln!("  - Proxy is offline");
                eprintln!("  - Proxy name is incorrect");
                eprintln!("  - Network timeout");
                return Err(e);
            }
        }
    } else {
        // Test all proxies
        let pm = ProxyManager::new(client.clone());

        println!("Fetching proxy nodes...");
        let proxies = pm.list_proxies().await?;

        if proxies.is_empty() {
            println!("\nNo proxy nodes found to test.");
            return Ok(());
        }

        println!("Found {} proxy node(s)", proxies.len());
        println!("Test URL: {}", test_url);
        println!("Timeout: {} ms\n", timeout);

        println!("Testing all proxies concurrently...");
        let start = Instant::now();

        // Test all proxies concurrently using tokio::spawn
        let mut test_tasks = Vec::new();

        for proxy in &proxies {
            let client_clone = client.clone();
            let proxy_name = proxy.name.clone();
            let test_url = test_url.to_string();

            let task = tokio::spawn(async move {
                let result = client_clone
                    .test_delay(&proxy_name, &test_url, timeout)
                    .await;
                (proxy_name, result)
            });

            test_tasks.push(task);
        }

        // Collect results
        let mut results = Vec::new();
        for task in test_tasks {
            if let Ok((name, result)) = task.await {
                results.push((name, result));
            }
        }

        let elapsed = start.elapsed();
        println!("✓ Completed all tests in {:?}\n", elapsed);

        // Separate successful and failed tests
        let mut successful: Vec<_> = results
            .iter()
            .filter_map(|(name, result)| result.as_ref().ok().map(|delay| (name, delay)))
            .collect();

        let failed: Vec<_> = results
            .iter()
            .filter_map(|(name, result)| result.as_ref().err().map(|err| (name, err.to_string())))
            .collect();

        // Sort by delay (fastest first)
        successful.sort_by_key(|(_, delay)| *delay);

        // Display results
        println!("=== Successful Tests ({}) ===\n", successful.len());
        for (i, (name, delay)) in successful.iter().enumerate() {
            let rating = if **delay < 100 {
                "Excellent"
            } else if **delay < 300 {
                "Good"
            } else if **delay < 1000 {
                "Fair"
            } else {
                "Slow"
            };
            println!("{}. {} - {} ms ({})", i + 1, name, delay, rating);
        }

        if !failed.is_empty() {
            println!("\n=== Failed Tests ({}) ===\n", failed.len());
            for (i, (name, error)) in failed.iter().enumerate() {
                println!("{}. {} - {}", i + 1, name, error);
            }
        }

        // Statistics
        if !successful.is_empty() {
            let fastest = successful[0];
            let slowest = successful[successful.len() - 1];
            let avg_delay: u32 =
                successful.iter().map(|(_, delay)| **delay).sum::<u32>() / successful.len() as u32;

            println!("\n=== Statistics ===");
            println!("Fastest: {} ({} ms)", fastest.0, fastest.1);
            println!("Slowest: {} ({} ms)", slowest.0, slowest.1);
            println!("Average: {} ms", avg_delay);
            println!("Success rate: {}/{}", successful.len(), results.len());
        }
    }

    println!("\n=== Next Steps ===");
    println!("  - Switch to fastest proxy: cargo run --example switch_proxy");
    println!("  - List all proxies: cargo run --example list_proxies");
    println!("  - Monitor traffic: cargo run --example stream_traffic");

    Ok(())
}
