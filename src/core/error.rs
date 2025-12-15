use thiserror::Error;

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
    Config(String),

    #[error("Service error: {0}")]
    Service(String),

    #[error("Version error: {0}")]
    Version(String),

    #[error("Proxy error: {0}")]
    Proxy(String),

    #[error("Not found: {0}")]
    NotFound(String),
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
        let err = MihomoError::Config("invalid configuration".to_string());
        assert_eq!(err.to_string(), "Config error: invalid configuration");
    }

    #[test]
    fn test_service_error_display() {
        let err = MihomoError::Service("failed to start".to_string());
        assert_eq!(err.to_string(), "Service error: failed to start");
    }

    #[test]
    fn test_version_error_display() {
        let err = MihomoError::Version("version not found".to_string());
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
            Err(MihomoError::Config("test error".to_string()))
        }

        assert_eq!(returns_ok().unwrap(), 42);
        assert!(returns_err().is_err());
    }
}
