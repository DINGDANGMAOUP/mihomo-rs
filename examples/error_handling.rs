//! Example: Error Handling
//!
//! Comprehensive error handling patterns and recovery strategies.
//!
//! ## Running
//! ```bash
//! cargo run --example error_handling
//! ```

use mihomo_rs::{ConfigManager, MihomoClient, MihomoError, Result};

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    println!("=== Error Handling Patterns ===\n");

    // Pattern 1: Basic error propagation
    println!("1. Basic Error Propagation:");
    match ConfigManager::new() {
        Ok(_) => println!("   ✓ ConfigManager created"),
        Err(e) => println!("   ✗ Error: {}", e),
    }
    println!();

    // Pattern 2: Detailed error matching
    println!("2. Detailed Error Matching:");
    let cm = ConfigManager::new()?;
    match cm.get_external_controller().await {
        Ok(url) => println!("   ✓ Controller: {}", url),
        Err(MihomoError::Config(msg)) => println!("   ✗ Config error: {}", msg),
        Err(MihomoError::Io(e)) => println!("   ✗ IO error: {}", e),
        Err(e) => println!("   ✗ Other error: {}", e),
    }
    println!();

    // Pattern 3: Retry logic
    println!("3. Retry Logic:");
    let max_retries = 3;
    for attempt in 1..=max_retries {
        match try_connect(&cm).await {
            Ok(_) => {
                println!("   ✓ Connected on attempt {}", attempt);
                break;
            }
            Err(e) if attempt < max_retries => {
                println!("   ✗ Attempt {} failed: {}", attempt, e);
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            }
            Err(e) => {
                println!("   ✗ All attempts failed: {}", e);
            }
        }
    }
    println!();

    // Pattern 4: Graceful degradation
    println!("4. Graceful Degradation:");
    let url = cm.get_external_controller().await.unwrap_or_else(|_| {
        println!("   ⚠ Using fallback URL");
        "http://127.0.0.1:9090".to_string()
    });
    println!("   Using URL: {}", url);

    println!("\n=== Error Types ===");
    println!("  - MihomoError::Http: Network errors");
    println!("  - MihomoError::Io: File system errors");
    println!("  - MihomoError::Config: Configuration errors");
    println!("  - MihomoError::NotFound: Resource not found");

    Ok(())
}

async fn try_connect(cm: &ConfigManager) -> Result<()> {
    let url = cm.get_external_controller().await?;
    let client = MihomoClient::new(&url, None)?;
    client.get_version().await?;
    Ok(())
}
