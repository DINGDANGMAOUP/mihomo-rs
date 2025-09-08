//! 错误处理模块
//!
//! 定义了 SDK 中使用的所有错误类型和结果类型。

use thiserror::Error;

use crate::logger::Logger;
use serde::{Deserialize, Serialize};

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

    /// IO错误
    #[error("IO错误: {0}")]
    IoError(String),

    /// 其他错误
    #[error("Other error: {0}")]
    Other(#[from] anyhow::Error),
}

/// SDK 的结果类型
pub type Result<T> = std::result::Result<T, MihomoError>;

/// 错误分类
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ErrorCategory {
    /// 网络相关错误
    Network,
    /// 配置相关错误
    Configuration,
    /// 认证相关错误
    Authentication,
    /// 服务相关错误
    Service,
    /// 数据处理错误
    DataProcessing,
    /// 系统错误
    System,
    /// 用户输入错误
    UserInput,
    /// 内部错误
    Internal,
}

/// 错误上下文信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorContext {
    /// 错误发生的操作
    pub operation: String,
    /// 错误发生的组件
    pub component: String,
    /// 相关的资源ID
    pub resource_id: Option<String>,
    /// 额外的上下文数据
    pub metadata: std::collections::HashMap<String, String>,
}

/// 增强的错误信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorInfo {
    /// 错误分类
    pub category: ErrorCategory,
    /// 错误代码
    pub code: String,
    /// 错误消息
    pub message: String,
    /// 错误上下文
    pub context: Option<ErrorContext>,
    /// 是否可重试
    pub retryable: bool,
    /// 建议的解决方案
    pub suggestion: Option<String>,
}

impl MihomoError {
    /// 创建配置错误
    pub fn config<S: Into<String>>(msg: S) -> Self {
        let error = MihomoError::Config(msg.into());
        error.log_error();
        error
    }

    /// 创建JSON错误
    pub fn json<S: Into<String>>(msg: S) -> Self {
        let error = MihomoError::Internal(msg.into());
        error.log_error();
        error
    }

    /// 创建无效参数错误
    pub fn invalid_parameter<S: Into<String>>(msg: S) -> Self {
        let error = MihomoError::InvalidParameter(msg.into());
        error.log_error();
        error
    }

    /// 创建网络错误
    pub fn network<S: Into<String>>(msg: S) -> Self {
        let error = MihomoError::Network(msg.into());
        error.log_error();
        error
    }

    /// 创建认证错误
    pub fn auth<S: Into<String>>(msg: S) -> Self {
        let error = MihomoError::Auth(msg.into());
        error.log_error();
        error
    }

    /// 创建未找到错误
    pub fn not_found<S: Into<String>>(msg: S) -> Self {
        let error = MihomoError::NotFound(msg.into());
        error.log_error();
        error
    }

    /// 创建内部错误
    pub fn internal<S: Into<String>>(msg: S) -> Self {
        let error = MihomoError::Internal(msg.into());
        error.log_error();
        error
    }

    /// 创建代理错误
    pub fn proxy<S: Into<String>>(msg: S) -> Self {
        let error = MihomoError::Proxy(msg.into());
        error.log_error();
        error
    }

    /// 创建规则错误
    pub fn rules<S: Into<String>>(msg: S) -> Self {
        let error = MihomoError::Rules(msg.into());
        error.log_error();
        error
    }

    /// 创建超时错误
    pub fn timeout<S: Into<String>>(msg: S) -> Self {
        let error = MihomoError::Timeout(msg.into());
        error.log_error();
        error
    }

    /// 创建服务不可用错误
    pub fn service_unavailable<S: Into<String>>(msg: S) -> Self {
        let error = MihomoError::ServiceUnavailable(msg.into());
        error.log_error();
        error
    }

    /// 创建服务错误
    pub fn service_error<S: Into<String>>(msg: S) -> Self {
        let error = MihomoError::ServiceError(msg.into());
        error.log_error();
        error
    }

