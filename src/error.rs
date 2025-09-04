//! 错误处理模块
//! 
//! 定义了 SDK 中使用的所有错误类型和结果类型。

use thiserror::Error;

/// SDK 的主要错误类型
#[derive(Error, Debug)]
pub enum MihomoError {
    /// HTTP 请求错误
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    /// JSON 序列化/反序列化错误
    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),

    /// YAML 序列化/反序列化错误
    #[error("YAML serialization error: {0}")]
    Yaml(#[from] serde_yaml::Error),

    /// URL 解析错误
    #[error("URL parse error: {0}")]
    UrlParse(#[from] url::ParseError),

    /// IP 地址解析错误
    #[error("IP address parse error: {0}")]
    AddrParse(#[from] std::net::AddrParseError),

    /// 配置错误
    #[error("Configuration error: {0}")]
    Config(String),

    /// 代理错误
    #[error("Proxy error: {0}")]
    Proxy(String),

    /// 规则引擎错误
    #[error("Rules engine error: {0}")]
    Rules(String),

    /// 认证错误
    #[error("Authentication error: {0}")]
    Auth(String),

    /// 网络连接错误
    #[error("Network connection error: {0}")]
    Network(String),

    /// 服务不可用错误
    #[error("Service unavailable: {0}")]
    ServiceUnavailable(String),

    /// 超时错误
    #[error("Operation timeout: {0}")]
    Timeout(String),

    /// 无效参数错误
    #[error("Invalid parameter: {0}")]
    InvalidParameter(String),

    /// 资源未找到错误
    #[error("Resource not found: {0}")]
    NotFound(String),

    /// 内部错误
    #[error("Internal error: {0}")]
    Internal(String),

    /// 服务错误
    #[error("服务错误: {0}")]
    ServiceError(String),

    /// 下载错误
    #[error("下载错误: {0}")]
    DownloadError(String),

    /// 版本未找到
    #[error("版本未找到: {0}")]
    VersionNotFound(String),

    /// 不支持的平台
    #[error("不支持的平台: {0}")]
    UnsupportedPlatform(String),

    /// 其他错误
    #[error("Other error: {0}")]
    Other(#[from] anyhow::Error),
}

/// SDK 的结果类型
pub type Result<T> = std::result::Result<T, MihomoError>;

impl MihomoError {
    /// 创建配置错误
    pub fn config<S: Into<String>>(msg: S) -> Self {
        MihomoError::Config(msg.into())
    }

    /// 创建代理错误
    pub fn proxy<S: Into<String>>(msg: S) -> Self {
        MihomoError::Proxy(msg.into())
    }

    /// 创建规则错误
    pub fn rules<S: Into<String>>(msg: S) -> Self {
        MihomoError::Rules(msg.into())
    }

    /// 创建认证错误
    pub fn auth<S: Into<String>>(msg: S) -> Self {
        MihomoError::Auth(msg.into())
    }

    /// 创建网络错误
    pub fn network<S: Into<String>>(msg: S) -> Self {
        MihomoError::Network(msg.into())
    }

    /// 创建超时错误
    pub fn timeout<S: Into<String>>(msg: S) -> Self {
        MihomoError::Timeout(msg.into())
    }

    /// 创建无效参数错误
    pub fn invalid_parameter<S: Into<String>>(msg: S) -> Self {
        MihomoError::InvalidParameter(msg.into())
    }

    /// 创建资源未找到错误
    pub fn not_found<S: Into<String>>(msg: S) -> Self {
        MihomoError::NotFound(msg.into())
    }

    /// 创建内部错误
    pub fn internal<S: Into<String>>(msg: S) -> Self {
        MihomoError::Internal(msg.into())
    }

    /// 创建服务错误
    pub fn service_error<S: Into<String>>(msg: S) -> Self {
        MihomoError::ServiceError(msg.into())
    }

    /// 创建下载错误
    pub fn download_error<S: Into<String>>(msg: S) -> Self {
        MihomoError::DownloadError(msg.into())
    }

    /// 创建版本未找到错误
    pub fn version_not_found<S: Into<String>>(msg: S) -> Self {
        MihomoError::VersionNotFound(msg.into())
    }

    /// 创建不支持的平台错误
    pub fn unsupported_platform<S: Into<String>>(msg: S) -> Self {
        MihomoError::UnsupportedPlatform(msg.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_creation() {
        let config_err = MihomoError::config("test config error");
        assert!(matches!(config_err, MihomoError::Config(_)));

        let proxy_err = MihomoError::proxy("test proxy error");
        assert!(matches!(proxy_err, MihomoError::Proxy(_)));
    }

    #[test]
    fn test_error_display() {
        let err = MihomoError::config("test error");
        let error_string = format!("{}", err);
        assert!(error_string.contains("Configuration error"));
        assert!(error_string.contains("test error"));
    }
}