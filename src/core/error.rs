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
    WebSocket(#[from] tokio_tungstenite::tungstenite::Error),

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

pub type Result<T> = std::result::Result<T, MihomoError>;
