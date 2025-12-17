# mihomo-rs

<div align="center">

[![Crates.io](https://img.shields.io/crates/v/mihomo-rs.svg)](https://crates.io/crates/mihomo-rs)
[![Documentation](https://docs.rs/mihomo-rs/badge.svg)](https://docs.rs/mihomo-rs)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

[Examples](./examples/) | [API Docs](https://docs.rs/mihomo-rs)

English | [简体中文](README_CN.md)

A Rust SDK and CLI tool for [mihomo](https://github.com/MetaCubeX/mihomo) proxy management with service lifecycle management, configuration handling, and real-time monitoring.

</div>

---

## Features

- 🔧 **Version Management** - Install, update, and switch between mihomo versions (rustup-like experience)
- ⚙️ **Configuration Management** - Manage multiple configuration profiles with validation
- 🚀 **Service Lifecycle** - Start, stop, restart mihomo service with PID management
- 🔄 **Proxy Operations** - List, switch, and test proxy nodes and groups
- 📊 **Real-time Monitoring** - Stream logs, traffic statistics, and memory usage
- 🔌 **Connection Management** - Monitor, filter, and close active connections in real-time
- 📦 **SDK Library** - Use as a library in your Rust applications
- 🖥️ **CLI Tool** - Command-line interface for easy management

## Installation

### As a Library

Add to your `Cargo.toml`:

```toml
[dependencies]
mihomo-rs = "*"
```

### As a CLI Tool

```bash
cargo install mihomo-rs
```

## Quick Start

### SDK Usage

```rust
use mihomo_rs::{Channel, ConfigManager, MihomoClient, ProxyManager, ServiceManager, VersionManager, ConnectionManager, Result};

#[tokio::main]
async fn main() -> Result<()> {
    // 1. Install mihomo
    let vm = VersionManager::new()?;
    vm.install_channel(Channel::Stable).await?;

    // 2. Setup configuration
    let cm = ConfigManager::new()?;
    cm.ensure_default_config().await?;
    let controller_url = cm.ensure_external_controller().await?;

    // 3. Start service
    let binary = vm.get_binary_path(None).await?;
    let config = cm.get_current_path().await?;
    let sm = ServiceManager::new(binary, config);
    sm.start().await?;

    // 4. Use proxy manager
    let client = MihomoClient::new(&controller_url, None)?;
    let pm = ProxyManager::new(client.clone());

    // List proxy groups
    let groups = pm.list_groups().await?;
    for group in groups {
        println!("{}: {} ({})", group.name, group.now, group.group_type);
    }

    // Switch proxy
    pm.switch("GLOBAL", "proxy-name").await?;

    // 5. Monitor connections
    let conn_mgr = ConnectionManager::new(client.clone());

    // List active connections
    let connections = conn_mgr.list().await?;
    println!("Active connections: {}", connections.len());

    // Filter connections by host
    let filtered = conn_mgr.filter_by_host("example.com").await?;

    // Close specific connection
    if let Some(conn) = connections.first() {
        conn_mgr.close(&conn.id).await?;
    }

    // 6. Stream real-time traffic
    let mut traffic_rx = client.stream_traffic().await?;
    while let Some(traffic) = traffic_rx.recv().await {
        println!("Upload: {} KB/s, Download: {} KB/s",
            traffic.up / 1024, traffic.down / 1024);
    }

    Ok(())
}
```

### CLI Usage

```bash
# Install mihomo
mihomo-rs install stable

# Start service
mihomo-rs start

# List proxies
mihomo-rs proxy list

# Switch proxy
mihomo-rs proxy switch GLOBAL proxy-name

# Stream logs (with level filter)
mihomo-rs logs --level info

# Stream traffic statistics
mihomo-rs traffic

# Show memory usage
mihomo-rs memory

# List active connections
mihomo-rs connection list

# Show connection statistics
mihomo-rs connection stats

# Stream connections in real-time
mihomo-rs connection stream

# Close specific connection
mihomo-rs connection close <connection-id>

# Close all connections
mihomo-rs connection close-all --force
```

## Examples

The [examples/](./examples/) directory includes comprehensive examples:

### Quick Start
- [hello_mihomo.rs](./examples/hello_mihomo.rs) - Minimal example
- [basic_workflow.rs](./examples/basic_workflow.rs) - Complete beginner workflow

### Version Management
- [install_version.rs](./examples/install_version.rs) - Install specific version
- [install_by_channel.rs](./examples/install_by_channel.rs) - Install from channel
- [list_versions.rs](./examples/list_versions.rs) - List installed versions
- [manage_versions.rs](./examples/manage_versions.rs) - Version lifecycle

### Configuration
- [manage_profiles.rs](./examples/manage_profiles.rs) - Profile management
- [custom_config.rs](./examples/custom_config.rs) - Custom configuration
- [external_controller.rs](./examples/external_controller.rs) - Controller setup

### Service Management
- [service_lifecycle.rs](./examples/service_lifecycle.rs) - Start/stop/restart
- [service_status.rs](./examples/service_status.rs) - Check status
- [auto_restart.rs](./examples/auto_restart.rs) - Auto-restart logic

### Proxy Operations
- [list_proxies.rs](./examples/list_proxies.rs) - List all proxies
- [list_groups.rs](./examples/list_groups.rs) - List proxy groups
- [switch_proxy.rs](./examples/switch_proxy.rs) - Switch proxy
- [test_delay.rs](./examples/test_delay.rs) - Test latency
- [current_proxy.rs](./examples/current_proxy.rs) - Current selections

### Monitoring
- [stream_logs.rs](./examples/stream_logs.rs) - Real-time logs
- [stream_logs_filtered.rs](./examples/stream_logs_filtered.rs) - Filtered logs
- [stream_traffic.rs](./examples/stream_traffic.rs) - Traffic monitoring
- [memory_usage.rs](./examples/memory_usage.rs) - Memory usage

### Connection Management
- [list_connections.rs](./examples/list_connections.rs) - List active connections with filtering
- [close_connections.rs](./examples/close_connections.rs) - Close connections by ID, host, or process
- [stream_connections.rs](./examples/stream_connections.rs) - Real-time connection monitoring

### Advanced
- [custom_home_dir.rs](./examples/custom_home_dir.rs) - Custom home directory
- [complete_workflow.rs](./examples/complete_workflow.rs) - Full application
- [error_handling.rs](./examples/error_handling.rs) - Error patterns
- [concurrent_operations.rs](./examples/concurrent_operations.rs) - Parallel ops

### Integration
- [first_time_setup.rs](./examples/first_time_setup.rs) - First-time setup
- [migration_helper.rs](./examples/migration_helper.rs) - Migration guide

Run any example with:
```bash
cargo run --example hello_mihomo
```

See [examples/README.md](./examples/README.md) for detailed documentation.

## Architecture

```
mihomo-rs/
├── src/
│   ├── core/           # Core HTTP/WebSocket client and types
│   │   ├── client.rs   # MihomoClient (HTTP + WebSocket)
│   │   ├── types.rs    # Data structures
│   │   ├── error.rs    # Error types
│   │   ├── port.rs     # Port utilities
│   │   └── home.rs     # Home directory management
│   ├── version/        # Version management
│   │   ├── manager.rs  # VersionManager
│   │   ├── channel.rs  # Channel (Stable/Beta/Nightly)
│   │   └── download.rs # Binary downloader
│   ├── config/         # Configuration management
│   │   ├── manager.rs  # ConfigManager
│   │   └── profile.rs  # Profile struct
│   ├── service/        # Service lifecycle
│   │   ├── manager.rs  # ServiceManager
│   │   └── process.rs  # Process utilities
│   ├── proxy/          # Proxy operations
│   │   ├── manager.rs  # ProxyManager
│   │   └── test.rs     # Delay testing
│   ├── connection/     # Connection management
│   │   └── manager.rs  # ConnectionManager
│   └── cli/            # CLI application
├── examples/           # 31 comprehensive examples
└── tests/              # Integration tests
```

## API Overview

### Main Modules

| Module | Description |
|--------|-------------|
| `MihomoClient` | HTTP/WebSocket client for mihomo API |
| `VersionManager` | Install and manage mihomo versions |
| `ConfigManager` | Manage configuration profiles |
| `ServiceManager` | Control service lifecycle |
| `ProxyManager` | High-level proxy operations |
| `ConnectionManager` | Monitor and manage active connections |

### Key Types

| Type | Description |
|------|-------------|
| `Version` | Mihomo version information |
| `ProxyNode` | Individual proxy node |
| `ProxyGroup` | Proxy group (Selector, URLTest, etc.) |
| `TrafficData` | Upload/download statistics |
| `MemoryData` | Memory usage information |
| `Channel` | Release channel (Stable/Beta/Nightly) |
| `Connection` | Active connection information |
| `ConnectionSnapshot` | Real-time connections snapshot |
| `ConnectionMetadata` | Connection metadata (source, destination, process, etc.) |

### Top-level Functions

```rust
// Convenience functions for common operations
use mihomo_rs::{install_mihomo, start_service, stop_service, switch_proxy};

// Install mihomo
install_mihomo(None).await?; // Latest stable

// Service management
start_service(&config_path).await?;
stop_service(&config_path).await?;

// Proxy switching
switch_proxy("GLOBAL", "proxy-name").await?;
```

## Configuration

### Default Locations

mihomo-rs stores data in `~/.config/mihomo-rs/` (or `$MIHOMO_HOME`):

```
~/.config/mihomo-rs/
├── versions/           # Installed mihomo binaries
│   ├── v1.18.0/
│   └── v1.18.9/
├── configs/            # Configuration profiles
│   ├── default.yaml
│   └── custom.yaml
├── config.toml         # mihomo-rs settings
└── mihomo.pid          # Service PID file
```

### Custom Home Directory

Set via environment variable:

```bash
export MIHOMO_HOME=/custom/path
```

Or programmatically:

```rust
use mihomo_rs::{VersionManager, ConfigManager};
use std::path::PathBuf;

let home = PathBuf::from("/custom/path");
let vm = VersionManager::with_home(home.clone())?;
let cm = ConfigManager::with_home(home)?;
```

### Example Configuration

```yaml
# ~/.config/mihomo-rs/configs/default.yaml
port: 7890
socks-port: 7891
allow-lan: false
mode: rule
log-level: info
external-controller: 127.0.0.1:9090

proxies:
  - name: "proxy1"
    type: ss
    server: server.example.com
    port: 443
    cipher: aes-256-gcm
    password: password

proxy-groups:
  - name: "GLOBAL"
    type: select
    proxies:
      - proxy1
```

## Development

### Building from Source

```bash
git clone https://github.com/DINGDANGMAOUP/mihomo-rs
cd mihomo-rs
cargo build --release
```

### Running Tests

```bash
cargo test
```

### Running Examples

```bash
# Enable logging for debugging
RUST_LOG=debug cargo run --example basic_workflow
```

## Use Cases

### 1. System Administrators
- Automate mihomo deployment and updates
- Monitor multiple mihomo instances
- Centralized configuration management

### 2. Application Developers
- Integrate proxy management into applications
- Real-time traffic monitoring
- Programmatic proxy switching

### 3. Power Users
- Manage multiple mihomo versions
- Quick proxy testing and switching
- Custom automation scripts

### 4. CI/CD Pipelines
- Automated testing with proxies
- Isolated test environments
- Version-specific testing

## Contributing

Contributions are welcome! Please see [CONTRIBUTING.md](./CONTRIBUTING.md) for guidelines.

### Development Setup

1. Install Rust (1.70+)
2. Clone the repository
3. Run tests: `cargo test`
4. Run clippy: `cargo clippy`
5. Format code: `cargo fmt`

## License

MIT License - see [LICENSE](./LICENSE) for details.

## Related Projects

- [mihomo](https://github.com/MetaCubeX/mihomo) - Mihomo
