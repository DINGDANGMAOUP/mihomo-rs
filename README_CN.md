# mihomo-rs

<div align="center">

[![Crates.io](https://img.shields.io/crates/v/mihomo-rs.svg)](https://crates.io/crates/mihomo-rs)
[![Documentation](https://docs.rs/mihomo-rs/badge.svg)](https://docs.rs/mihomo-rs)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Ask DeepWiki](https://deepwiki.com/badge.svg)](https://deepwiki.com/DINGDANGMAOUP/mihomo-rs)

[示例](./examples/) | [API 文档](https://docs.rs/mihomo-rs)

[English](README.md) | 简体中文

面向 [mihomo](https://github.com/MetaCubeX/mihomo) 的 Rust SDK 与 CLI：覆盖版本管理、配置管理、服务生命周期、代理操作和实时连接/流量监控。

</div>

## 项目能力

- 从 GitHub Release 安装/切换 mihomo 版本（`stable`、`beta`、`nightly` 或显式版本号）。
- 管理本地配置 profile（默认在 `~/.config/mihomo-rs`，可由 `$MIHOMO_HOME` 覆盖）。
- 启动/停止/重启 mihomo，并维护 PID 状态。
- 查询代理组/节点、切换代理、测试延迟。
- 查询/过滤/关闭连接。
- 通过 WebSocket 流式读取日志、流量和连接快照。

## 安装

```bash
cargo install mihomo-rs
```

作为库使用：

```toml
[dependencies]
mihomo-rs = "*"
```

## 快速开始（CLI）

```bash
# 1) 安装并设置版本
mihomo-rs version install stable
mihomo-rs version list
mihomo-rs version use v1.19.17

# 2) 启动服务（缺省配置会自动创建）
mihomo-rs service start
mihomo-rs service status

# 3) 代理操作
mihomo-rs proxy groups
mihomo-rs proxy switch GLOBAL "Proxy-A"

# 4) 监控
mihomo-rs service logs --level info
mihomo-rs service traffic
mihomo-rs connection stats
```

完整命令请执行 `mihomo-rs --help`。

## 快速开始（SDK）

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

## 渐进式示例

`examples/` 按阶段组织：

1. `01_bootstrap.rs`：隔离 home + 初始化管理器
2. `02_config_profiles.rs`：配置保存/列举/切换
3. `03_version_inventory.rs`：版本清单与默认版本读取
4. `04_service_lifecycle_dry_run.rs`：服务生命周期 dry-run
5. `05_proxy_queries.rs`：代理查询（在线）
6. `06_connection_queries.rs`：连接查询与过滤（在线）
7. `07_streaming.rs`：日志与流量流式读取（在线）
8. `08_complete_workflow.rs`：端到端工作流模板

```bash
cargo run --example 01_bootstrap
```

详情见 [examples/README.md](./examples/README.md)。

## 命令总览

- 版本：`version install|update|use|list|list-remote|uninstall`
- 配置：`config list|current|path|set|unset|use|show|delete`
- 服务：`service start|stop|restart|status|logs|traffic|memory`
- 代理：`proxy list|groups|switch|test|current`
- 连接：`connection list [--host ...] [--process ...]`、`connection stats|stream`、`connection close [--id ...|--all|--host ...|--process ...]`

其中 `proxy list` 用于查看代理节点，`proxy groups` 用于查看可切换分组，`proxy current` 用于查看各分组当前选择。


## 数据目录

默认路径：`~/.config/mihomo-rs/`

```text
~/.config/mihomo-rs/
├── versions/      # 已安装内核
├── configs/       # profile yaml
├── config.toml    # 默认版本与默认 profile
└── mihomo.pid     # PID 记录
```

自定义目录：

```bash
export MIHOMO_HOME=/custom/path
```

如果只想把 profile 配置放到 iCloud 或其他云同步目录，而版本、PID 等运行文件仍保留在本地，
可以在 `config.toml` 里单独配置 `configs` 目录：

```toml
[paths]
configs_dir = "~/Library/Mobile Documents/com~apple~CloudDocs/mihomo-rs/configs"
```

也可以临时通过环境变量覆盖：

```bash
export MIHOMO_CONFIGS_DIR=/custom/configs/path
```

也可以直接通过 CLI 写入 `config.toml`：

```bash
mihomo-rs config set configs-dir "~/Library/Mobile Documents/com~apple~CloudDocs/mihomo-rs/configs"
```

## 开发

```bash
git clone https://github.com/DINGDANGMAOUP/mihomo-rs.git
cd mihomo-rs
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
```

## 安全

见 [SECURITY.md](./SECURITY.md)。

## 贡献

见 [CONTRIBUTING.md](./CONTRIBUTING.md)。

## 许可证

MIT，见 [LICENSE](./LICENSE)。
