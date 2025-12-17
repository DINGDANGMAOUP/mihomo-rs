# mihomo-rs Examples

This directory contains comprehensive examples demonstrating all features of the mihomo-rs SDK.

## Running Examples

To run an example:

```bash
cargo run --example hello_mihomo
```

Enable logging for more details:

```bash
RUST_LOG=debug cargo run --example hello_mihomo
```

## Example Categories


Basic examples for getting started:

- **hello_mihomo.rs** - Minimal example showing basic client usage (get version, list proxies)
- **basic_workflow.rs** - Common beginner workflow (install → config → start → list → stop)

**Prerequisites**: None (examples will guide you through setup)


Examples for managing mihomo versions:

- **install_version.rs** - Install a specific mihomo version from GitHub releases
- **install_by_channel.rs** - Install latest version from a channel (Stable/Beta/Nightly)
- **list_versions.rs** - Display all installed versions with details
- **manage_versions.rs** - Complete version lifecycle (install → switch → uninstall)

**Prerequisites**: Internet connection for downloading mihomo binaries


Examples for configuration and profile management:

- **manage_profiles.rs** - Create, list, switch, and delete configuration profiles
- **custom_config.rs** - Create and customize mihomo YAML configuration
- **external_controller.rs** - Setup and verify external controller for API access

**Prerequisites**: mihomo installed (run version_management examples first)


Examples for service lifecycle management:

- **service_lifecycle.rs** - Start, stop, and restart mihomo service
- **service_status.rs** - Check service status and get PID information
- **auto_restart.rs** - Automatic restart with health check logic

**Prerequisites**: mihomo installed and configured


Examples for proxy management (most commonly used):

- **list_proxies.rs** - List all proxy nodes with details (type, delay, status)
- **list_groups.rs** - Display proxy groups and their members
- **switch_proxy.rs** - Switch the active proxy in a group
- **test_delay.rs** - Test latency of proxy nodes
- **current_proxy.rs** - Get current proxy selections for all groups

**Prerequisites**: mihomo service running with valid configuration


Examples for real-time monitoring:

- **stream_logs.rs** - Real-time log streaming from mihomo
- **stream_logs_filtered.rs** - Log streaming with level filtering (error, warning, info, debug)
- **stream_traffic.rs** - Traffic monitoring with upload/download rate calculation
- **memory_usage.rs** - Monitor mihomo memory usage

**Prerequisites**: mihomo service running


Examples for monitoring and managing active connections:

- **list_connections.rs** - List all active connections with detailed information (source, destination, proxy chain, traffic)
- **close_connections.rs** - Close connections by ID, host, or process name
- **stream_connections.rs** - Real-time connection monitoring with statistics and traffic updates

**Prerequisites**: mihomo service running with active network traffic


Advanced usage patterns:

- **custom_home_dir.rs** - Use custom home directory for mihomo data (useful for multi-user setups)
- **complete_workflow.rs** - Full application example (setup → run → monitor → shutdown)
- **error_handling.rs** - Comprehensive error handling patterns and recovery strategies
- **concurrent_operations.rs** - Parallel operations (e.g., testing multiple proxy delays concurrently)

**Prerequisites**: Understanding of basic examples


Integration scenarios and migration helpers:

- **first_time_setup.rs** - Complete first-time setup guide for new users
- **migration_helper.rs** - Migrate from manual mihomo setup to mihomo-rs management

**Prerequisites**: None (comprehensive setup examples)

## Common Patterns

### Error Handling

All examples use the standard `Result<()>` pattern:

```rust
use mihomo_rs::Result;

#[tokio::main]
async fn main() -> Result<()> {
    // Operations that may fail use the ? operator
    let client = MihomoClient::new("http://127.0.0.1:9090", None)?;
    let version = client.get_version().await?;

    println!("Mihomo version: {}", version.version);
    Ok(())
}
```

### Client Initialization

Most examples need to connect to a running mihomo instance:

