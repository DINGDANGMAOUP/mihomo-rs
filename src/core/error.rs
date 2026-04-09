use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCode {
    InvalidExternalController,
    InvalidProfileName,
    InvalidVersion,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ErrorDetail {
    pub code: Option<ErrorCode>,
    pub message: String,
}

impl ErrorDetail {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            code: None,
            message: message.into(),
        }
    }

    pub fn with_code(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code: Some(code),
            message: message.into(),
        }
    }

    pub fn as_str(&self) -> &str {
        &self.message
    }
}

impl std::fmt::Display for ErrorDetail {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl From<String> for ErrorDetail {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl From<&str> for ErrorDetail {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl std::fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let code = match self {
            ErrorCode::InvalidExternalController => "E_CFG_INVALID_EXTERNAL_CONTROLLER",
            ErrorCode::InvalidProfileName => "E_CFG_INVALID_PROFILE_NAME",
            ErrorCode::InvalidVersion => "E_VER_INVALID_VERSION",
        };
        f.write_str(code)
    }
}

impl std::str::FromStr for ErrorCode {
    type Err = ();

    fn from_str(code: &str) -> std::result::Result<Self, Self::Err> {
        match code {
            "E_CFG_INVALID_EXTERNAL_CONTROLLER" => Ok(ErrorCode::InvalidExternalController),
            "E_CFG_INVALID_PROFILE_NAME" => Ok(ErrorCode::InvalidProfileName),
            "E_VER_INVALID_VERSION" => Ok(ErrorCode::InvalidVersion),
            _ => Err(()),
        }
    }
}

#[derive(Error, Debug)]
pub enum MihomoError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("YAML error: {0}")]
    Yaml(#[from] serde_yaml::Error),

    #[error("URL parse error: {0}")]
    UrlParse(#[from] url::ParseError),

    #[error("WebSocket error: {0}")]
    WebSocket(Box<tokio_tungstenite::tungstenite::Error>),

    #[error("Config error: {0}")]
    Config(ErrorDetail),

    #[error("Service error: {0}")]
    Service(String),

    #[error("Version error: {0}")]
    Version(ErrorDetail),

    #[error("Proxy error: {0}")]
    Proxy(String),

    #[error("Not found: {0}")]
    NotFound(String),
}

impl MihomoError {
    pub fn config(message: impl Into<String>) -> Self {
        Self::Config(ErrorDetail::new(message))
    }

    pub fn config_with_code(code: ErrorCode, message: impl Into<String>) -> Self {
        Self::Config(ErrorDetail::with_code(code, message))
    }

    pub fn version(message: impl Into<String>) -> Self {
        Self::Version(ErrorDetail::new(message))
    }

    pub fn version_with_code(code: ErrorCode, message: impl Into<String>) -> Self {
        Self::Version(ErrorDetail::with_code(code, message))
    }
}

// Manual From implementation for WebSocket error to box it
impl From<tokio_tungstenite::tungstenite::Error> for MihomoError {
    fn from(err: tokio_tungstenite::tungstenite::Error) -> Self {
        MihomoError::WebSocket(Box::new(err))
    }
}

pub type Result<T> = std::result::Result<T, MihomoError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_error_display() {
        let err = MihomoError::config("invalid configuration");
        assert_eq!(err.to_string(), "Config error: invalid configuration");
    }

    #[test]
    fn test_service_error_display() {
        let err = MihomoError::Service("failed to start".to_string());
        assert_eq!(err.to_string(), "Service error: failed to start");
    }

    #[test]
    fn test_version_error_display() {
        let err = MihomoError::version("version not found");
        assert_eq!(err.to_string(), "Version error: version not found");
    }

    #[test]
    fn test_proxy_error_display() {
        let err = MihomoError::Proxy("proxy unavailable".to_string());
        assert_eq!(err.to_string(), "Proxy error: proxy unavailable");
    }

    #[test]
    fn test_not_found_error_display() {
        let err = MihomoError::NotFound("resource not found".to_string());
        assert_eq!(err.to_string(), "Not found: resource not found");
    }

    #[test]
    fn test_io_error_conversion() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let mihomo_err: MihomoError = io_err.into();
        assert!(matches!(mihomo_err, MihomoError::Io(_)));
    }

    #[test]
    fn test_json_error_conversion() {
        let json_err = serde_json::from_str::<serde_json::Value>("invalid json").unwrap_err();
        let mihomo_err: MihomoError = json_err.into();
        assert!(matches!(mihomo_err, MihomoError::Json(_)));
    }

    #[test]
    fn test_url_parse_error_conversion() {
        let url_err = url::Url::parse("not a url").unwrap_err();
        let mihomo_err: MihomoError = url_err.into();
        assert!(matches!(mihomo_err, MihomoError::UrlParse(_)));
    }

    #[test]
    fn test_result_type() {
        fn returns_ok() -> Result<i32> {
            Ok(42)
        }

        fn returns_err() -> Result<i32> {
            Err(MihomoError::config("test error"))
        }

        assert_eq!(returns_ok().unwrap(), 42);
        assert!(returns_err().is_err());
    }

    #[test]
    fn test_websocket_error_conversion() {
        let ws_err = tokio_tungstenite::tungstenite::Error::ConnectionClosed;
        let mihomo_err: MihomoError = ws_err.into();
        assert!(matches!(mihomo_err, MihomoError::WebSocket(_)));
    }

    #[test]
    fn test_error_code_display_and_from_str() {
        use std::str::FromStr;

        assert_eq!(
            ErrorCode::InvalidExternalController.to_string(),
            "E_CFG_INVALID_EXTERNAL_CONTROLLER"
        );
        assert_eq!(
            ErrorCode::from_str("E_CFG_INVALID_PROFILE_NAME").ok(),
            Some(ErrorCode::InvalidProfileName)
        );
        assert!(ErrorCode::from_str("UNKNOWN").is_err());
    }
}