    /// 创建下载错误
    pub fn download_error<S: Into<String>>(msg: S) -> Self {
        let error = MihomoError::DownloadError(msg.into());
        error.log_error();
        error
    }

    /// 创建IO错误
    pub fn io_error<S: Into<String>>(msg: S) -> Self {
        let error = MihomoError::IoError(msg.into());
        error.log_error();
        error
    }

    /// 创建版本未找到错误
    pub fn version_not_found<S: Into<String>>(msg: S) -> Self {
        let error = MihomoError::VersionNotFound(msg.into());
        error.log_error();
        error
    }

    /// 创建不支持的平台错误
    pub fn unsupported_platform<S: Into<String>>(msg: S) -> Self {
        let error = MihomoError::UnsupportedPlatform(msg.into());
        error.log_error();
        error
    }
}

/// 手动实现Clone trait
impl Clone for MihomoError {
    fn clone(&self) -> Self {
        match self {
            MihomoError::Http(_) => MihomoError::Internal("HTTP error".to_string()),
            MihomoError::Json(_) => MihomoError::Internal("JSON error".to_string()),
            MihomoError::Yaml(_) => MihomoError::Internal("YAML error".to_string()),
            MihomoError::UrlParse(_) => MihomoError::Internal("URL parse error".to_string()),
            MihomoError::AddrParse(_) => MihomoError::Internal("Address parse error".to_string()),
            MihomoError::Config(s) => MihomoError::Config(s.clone()),
            MihomoError::Proxy(s) => MihomoError::Proxy(s.clone()),
            MihomoError::Rules(s) => MihomoError::Rules(s.clone()),
            MihomoError::Auth(s) => MihomoError::Auth(s.clone()),
            MihomoError::Network(s) => MihomoError::Network(s.clone()),
            MihomoError::ServiceUnavailable(s) => MihomoError::ServiceUnavailable(s.clone()),
            MihomoError::Timeout(s) => MihomoError::Timeout(s.clone()),
            MihomoError::InvalidParameter(s) => MihomoError::InvalidParameter(s.clone()),
            MihomoError::NotFound(s) => MihomoError::NotFound(s.clone()),
            MihomoError::Internal(s) => MihomoError::Internal(s.clone()),
            MihomoError::ServiceError(s) => MihomoError::ServiceError(s.clone()),
            MihomoError::DownloadError(s) => MihomoError::DownloadError(s.clone()),
            MihomoError::VersionNotFound(s) => MihomoError::VersionNotFound(s.clone()),
            MihomoError::UnsupportedPlatform(s) => MihomoError::UnsupportedPlatform(s.clone()),
            MihomoError::IoError(s) => MihomoError::IoError(s.clone()),
            MihomoError::Other(_) => MihomoError::Internal("Other error".to_string()),
        }
    }
}

impl MihomoError {
    /// 获取错误分类
    pub fn category(&self) -> ErrorCategory {
        match self {
            MihomoError::Http(_) | MihomoError::Network(_) | MihomoError::Timeout(_) => {
                ErrorCategory::Network
            }
            MihomoError::Config(_) => ErrorCategory::Configuration,
            MihomoError::Auth(_) => ErrorCategory::Authentication,
            MihomoError::ServiceError(_) | MihomoError::ServiceUnavailable(_) => {
                ErrorCategory::Service
            }
            MihomoError::Json(_) | MihomoError::Yaml(_) => ErrorCategory::DataProcessing,
            MihomoError::UrlParse(_)
            | MihomoError::AddrParse(_)
            | MihomoError::InvalidParameter(_) => ErrorCategory::UserInput,
            MihomoError::IoError(_) => ErrorCategory::System,
            _ => ErrorCategory::Internal,
        }
    }

