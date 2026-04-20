# mihomo-rs

<div align="center">

[![Crates.io](https://img.shields.io/crates/v/mihomo-rs.svg)](https://crates.io/crates/mihomo-rs)
[![Documentation](https://docs.rs/mihomo-rs/badge.svg)](https://docs.rs/mihomo-rs)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Ask DeepWiki](https://deepwiki.com/badge.svg)](https://deepwiki.com/DINGDANGMAOUP/mihomo-rs)

[Examples](./examples/) | [API Docs](https://docs.rs/mihomo-rs)

English | [简体中文](README_CN.md)

Rust SDK and CLI for [mihomo](https://github.com/MetaCubeX/mihomo): version install/switch, config profiles, service lifecycle, proxy operations, and real-time connection/traffic monitoring.

</div>

## What It Does

- Manage mihomo versions from GitHub releases (`stable`, `beta`, `nightly` or explicit tag like `v1.19.17`).
- Manage local config profiles under `~/.config/mihomo-rs` (or `$MIHOMO_HOME`).
- Start/stop/restart mihomo with PID tracking.
- Query/switch proxies and run delay tests.
- Query/filter/close active connections.
- Stream logs, traffic, and connection snapshots via WebSocket APIs.

## Install

```bash
cargo install mihomo-rs
```

Library usage:

```toml
[dependencies]
mihomo-rs = "*"
```

## Quick Start (CLI)

```bash
# 1) Install and select a kernel version
mihomo-rs install stable
mihomo-rs list
mihomo-rs default v1.19.17

# 2) Start service (auto-creates default config when missing)
mihomo-rs start
mihomo-rs status

# 3) Proxy operations
mihomo-rs proxy groups
mihomo-rs proxy switch GLOBAL "Proxy-A"

# 4) Observability
mihomo-rs logs --level info
mihomo-rs traffic
mihomo-rs connection stats
```

Run `mihomo-rs --help` for full command list.

## Quick Start (SDK)

```rust
use mihomo_rs::{Channel, ConfigManager, MihomoClient, ProxyManager, ServiceManager, VersionManager, Result};

#[tokio::main]
async fn main() -> Result<()> {
    let vm = VersionManager::new()?;
    vm.install_channel(Channel::Stable).await?;

    let cm = ConfigManager::new()?;
    cm.ensure_default_config().await?;
    let controller_url = cm.ensure_external_controller().await?;

    let binary = vm.get_binary_path(None).await?;
    let config = cm.get_current_path().await?;
    let sm = ServiceManager::new(binary, config);
    sm.start().await?;

    let client = MihomoClient::new(&controller_url, None)?;
    let pm = ProxyManager::new(client);
    let groups = pm.list_groups().await?;

    println!("groups: {}", groups.len());
    Ok(())
}
```

## Progressive Examples

Examples are organized as an incremental path:

1. `01_bootstrap.rs` - isolated home + manager bootstrap
2. `02_config_profiles.rs` - save/list/switch config profiles
3. `03_version_inventory.rs` - local version inventory/default lookup
4. `04_service_lifecycle_dry_run.rs` - service manager dry-run checks
5. `05_proxy_queries.rs` - proxy/group queries (online)
6. `06_connection_queries.rs` - connection queries/filters (online)
7. `07_streaming.rs` - logs/traffic streaming (online)
8. `08_complete_workflow.rs` - full orchestration template

```bash
cargo run --example 01_bootstrap
```

See [examples/README.md](./examples/README.md) for details.

## CLI Command Map

- Version: `install`, `update`, `default`, `list`, `list-remote`, `uninstall`
- Config: `config list|current|path|set|unset|use|show|delete`
- Service: `start`, `stop`, `restart`, `status`
- Proxy: `proxy list|groups|switch|test|current`
- Telemetry: `logs`, `traffic`, `memory`
- Connections: `connection list|stats|stream|close|close-all|filter-host|filter-process|close-by-host|close-by-process`

## Data Directory

By default, data is stored under `~/.config/mihomo-rs/`.

```text
~/.config/mihomo-rs/
├── versions/      # Installed kernels
├── configs/       # Profile yaml files
├── config.toml    # Default version/profile
└── mihomo.pid     # PID record
```

Override with:

```bash
export MIHOMO_HOME=/custom/path
```

To keep only profile files in a cloud-synced folder while leaving binaries and runtime files local,
set a dedicated config directory in `config.toml`:

```toml
[paths]
configs_dir = "~/Library/Mobile Documents/com~apple~CloudDocs/mihomo-rs/configs"
```

You can also override it temporarily with:

```bash
export MIHOMO_CONFIGS_DIR=/custom/configs/path
```

Or write it into `config.toml` via CLI:

```bash
mihomo-rs config set configs-dir "~/Library/Mobile Documents/com~apple~CloudDocs/mihomo-rs/configs"
```

## Development

```bash
git clone https://github.com/DINGDANGMAOUP/mihomo-rs.git
cd mihomo-rs
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
```

## Security

See [SECURITY.md](./SECURITY.md).

## Contributing

See [CONTRIBUTING.md](./CONTRIBUTING.md).

## License

MIT. See [LICENSE](./LICENSE).
