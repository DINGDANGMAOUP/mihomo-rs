//! Example: Concurrent Operations
//!
//! Parallel operations (e.g., testing multiple proxy delays concurrently).
//!
//! ## Running
//! ```bash
//! cargo run --example concurrent_operations
//! ```

use mihomo_rs::{ConfigManager, MihomoClient, ProxyManager, Result};
use std::time::Instant;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    println!("=== Concurrent Proxy Delay Testing ===\n");

    let cm = ConfigManager::new()?;
    let url = cm.get_external_controller().await?;
    let client = MihomoClient::new(&url, None)?;
    let pm = ProxyManager::new(client.clone());

    let proxies = pm.list_proxies().await?;
    if proxies.is_empty() {
        println!("No proxies to test");
        return Ok(());
    }

    println!("Testing {} proxies concurrently...\n", proxies.len());

    let test_url = "http://www.gstatic.com/generate_204";
    let timeout = 5000;

    // Sequential testing (for comparison)
    println!("Sequential testing:");
    let start = Instant::now();
    for proxy in proxies.iter().take(3) {
        if let Ok(delay) = client.test_delay(&proxy.name, test_url, timeout).await {
            println!("  {} - {} ms", proxy.name, delay);
        }
    }
    let sequential_time = start.elapsed();
    println!("Time: {:?}\n", sequential_time);

    // Concurrent testing
    println!("Concurrent testing:");
    let start = Instant::now();

    let mut tasks = Vec::new();
    for proxy in proxies.iter().take(3) {
        let client = client.clone();
        let name = proxy.name.clone();
        let test_url = test_url.to_string();

        tasks.push(tokio::spawn(async move {
            client
                .test_delay(&name, &test_url, timeout)
                .await
                .ok()
                .map(|delay| (name, delay))
        }));
    }

    let results = futures_util::future::join_all(tasks).await;
    for result in results {
        if let Ok(Some((name, delay))) = result {
            println!("  {} - {} ms", name, delay);
        }
    }

    let concurrent_time = start.elapsed();
    println!("Time: {:?}\n", concurrent_time);

    println!("=== Performance Gain ===");
    let speedup = sequential_time.as_secs_f64() / concurrent_time.as_secs_f64();
    println!("Speedup: {:.2}x faster", speedup);

    Ok(())
}
