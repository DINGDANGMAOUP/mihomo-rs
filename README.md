# mihomo-rs

<div align="center">

[![Crates.io](https://img.shields.io/crates/v/mihomo-rs.svg)](https://crates.io/crates/mihomo-rs)
[![Documentation](https://docs.rs/mihomo-rs/badge.svg)](https://docs.rs/mihomo-rs)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Ask DeepWiki](https://deepwiki.com/badge.svg)](https://deepwiki.com/DINGDANGMAOUP/mihomo-rs)

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

The [examples/](./examples/) directory is now organized as a progressive 8-step path:

- [01_bootstrap.rs](./examples/01_bootstrap.rs) - Initialize managers with isolated home
- [02_config_profiles.rs](./examples/02_config_profiles.rs) - Profile save/list/switch flow
- [03_version_inventory.rs](./examples/03_version_inventory.rs) - Version inventory and default lookup
- [04_service_lifecycle_dry_run.rs](./examples/04_service_lifecycle_dry_run.rs) - ServiceManager construction and status checks
- [05_proxy_queries.rs](./examples/05_proxy_queries.rs) - Proxy groups and node queries (online)
- [06_connection_queries.rs](./examples/06_connection_queries.rs) - Connection list/filter/statistics (online)
- [07_streaming.rs](./examples/07_streaming.rs) - Log and traffic streaming entrypoints (online)
- [08_complete_workflow.rs](./examples/08_complete_workflow.rs) - End-to-end orchestration template

Run any example with:
```bash
cargo run --example 01_bootstrap
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
├── examples/           # 8 progressive examples
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

### Coverage Gate

```bash
cargo install cargo-llvm-cov --locked
rustup component add llvm-tools-preview
cargo llvm-cov --workspace --all-features --tests --summary-only --fail-under-lines 96
```

### Running Examples

```bash
# Enable logging for debugging
RUST_LOG=debug cargo run --example 01_bootstrap
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
