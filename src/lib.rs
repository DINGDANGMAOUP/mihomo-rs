//! # Mihomo RS SDK
//!
//! 一个用于管理和控制 mihomo 代理服务的 Rust SDK。
//! 提供配置管理、代理控制、规则引擎和监控功能。

pub mod config;
pub mod proxy;
pub mod rules;
pub mod monitor;
pub mod error;
pub mod types;
pub mod client;
pub mod utils;

// 重新导出主要的公共接口
pub use client::MihomoClient;
pub use config::{Config, ProxyConfig, RuleConfig};
pub use error::{MihomoError, Result};
pub use types::*;

/// SDK 版本信息
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// 初始化日志系统
/// 
/// # Examples
/// 
/// ```
/// mihomo_rs::init_logger();
/// ```
pub fn init_logger() {
    env_logger::init();
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
    fn test_version() {
        assert!(!VERSION.is_empty());
    }

    #[test]
    fn test_create_client() {
        let client = create_client("http://127.0.0.1:9090", None);
        assert!(client.is_ok());
    }
}