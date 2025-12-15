# mihomo-rs

<div align="center">

[![Crates.io](https://img.shields.io/crates/v/mihomo-rs.svg)](https://crates.io/crates/mihomo-rs)
[![Documentation](https://docs.rs/mihomo-rs/badge.svg)](https://docs.rs/mihomo-rs)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

[Examples](./examples/) | [API Docs](https://docs.rs/mihomo-rs)

[English](README.md) | 简体中文


一个用于 [mihomo](https://github.com/MetaCubeX/mihomo) 代理管理的 Rust SDK 和命令行工具,提供服务生命周期管理、配置处理和实时监控功能。

</div>

---

## 主要特性

- 🔧 **版本管理** - 安装、更新和切换 mihomo 版本(类似 rustup 的体验)
- ⚙️ **配置管理** - 管理多个配置文件并进行验证
- 🚀 **服务生命周期** - 启动、停止、重启 mihomo 服务,支持 PID 管理
- 🔄 **代理操作** - 列出、切换和测试代理节点及组
- 📊 **实时监控** - 流式传输日志、流量统计和内存使用情况
- 📦 **SDK 库** - 可作为库在 Rust 应用程序中使用
- 🖥️ **CLI 工具** - 命令行界面,便于管理

## 安装

### 作为库使用

添加到 `Cargo.toml`:

```toml
[dependencies]
mihomo-rs = "1.0.1"
tokio = { version = "1", features = ["full"] }
```

### 作为 CLI 工具

```bash
cargo install mihomo-rs
```

## 快速开始

### SDK 使用示例

```rust
use mihomo_rs::{Channel, ConfigManager, MihomoClient, ProxyManager, ServiceManager, VersionManager, Result};

#[tokio::main]
async fn main() -> Result<()> {
    // 1. 安装 mihomo
    let vm = VersionManager::new()?;
    vm.install_channel(Channel::Stable).await?;

    // 2. 设置配置
    let cm = ConfigManager::new()?;
    cm.ensure_default_config().await?;
    let controller_url = cm.ensure_external_controller().await?;

    // 3. 启动服务
    let binary = vm.get_binary_path(None).await?;
    let config = cm.get_current_path().await?;
    let sm = ServiceManager::new(binary, config);
    sm.start().await?;

    // 4. 使用代理管理器
    let client = MihomoClient::new(&controller_url, None)?;
    let pm = ProxyManager::new(client);

    // 列出代理组
    let groups = pm.list_groups().await?;
    for group in groups {
        println!("{}: {} ({})", group.name, group.now, group.group_type);
    }

    // 切换代理
    pm.switch("GLOBAL", "proxy-name").await?;

    Ok(())
}
```

### CLI 使用

```bash
# 安装 mihomo
mihomo-rs version install --channel stable

# 启动服务
mihomo-rs service start

# 列出代理
mihomo-rs proxy list

# 切换代理
mihomo-rs proxy switch GLOBAL proxy-name

# 监控流量
mihomo-rs monitor traffic
```

## 示例

[examples/](./examples/) 目录包含 28 个按类别组织的综合示例:

- **快速开始** - 基础示例和完整工作流程
- **版本管理** - 安装、列出和管理版本
- **配置管理** - 配置文件和外部控制器设置
- **服务管理** - 启动、停止、重启和状态检查
- **代理操作** - 列出、切换和测试代理
- **监控** - 实时日志、流量和内存监控
- **高级用法** - 自定义主目录、错误处理、并发操作
- **集成** - 首次设置和迁移指南

运行示例:
```bash
cargo run --example hello_mihomo
```

查看 [examples/README.md](./examples/README.md) 获取详细文档。

## API 概述

### 主要模块

| 模块 | 说明 |
|------|------|
| `MihomoClient` | mihomo API 的 HTTP/WebSocket 客户端 |
| `VersionManager` | 安装和管理 mihomo 版本 |
| `ConfigManager` | 管理配置文件 |
| `ServiceManager` | 控制服务生命周期 |
| `ProxyManager` | 高级代理操作 |

### 主要类型

| 类型 | 说明 |
|------|------|
| `Version` | mihomo 版本信息 |
| `ProxyNode` | 单个代理节点 |
| `ProxyGroup` | 代理组(Selector、URLTest 等) |
| `TrafficData` | 上传/下载统计 |
| `MemoryData` | 内存使用信息 |
| `Channel` | 发布渠道(Stable/Beta/Nightly) |

## 配置

### 默认位置

mihomo-rs 将数据存储在 `~/.config/mihomo-rs/`(或 `$MIHOMO_HOME`):

```
~/.config/mihomo-rs/
├── versions/           # 已安装的 mihomo 二进制文件
├── configs/            # 配置文件
├── config.toml         # mihomo-rs 设置
└── mihomo.pid          # 服务 PID 文件
```

### 自定义主目录

通过环境变量设置:

```bash
export MIHOMO_HOME=/custom/path
```

或通过代码:

```rust
let home = PathBuf::from("/custom/path");
let vm = VersionManager::with_home(home.clone())?;
```

## 开发

### 从源码构建

```bash
git clone https://github.com/DINGDANGMAOUP/mihomo-rs
cd mihomo-rs
cargo build --release
```

### 运行测试

```bash
cargo test
```

## 贡献

欢迎贡献!请参阅 [CONTRIBUTING.md](./CONTRIBUTING.md) 了解指南。

## 许可证

MIT 许可证 - 详见 [LICENSE](./LICENSE)

## 相关项目

- [mihomo](https://github.com/MetaCubeX/mihomo) - Mihomo
