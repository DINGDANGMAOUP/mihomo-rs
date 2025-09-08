//! # Mihomo RS SDK
//!
//! 一个用于管理和控制 mihomo 代理服务的 Rust SDK。
//! 提供配置管理、代理控制、规则引擎和监控功能。

pub mod client;
pub mod config;
pub mod error;
pub mod logger;
pub mod monitor;
pub mod proxy;
pub mod retry;
pub mod rules;
pub mod service;
pub mod types;
pub mod utils;

// 重新导出主要的公共接口
pub use client::MihomoClient;
pub use config::{Config, ProxyConfig, RuleConfig};
pub use error::{ErrorCategory, ErrorContext, ErrorInfo, MihomoError, Result};
pub use retry::{retry_async, retry_sync, RetryExecutor, RetryPolicy};
pub use service::{ServiceConfig, ServiceManager, ServiceStatus, VersionInfo};
pub use types::{
    ApiResponse, Connection, ConnectionMetadata, ConnectionsResponse, DelayHistory, DnsQuery,
    EmptyResponse, GcStats, HealthCheckResult, LogEntry, LogLevel, Memory, MemoryUsage, Provider,
    ProviderHealthResponse, ProxyGroup, ProxyGroupType, ProxyItem, ProxyNode, ProxyType, Rule,
    RuleProvider, RuleStats, RuleType, RuntimeInfo, ServiceConfigInfo, ServiceConfigUpdate,
    ServiceStats, SubscriptionInfo, SystemInfo, Traffic, Version,
};

/// SDK 版本信息
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// 初始化日志系统
///
/// # Arguments
///
/// * `config` - 日志配置，如果为None则使用默认配置
///
/// # Examples
///
/// ```
/// use mihomo_rs::logger::LoggerConfig;
///
/// // 使用默认配置
/// mihomo_rs::init_logger(None);
///
/// // 使用自定义配置
/// let config = LoggerConfig {
///     level: log::LevelFilter::Debug,
///     show_timestamp: true,
///     show_module: true,
///     ..Default::default()
/// };
/// mihomo_rs::init_logger(Some(config));
/// ```
pub fn init_logger(config: Option<logger::LoggerConfig>) {
    logger::init_logger(config);
}

/// 使用默认配置初始化日志系统（向后兼容）
///
/// # Examples
///
/// ```
/// mihomo_rs::init_default_logger();
/// ```
pub fn init_default_logger() {
    logger::init_logger(None);
}

/// 创建一个新的 Mihomo 客户端实例
///
/// # Arguments
///
/// * `base_url` - mihomo 服务的基础 URL
/// * `secret` - API 访问密钥（可选）
///
/// # Examples
///
/// ```
/// use mihomo_rs::create_client;
///
/// let client = create_client("http://127.0.0.1:9090", Some("your-secret".to_string()));
/// ```
pub fn create_client(base_url: &str, secret: Option<String>) -> Result<MihomoClient> {
    MihomoClient::new(base_url, secret)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[allow(clippy::const_is_empty)]
    fn test_version() {
        assert!(!VERSION.is_empty(), "Version should not be empty");
    }

    #[test]
    fn test_create_client() {
        let client = create_client("http://127.0.0.1:9090", None);
        assert!(client.is_ok());
    }
}
