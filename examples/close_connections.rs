//! Example: Close Connections
//!
//! This example demonstrates how to close connections.
//! It shows:
//! - Connecting to mihomo
//! - Listing active connections
//! - Closing specific connections by ID
//! - Closing connections by host or process
//! - Closing all connections
//!
//! ## Prerequisites
//! - mihomo service must be running
//! - Configuration with external-controller enabled
//!
//! ## Running
//! ```bash
//! cargo run --example close_connections
//! ```

use mihomo_rs::{ConfigManager, ConnectionManager, MihomoClient, Result};

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    println!("=== Close Connections Example ===\n");

    // Connect to mihomo
    let cm = ConfigManager::new()?;
    let url = cm.get_external_controller().await?;
    let client = MihomoClient::new(&url, None)?;

    println!("Connected to: {}\n", url);

    // Use ConnectionManager for high-level operations
    let conn_mgr = ConnectionManager::new(client);

    // List current connections
    println!("Fetching active connections...");
    let connections = conn_mgr.list().await?;

    if connections.is_empty() {
        println!("\nNo active connections found.");
        println!("Start some network activity and run this example again.");
        return Ok(());
    }

    println!("Found {} active connection(s):\n", connections.len());

    // Display connections
    for (i, conn) in connections.iter().enumerate() {
        let host = if !conn.metadata.host.is_empty() {
            &conn.metadata.host
        } else {
            &conn.metadata.destination_ip
        };

        println!("{}. ID: {} -> {}", i + 1, &conn.id[..8], host);

        if !conn.chains.is_empty() {
            println!("   Chain: {}", conn.chains.join(" -> "));
        }
    }

    println!("\n=== Close Connection Examples ===\n");

    // Example 1: Close by host
    println!("1. Close connections by host");
    println!("   Example: Close all connections to 'example.com'");
    println!("   Code: conn_mgr.close_by_host(\"example.com\").await?;");
    println!();

    // Example 2: Close by process
    println!("2. Close connections by process");
    println!("   Example: Close all connections from Chrome");
    println!("   Code: conn_mgr.close_by_process(\"chrome\").await?;");
    println!();

    // Example 3: Close specific connection
    if let Some(first_conn) = connections.first() {
        println!("3. Close specific connection by ID");
        println!("   Example: Close connection {}", &first_conn.id[..8]);
        println!("   Code: conn_mgr.close(\"{}\").await?;", first_conn.id);
        println!();

        // Uncomment to actually close the connection
        // println!("   Closing connection...");
        // conn_mgr.close(&first_conn.id).await?;
        // println!("   ✓ Connection closed");
    }

    // Example 4: Close all connections
    println!("4. Close all connections");
    println!("   WARNING: This will close ALL active connections!");
    println!("   Code: conn_mgr.close_all().await?;");
    println!();

    println!("=== Safety Note ===");
    println!("This example does not actually close any connections by default.");
    println!("Uncomment the relevant lines in the code to enable closing.");
    println!();

    // Interactive example: Let user choose what to close
    println!("=== Interactive Example ===");
    println!("To close connections interactively, modify this example:");
    println!();
    println!("  1. Read user input for host/process/ID");
    println!("  2. Call appropriate close method");
    println!("  3. Verify connections were closed by listing again");
    println!();

    println!("=== Filter and Close Example ===");
    println!("You can filter connections before closing:");
    println!();
    println!("  let google_conns = conn_mgr.filter_by_host(\"google.com\").await?;");
    println!("  for conn in google_conns {{");
    println!("      conn_mgr.close(&conn.id).await?;");
    println!("  }}");
    println!();

    println!("=== Next Steps ===");
    println!("  - List connections: cargo run --example list_connections");
    println!("  - Stream connections: cargo run --example stream_connections");
    println!("  - Monitor traffic: cargo run --example stream_traffic");

    Ok(())
}
