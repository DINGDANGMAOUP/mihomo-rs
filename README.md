# mihomo-rs

[![CI](https://github.com/DINGDANGMAOUP/mihomo-rs/workflows/CI/badge.svg)](https://github.com/DINGDANGMAOUP/mihomo-rs/actions/workflows/ci.yml)
[![Release](https://github.com/DINGDANGMAOUP/mihomo-rs/workflows/Release/badge.svg)](https://github.com/DINGDANGMAOUP/mihomo-rs/actions/workflows/release.yml)
[![Crates.io](https://img.shields.io/crates/v/mihomo-rs.svg)](https://crates.io/crates/mihomo-rs)
[![Documentation](https://docs.rs/mihomo-rs/badge.svg)](https://docs.rs/mihomo-rs)
[![License](https://img.shields.io/crates/l/mihomo-rs.svg)](https://github.com/DINGDANGMAOUP/mihomo-rs/blob/main/LICENSE)

A Rust SDK and CLI tool for mihomo proxy management, inspired by rustup's design philosophy.

## Features

- **Version Management**: Install, update, and switch between mihomo kernel versions (rustup-like)
- **Configuration Management**: Manage multiple configuration profiles
- **Service Management**: Start, stop, and restart mihomo service
- **Proxy Management**: List, switch, and test proxy nodes
- **High-level SDK**: Easy-to-use Rust library for integration

## Installation

```bash
cargo install --path .
```

## CLI Usage

### Version Management

```bash
# Install latest stable version
mihomo-rs install

# Install specific version
mihomo-rs install v1.18.0

# Install from channel (stable/beta/nightly)
mihomo-rs install stable

# Update to latest stable
mihomo-rs update

# List installed versions
mihomo-rs list

# Set default version
mihomo-rs default v1.18.0

# Uninstall a version
mihomo-rs uninstall v1.18.0
```

### Configuration Management

```bash
# List config profiles
mihomo-rs config list

# Switch to a profile
mihomo-rs config use production

# Show config content
mihomo-rs config show

# Delete a profile
mihomo-rs config delete old-config
```

### Service Management

```bash
# Start mihomo service
mihomo-rs start

# Stop mihomo service
mihomo-rs stop

# Restart mihomo service
mihomo-rs restart

# Check service status
mihomo-rs status
```

### Proxy Management

```bash
# List all proxies
mihomo-rs proxy list

# List proxy groups
mihomo-rs proxy groups

# Switch proxy in group
mihomo-rs proxy switch "PROXY" "HongKong-01"

# Test proxy delay
mihomo-rs proxy test "HongKong-01"

# Test all proxies
mihomo-rs proxy test

# Show current proxies
mihomo-rs proxy current
```

### Log Management

```bash
# Stream mihomo logs in real-time
mihomo-rs logs

# Filter logs by level
mihomo-rs logs --level info
mihomo-rs logs --level warning
mihomo-rs logs --level error
mihomo-rs logs --level debug
```

### Traffic and Memory Monitoring

```bash
# Stream real-time traffic statistics
mihomo-rs traffic

# Show current memory usage
mihomo-rs memory
```

## SDK Usage

Add to your `Cargo.toml`:

```toml
[dependencies]
mihomo-rs = "1.0"
tokio = { version = "1.0", features = ["full"] }
```

### Examples

The `examples/` directory contains comprehensive examples demonstrating all SDK features:

```bash
# Complete workflow - demonstrates all major features
cargo run --example complete_workflow

# Version management - install, list, and manage versions
cargo run --example version_management

# Configuration management - manage profiles and settings
cargo run --example config_management

# Service management - start, stop, and check status
cargo run --example service_management

# List all proxies and groups
cargo run --example list_proxies

# Detailed proxy group information
cargo run --example proxy_groups

# Switch proxy in a group
cargo run --example switch_proxy

# Test proxy delays
cargo run --example test_delay

# Stream logs in real-time
cargo run --example stream_logs

# Stream filtered logs (error level only)
cargo run --example stream_logs_filtered

# Advanced log processing example
cargo run --example stream_logs_advanced

# Stream traffic statistics
cargo run --example stream_traffic

# Get memory usage
cargo run --example get_memory

# Monitor both traffic and memory
cargo run --example monitor_traffic_memory
```

### Quick Start

```rust
use mihomo_rs::{ConfigManager, MihomoClient, ProxyManager, Result};

#[tokio::main]
async fn main() -> Result<()> {
    // Get external controller URL from config
    let config_manager = ConfigManager::new()?;
    let url = config_manager.get_external_controller().await?;

    // Create client and proxy manager
    let client = MihomoClient::new(&url, None)?;
    let proxy_manager = ProxyManager::new(client);

    // List all proxy nodes
    let proxies = proxy_manager.list_proxies().await?;
    for proxy in proxies {
        println!("{}: {:?}", proxy.name, proxy.delay);
    }

    // Switch proxy
    proxy_manager.switch("Auto", "🇭🇰 HK01 • vLess").await?;

    Ok(())
}
```

### Monitoring for Third-Party Applications

The SDK provides channel-based APIs for real-time monitoring, allowing third-party applications to process data flexibly:

```rust
use mihomo_rs::{ConfigManager, MihomoClient, Result};

#[tokio::main]
async fn main() -> Result<()> {
    let config_manager = ConfigManager::new()?;
    let url = config_manager.get_external_controller().await?;
    let client = MihomoClient::new(&url, None)?;

    // Stream logs
    let mut log_rx = client.stream_logs(None).await?;
    tokio::spawn(async move {
        while let Some(log) = log_rx.recv().await {
            // Process logs: send to logging system, store in DB, etc.
        }
    });

    // Stream traffic statistics
    let mut traffic_rx = client.stream_traffic().await?;
    tokio::spawn(async move {
        while let Some(traffic) = traffic_rx.recv().await {
            // Monitor bandwidth: traffic.up, traffic.down (bytes/s)
            // Send to monitoring dashboard, trigger alerts, etc.
        }
    });

    // Query memory periodically
    let memory = client.get_memory().await?;
    println!("Memory: {} MB / {} MB",
        memory.in_use / 1024 / 1024,
        memory.os_limit / 1024 / 1024
    );

    Ok(())
}
```

### Advanced Usage

```rust
use mihomo_rs::{
    VersionManager, ConfigManager, ServiceManager,
    MihomoClient, ProxyManager, Channel, Result
};

#[tokio::main]
async fn main() -> Result<()> {
    // Version management
    let vm = VersionManager::new()?;
    vm.install_channel(Channel::Stable).await?;
    vm.set_default("v1.18.0").await?;

    // Configuration management
    let cm = ConfigManager::new()?;
    cm.set_current("production").await?;
    let config_path = cm.get_current_path().await?;

    // Service management
    let binary = vm.get_binary_path(None).await?;
    let sm = ServiceManager::new(binary, config_path);
    sm.start().await?;

    // Proxy management with config-based URL
    let url = cm.get_external_controller().await?;
    let client = MihomoClient::new(&url, None)?;
    let pm = ProxyManager::new(client);

    // List groups and their current proxies
    let groups = pm.list_groups().await?;
    for group in groups {
        println!("{} -> {}", group.name, group.now);
    }

    // Stream traffic statistics
    let mut traffic_rx = client.stream_traffic().await?;
    tokio::spawn(async move {
        while let Some(traffic) = traffic_rx.recv().await {
            println!("↑ {} KB/s  ↓ {} KB/s",
                traffic.up / 1024, traffic.down / 1024);
        }
    });

    // Get memory usage
    let memory = client.get_memory().await?;
    println!("Memory: {} MB", memory.in_use / 1024 / 1024);

    Ok(())
}
```

## Architecture

```
mihomo-rs/
├── src/
│   ├── lib.rs              # SDK public API
│   ├── main.rs             # CLI entry point
│   ├── core/               # Core SDK modules
│   │   ├── client.rs       # HTTP client for mihomo API
│   │   ├── error.rs        # Error types
│   │   └── types.rs        # Common types
│   ├── version/            # Version management
│   │   ├── manager.rs      # Install/switch versions
│   │   ├── channel.rs      # Stable/beta/nightly
│   │   └── download.rs     # Download kernels
│   ├── config/             # Configuration management
│   │   ├── manager.rs      # Config operations
│   │   └── profile.rs      # Multiple profiles
│   ├── service/            # Service lifecycle
│   │   ├── manager.rs      # Start/stop/restart
│   │   └── process.rs      # Process management
│   ├── proxy/              # Proxy management
│   │   ├── manager.rs      # Proxy operations
│   │   └── test.rs         # Delay testing
│   └── cli/                # CLI-specific
│       ├── commands.rs     # Command definitions
│       └── output.rs       # Output formatting
```

## Configuration

mihomo-rs stores its configuration in `~/.config/mihomo-rs/` by default:

- `config.toml` - mihomo-rs settings (default version, profile)
- `versions/` - Installed mihomo kernel versions
- `configs/` - Configuration profiles
- `mihomo.pid` - Service PID file

### Custom Home Directory

You can customize the home directory in two ways:

**1. Environment Variable (CLI usage):**

```bash
# Use a custom directory
export MIHOMO_HOME=/path/to/custom/dir
mihomo-rs list

# Or set it for a single command
MIHOMO_HOME=/path/to/custom/dir mihomo-rs list
```

**2. Programmatically (SDK usage):**

```rust
use mihomo_rs::{VersionManager, ConfigManager};
use std::path::PathBuf;

let home = PathBuf::from("/opt/mihomo");
let vm = VersionManager::with_home(home.clone())?;
let cm = ConfigManager::with_home(home)?;

// All operations now use the custom directory
vm.install("v1.18.0").await?;
```

This is useful for:
- Running multiple isolated instances
- Using a different storage location
- Testing without affecting your main configuration
- Multi-tenant applications

## License

MIT

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.
