# mihomo-rs Examples

This directory contains comprehensive examples demonstrating all features of the mihomo-rs SDK.

## Running Examples

To run an example:

```bash
# Run example by full path
cargo run --example 01_quickstart/hello_mihomo

# Or with just the name
cargo run --example hello_mihomo
```

Enable logging for more details:

```bash
RUST_LOG=debug cargo run --example hello_mihomo
```

## Example Categories

### 01_quickstart/

Basic examples for getting started:

- **hello_mihomo.rs** - Minimal example showing basic client usage (get version, list proxies)
- **basic_workflow.rs** - Common beginner workflow (install → config → start → list → stop)

**Prerequisites**: None (examples will guide you through setup)

### 02_version_management/

Examples for managing mihomo versions:

- **install_version.rs** - Install a specific mihomo version from GitHub releases
- **install_by_channel.rs** - Install latest version from a channel (Stable/Beta/Nightly)
- **list_versions.rs** - Display all installed versions with details
- **manage_versions.rs** - Complete version lifecycle (install → switch → uninstall)

**Prerequisites**: Internet connection for downloading mihomo binaries

### 03_configuration/

Examples for configuration and profile management:

- **manage_profiles.rs** - Create, list, switch, and delete configuration profiles
- **custom_config.rs** - Create and customize mihomo YAML configuration
- **external_controller.rs** - Setup and verify external controller for API access

**Prerequisites**: mihomo installed (run version_management examples first)

### 04_service/

Examples for service lifecycle management:

- **service_lifecycle.rs** - Start, stop, and restart mihomo service
- **service_status.rs** - Check service status and get PID information
- **auto_restart.rs** - Automatic restart with health check logic

**Prerequisites**: mihomo installed and configured

### 05_proxy_operations/

Examples for proxy management (most commonly used):

- **list_proxies.rs** - List all proxy nodes with details (type, delay, status)
- **list_groups.rs** - Display proxy groups and their members
- **switch_proxy.rs** - Switch the active proxy in a group
- **test_delay.rs** - Test latency of proxy nodes
- **current_proxy.rs** - Get current proxy selections for all groups

**Prerequisites**: mihomo service running with valid configuration

### 06_monitoring/

Examples for real-time monitoring:

- **stream_logs.rs** - Real-time log streaming from mihomo
- **stream_logs_filtered.rs** - Log streaming with level filtering (error, warning, info, debug)
- **stream_traffic.rs** - Traffic monitoring with upload/download rate calculation
- **memory_usage.rs** - Monitor mihomo memory usage

**Prerequisites**: mihomo service running

### 07_advanced/

Advanced usage patterns:

- **custom_home_dir.rs** - Use custom home directory for mihomo data (useful for multi-user setups)
- **complete_workflow.rs** - Full application example (setup → run → monitor → shutdown)
- **error_handling.rs** - Comprehensive error handling patterns and recovery strategies
- **concurrent_operations.rs** - Parallel operations (e.g., testing multiple proxy delays concurrently)

**Prerequisites**: Understanding of basic examples

### 08_integration/

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

1. Start with **01_quickstart/hello_mihomo.rs** or **basic_workflow.rs**
2. Learn version management with **02_version_management/** examples
3. Explore proxy operations with **05_proxy_operations/** examples
4. Try real-time monitoring with **06_monitoring/** examples
5. Study advanced patterns in **07_advanced/** examples

## See Also

- [Main README](../README.md) - Project overview and installation
- [API Documentation](https://docs.rs/mihomo-rs) - Complete API reference
- [Source Code](../src/) - SDK implementation

## Contributing

Found an issue or have an improvement? See [CONTRIBUTING.md](../CONTRIBUTING.md) for guidelines.