```rust
use mihomo_rs::{ConfigManager, MihomoClient};

#[tokio::main]
async fn main() -> Result<()> {
    // Get the external controller URL from configuration
    let cm = ConfigManager::new()?;
    let url = cm.get_external_controller().await?;

    // Create client (with optional authentication secret)
    let client = MihomoClient::new(&url, None)?;

    // Use client...
    Ok(())
}
```

### Service Lifecycle

Starting and stopping the mihomo service:

```rust
use mihomo_rs::{VersionManager, ConfigManager, ServiceManager};

#[tokio::main]
async fn main() -> Result<()> {
    let vm = VersionManager::new()?;
    let cm = ConfigManager::new()?;

    // Ensure configuration exists
    cm.ensure_default_config().await?;
    cm.ensure_external_controller().await?;

    // Get paths
    let binary = vm.get_binary_path(None).await?;
    let config = cm.get_current_path().await?;

    // Create and start service
    let sm = ServiceManager::new(binary, config);
    sm.start().await?;

    // ... do work ...

    sm.stop().await?;
    Ok(())
}
```

### Stream Handling

Working with real-time streams (logs, traffic):

```rust
use mihomo_rs::MihomoClient;

#[tokio::main]
async fn main() -> Result<()> {
    let client = MihomoClient::new("http://127.0.0.1:9090", None)?;

    // Get stream receiver
    let mut rx = client.stream_logs(None).await?;

    // Process messages
    while let Some(log) = rx.recv().await {
        println!("{}", log);
    }

    Ok(())
}
```

### Connection Management

Working with active connections:

```rust
use mihomo_rs::{ConfigManager, MihomoClient, ConnectionManager};

#[tokio::main]
async fn main() -> Result<()> {
    let cm = ConfigManager::new()?;
    let url = cm.get_external_controller().await?;
    let client = MihomoClient::new(&url, None)?;

    // Create connection manager
    let conn_mgr = ConnectionManager::new(client);

    // List active connections
    let connections = conn_mgr.list().await?;
    println!("Active connections: {}", connections.len());

    // Get connection statistics
    let (download, upload, count) = conn_mgr.get_statistics().await?;
    println!("Total: {} connections, ↓ {} KB, ↑ {} KB",
        count, download / 1024, upload / 1024);

    // Filter connections by host
    let filtered = conn_mgr.filter_by_host("example.com").await?;

    // Close specific connection
    if let Some(conn) = connections.first() {
        conn_mgr.close(&conn.id).await?;
    }

    // Stream real-time connection updates
    let mut rx = conn_mgr.stream().await?;
    while let Some(snapshot) = rx.recv().await {
        println!("Connections: {}, Traffic: ↓ {} KB / ↑ {} KB",
            snapshot.connections.len(),
            snapshot.download_total / 1024,
            snapshot.upload_total / 1024);
    }

    Ok(())
}
```

## Troubleshooting

### "Connection refused" errors

Make sure mihomo service is running:

```bash
cargo run --example service_lifecycle
```

Or check status:

```bash
cargo run --example service_status
```

### "mihomo not found" errors

Install mihomo first:

```bash
cargo run --example install_version
```

Or use the quick install:

```bash
cargo run --example basic_workflow
```

### "Config file not found" errors

Ensure default configuration exists:

```rust
let cm = ConfigManager::new()?;
cm.ensure_default_config().await?;
```

### Permission errors

On Linux/macOS, you may need to make the mihomo binary executable:

```bash
chmod +x ~/.config/mihomo-rs/versions/*/mihomo
```

## Next Steps

1. Start with **hello_mihomo.rs** or **basic_workflow.rs**
2. Learn version management with **/** examples
3. Explore proxy operations with **/** examples
4. Try real-time monitoring with **/** examples
5. Manage connections with **list_connections.rs**, **close_connections.rs**, and **stream_connections.rs**
6. Study advanced patterns in **/** examples

## See Also

- [Main README](../README.md) - Project overview and installation
- [API Documentation](https://docs.rs/mihomo-rs) - Complete API reference
- [Source Code](../src/) - SDK implementation

## Contributing

Found an issue or have an improvement? See [CONTRIBUTING.md](../CONTRIBUTING.md) for guidelines.