    /// 获取错误代码
    pub fn code(&self) -> String {
        match self {
            MihomoError::Http(_) => "HTTP_ERROR".to_string(),
            MihomoError::Json(_) => "JSON_ERROR".to_string(),
            MihomoError::Yaml(_) => "YAML_ERROR".to_string(),
            MihomoError::UrlParse(_) => "URL_PARSE_ERROR".to_string(),
            MihomoError::AddrParse(_) => "ADDR_PARSE_ERROR".to_string(),
            MihomoError::Config(_) => "CONFIG_ERROR".to_string(),
            MihomoError::Proxy(_) => "PROXY_ERROR".to_string(),
            MihomoError::Rules(_) => "RULES_ERROR".to_string(),
            MihomoError::Auth(_) => "AUTH_ERROR".to_string(),
            MihomoError::Network(_) => "NETWORK_ERROR".to_string(),
            MihomoError::ServiceUnavailable(_) => "SERVICE_UNAVAILABLE".to_string(),
            MihomoError::Timeout(_) => "TIMEOUT_ERROR".to_string(),
            MihomoError::InvalidParameter(_) => "INVALID_PARAMETER".to_string(),
            MihomoError::NotFound(_) => "NOT_FOUND".to_string(),
            MihomoError::Internal(_) => "INTERNAL_ERROR".to_string(),
            MihomoError::ServiceError(_) => "SERVICE_ERROR".to_string(),
            MihomoError::DownloadError(_) => "DOWNLOAD_ERROR".to_string(),
            MihomoError::VersionNotFound(_) => "VERSION_NOT_FOUND".to_string(),
            MihomoError::UnsupportedPlatform(_) => "UNSUPPORTED_PLATFORM".to_string(),
            MihomoError::IoError(_) => "IO_ERROR".to_string(),
            MihomoError::Other(_) => "OTHER_ERROR".to_string(),
        }
    }

    /// 判断错误是否可重试
    pub fn is_retryable(&self) -> bool {
        match self {
            MihomoError::Http(_)
            | MihomoError::Network(_)
            | MihomoError::Timeout(_)
            | MihomoError::ServiceUnavailable(_) => true,
            MihomoError::Auth(_) | MihomoError::InvalidParameter(_) | MihomoError::NotFound(_) => {
                false
            }
            _ => false,
        }
    }

    /// 获取建议的解决方案
    pub fn suggestion(&self) -> Option<String> {
        match self {
            MihomoError::Network(_) => Some("请检查网络连接和服务器状态".to_string()),
            MihomoError::Auth(_) => Some("请检查API密钥是否正确".to_string()),
            MihomoError::Config(_) => Some("请检查配置文件格式和内容".to_string()),
            MihomoError::Timeout(_) => Some("请尝试增加超时时间或检查网络延迟".to_string()),
            MihomoError::ServiceUnavailable(_) => Some("请检查服务是否正在运行".to_string()),
            MihomoError::InvalidParameter(_) => Some("请检查输入参数的格式和有效性".to_string()),
            _ => None,
        }
    }

    /// 创建带上下文的错误信息
    pub fn with_context(self, operation: &str, component: &str) -> ErrorInfo {
        let context = ErrorContext {
            operation: operation.to_string(),
            component: component.to_string(),
            resource_id: None,
            metadata: std::collections::HashMap::new(),
        };

        ErrorInfo {
            category: self.category(),
            code: self.code(),
            message: self.to_string(),
            context: Some(context),
            retryable: self.is_retryable(),
            suggestion: self.suggestion(),
        }
    }

    /// 记录错误日志
    fn log_error(&self) {
        Logger::error(&format!("[{}] {}", self.code(), self));
    }

    /// 转换为错误信息
    pub fn to_error_info(&self) -> ErrorInfo {
        ErrorInfo {
            category: self.category(),
            code: self.code(),
            message: self.to_string(),
            context: None,
            retryable: self.is_retryable(),
            suggestion: self.suggestion(),
        }
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
