# Mihomo RS

[![Crates.io](https://img.shields.io/crates/v/mihomo-rs.svg)](https://crates.io/crates/mihomo-rs)
[![Documentation](https://docs.rs/mihomo-rs/badge.svg)](https://docs.rs/mihomo-rs)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

一个用于管理和控制 mihomo 代理服务的 Rust SDK 和命令行工具。

## 特性

- 🚀 **完整的 SDK**：提供配置管理、代理控制、规则引擎和监控功能
- 🛠️ **命令行工具**：功能丰富的 CLI 工具，支持服务管理和代理控制
- 📦 **服务管理**：自动下载、安装、启动、停止、升级和卸载 mihomo 服务
- 🔄 **版本管理**：支持多版本管理和自动升级
- 📊 **实时监控**：提供连接状态、流量统计和性能监控
- 🎯 **规则引擎**：支持复杂的流量分流规则
- 🔧 **配置管理**：完整的配置文件解析和管理功能

## 安装

### 作为库使用

在 `Cargo.toml` 中添加依赖：

```toml
[dependencies]
mihomo-rs = "0.1.0"
```

### 作为命令行工具安装

```bash
cargo install mihomo-rs
```

或者从源码编译：

```bash
git clone https://github.com/mihomo-rs/mihomo-rs.git
cd mihomo-rs
cargo build --release
```

## 快速开始

### SDK 使用示例

```rust
use mihomo_rs::{MihomoClient, create_client};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志
    mihomo_rs::init_logger();
    
    // 创建客户端
    let client = create_client("http://127.0.0.1:9090", Some("your-secret".to_string()))?;
    
    // 获取版本信息
    let version = client.version().await?;
    println!("Mihomo 版本: {}", version.version);
    
    // 获取代理列表
    let proxies = client.proxies().await?;
    println!("可用代理数量: {}", proxies.len());
    
    // 获取连接信息
    let connections = client.connections().await?;
    println!("当前连接数: {}", connections.len());
    
    Ok(())
}
```

### 命令行工具使用

#### 服务管理

```bash
# 初始化服务（创建配置目录和默认配置）
mihomo-rs service init

# 下载并安装最新版本
mihomo-rs service version latest

# 启动服务
mihomo-rs service start

# 查看服务状态
mihomo-rs service status

# 停止服务
mihomo-rs service stop

# 重启服务
mihomo-rs service restart

# 升级到最新版本
mihomo-rs service upgrade

# 升级到指定版本
mihomo-rs service upgrade --version v1.19.13

# 卸载服务（保留配置）
mihomo-rs service uninstall --keep-config --confirm

# 清理备份文件（保留最新3个）
mihomo-rs service cleanup --keep 3
```

#### 代理管理

```bash
# 查看服务状态
mihomo-rs status

# 列出所有代理
mihomo-rs proxy list

# 切换代理
mihomo-rs proxy switch GLOBAL Shadowsocks

# 测试代理延迟
mihomo-rs proxy test Shadowsocks
```

#### 配置管理

```bash
# 显示当前配置
mihomo-rs config show

# 重新加载配置
mihomo-rs config reload

# 验证配置文件
mihomo-rs config validate /path/to/config.yaml
```

#### 监控功能

```bash
# 实时监控（每5秒刷新，持续60秒）
mihomo-rs monitor --interval 5 --duration 60

# 查看规则信息
mihomo-rs rules

# 查看连接信息
mihomo-rs connections

# 关闭指定连接
mihomo-rs connections close <connection-id>

# 关闭所有连接
mihomo-rs connections close-all
```

## API 文档

### 核心模块

#### MihomoClient

主要的客户端类，提供与 mihomo 服务的交互接口。

```rust
use mihomo_rs::MihomoClient;

let client = MihomoClient::new("http://127.0.0.1:9090", Some("secret".to_string()))?;
```

主要方法：
- `version()` - 获取版本信息
- `proxies()` - 获取代理列表
- `connections()` - 获取连接信息
- `switch_proxy(group, proxy)` - 切换代理
- `test_proxy_delay(proxy, url, timeout)` - 测试代理延迟
- `reload_config()` - 重新加载配置

#### ServiceManager

服务管理器，提供 mihomo 服务的生命周期管理。

```rust
use mihomo_rs::ServiceManager;

let mut service_manager = ServiceManager::new();
```

主要方法：
- `init()` - 初始化服务
- `start()` - 启动服务
- `stop()` - 停止服务
- `restart()` - 重启服务
- `status()` - 获取服务状态
- `upgrade_to_latest()` - 升级到最新版本
- `upgrade_to_version(version)` - 升级到指定版本
- `uninstall(keep_config)` - 卸载服务

#### ConfigManager

配置管理器，处理 mihomo 配置文件的解析和管理。

```rust
use mihomo_rs::config::ConfigManager;

let config_manager = ConfigManager::new();
```

#### Monitor

监控模块，提供实时的连接和流量监控。

```rust
use mihomo_rs::monitor::Monitor;

let monitor = Monitor::new(client);
```

### 错误处理

所有 API 调用都返回 `Result<T, MihomoError>`，其中 `MihomoError` 包含详细的错误信息：

```rust
use mihomo_rs::{MihomoError, Result};

match client.version().await {
    Ok(version) => println!("版本: {}", version.version),
    Err(MihomoError::Network(e)) => eprintln!("网络错误: {}", e),
    Err(MihomoError::Auth(e)) => eprintln!("认证错误: {}", e),
    Err(e) => eprintln!("其他错误: {}", e),
}
```

## 配置

### 默认配置位置

- **配置目录**: `~/.config/mihomo-rs/`
- **配置文件**: `~/.config/mihomo-rs/config.yaml`
- **二进制文件**: `~/.config/mihomo-rs/mihomo`
- **PID 文件**: `~/.config/mihomo-rs/mihomo.pid`
- **备份目录**: `~/.config/mihomo-rs/backups/`

### 配置文件示例

```yaml
port: 7890
socks-port: 7891
allow-lan: false
mode: rule
log-level: info
external-controller: 127.0.0.1:9090
secret: "your-secret-here"

proxies:
  - name: "ss1"
    type: ss
    server: server
    port: 443
    cipher: chacha20-ietf-poly1305
    password: "password"

proxy-groups:
  - name: "GLOBAL"
    type: select
    proxies:
      - ss1
      - DIRECT

rules:
  - DOMAIN-SUFFIX,google.com,GLOBAL
  - DOMAIN-KEYWORD,google,GLOBAL
  - GEOIP,CN,DIRECT
  - MATCH,GLOBAL
```

## 开发

### 构建项目

```bash
# 克隆项目
git clone https://github.com/mihomo-rs/mihomo-rs.git
cd mihomo-rs

# 构建
cargo build

# 运行测试
cargo test

# 构建发布版本
cargo build --release
```

### 运行示例

```bash
# 基本使用示例
cargo run --example basic_usage

# 高级使用示例
cargo run --example advanced_usage
```

### 测试

项目包含完整的单元测试和集成测试：

```bash
# 运行所有测试
cargo test

# 运行特定测试
cargo test test_client

# 运行集成测试
cargo test --test integration_tests

# 运行性能测试
cargo test --test performance_tests
```

## 贡献

欢迎贡献代码！请遵循以下步骤：

1. Fork 本项目
2. 创建特性分支 (`git checkout -b feature/amazing-feature`)
3. 提交更改 (`git commit -m 'Add some amazing feature'`)
4. 推送到分支 (`git push origin feature/amazing-feature`)
5. 创建 Pull Request

### 代码规范

- 使用 `cargo fmt` 格式化代码
- 使用 `cargo clippy` 检查代码质量
- 添加适当的文档注释
- 确保所有测试通过

## 许可证

本项目采用 MIT 许可证。详见 [LICENSE](LICENSE) 文件。

## 相关项目

- [mihomo](https://github.com/MetaCubeX/mihomo) - 原始的 mihomo 项目
- [clash](https://github.com/Dreamacro/clash) - Clash 代理工具

## 支持

如果您遇到问题或有建议，请：

1. 查看 [文档](https://docs.rs/mihomo-rs)
2. 搜索 [已有 Issues](https://github.com/DINGDANGMAOUP/mihomo-rs/issues)
3. 创建新的 [Issue](https://github.com/DINGDANGMAOUP/mihomo-rs/issues/new)
## 更新日志

### v0.1.0

- 初始版本发布
- 完整的 SDK 功能
- 命令行工具
- 服务管理功能
- 版本管理和升级功能
- 配置管理和监控功能