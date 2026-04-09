use super::error::Result;
use super::types::*;
use futures_util::StreamExt;
use percent_encoding::{utf8_percent_encode, AsciiSet, CONTROLS};
use reqwest::Client;
use serde_json::json;
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use url::Url;

const PATH_SEGMENT_ENCODE_SET: &AsciiSet = &CONTROLS
    .add(b' ')
    .add(b'"')
    .add(b'#')
    .add(b'%')
    .add(b'/')
    .add(b'?')
    .add(b'[')
    .add(b'\\')
    .add(b']')
    .add(b'^')
    .add(b'`')
    .add(b'{')
    .add(b'|')
    .add(b'}');

#[derive(Clone)]
enum Transport {
    Tcp { client: Client, base_url: Url },
    Unix { socket_path: PathBuf },
}

#[derive(Clone)]
pub struct MihomoClient {
    transport: Transport,
    secret: Option<String>,
    ws_connect_timeout: Duration,
}

impl MihomoClient {
    pub fn new(base_url: &str, secret: Option<String>) -> Result<Self> {
        let transport = if base_url.starts_with('/')
            || base_url.starts_with("unix://")
            || base_url.starts_with(r"\\")
        {
            let path = base_url.strip_prefix("unix://").unwrap_or(base_url);
            Transport::Unix {
                socket_path: PathBuf::from(path),
            }
        } else {
            let url = Url::parse(base_url)?;
            Transport::Tcp {
                client: Client::new(),
                base_url: url,
            }
        };

        Ok(Self {
            transport,
            secret,
            ws_connect_timeout: Duration::from_secs(10),
        })
    }

    async fn http_request(
        &self,
        method: &str,
        path: &str,
        query: Option<&[(&str, String)]>,
        body: Option<serde_json::Value>,
    ) -> Result<Vec<u8>> {
        match &self.transport {
            Transport::Tcp { client, base_url } => {
                let url = base_url.join(path)?;
                let mut req = match method {
                    "GET" => client.get(url),
                    "PUT" => client.put(url),
                    "DELETE" => client.delete(url),
                    _ => {
                        return Err(super::error::MihomoError::Config(
                            "Unsupported method".into(),
                        ))
                    }
                };

                if let Some(q) = query {
                    req = req.query(q);
                }
                if let Some(b) = body {
                    req = req.json(&b);
                }
                req = self.add_auth(req);

                let resp = req.send().await?.error_for_status()?;
                Ok(resp.bytes().await?.to_vec())
            }
            Transport::Unix { socket_path } => {
                self.unix_http_request(method, path, query, body, socket_path)
                    .await
            }
        }
    }

    async fn unix_http_request(
        &self,
        method: &str,
        path: &str,
        query: Option<&[(&str, String)]>,
        body: Option<serde_json::Value>,
        socket_path: &PathBuf,
    ) -> Result<Vec<u8>> {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        #[cfg(unix)]
        {
            use tokio::net::UnixStream;
            let mut stream = UnixStream::connect(socket_path).await?;

            let query_str = query
                .map(|q| {
                    let mut serializer = url::form_urlencoded::Serializer::new(String::new());
                    for (k, v) in q {
                        serializer.append_pair(k, v);
                    }
                    format!("?{}", serializer.finish())
                })
                .unwrap_or_default();

            let body_str = match body {
                Some(b) => serde_json::to_string(&b)?,
                None => String::new(),
            };

            let auth_header = self
                .secret
                .as_ref()
                .map(|s| format!("Authorization: Bearer {}\r\n", s))
                .unwrap_or_default();

            let request = format!(
                "{} {}{} HTTP/1.1\r\n\
                 Host: localhost\r\n\
                 Content-Length: {}\r\n\
                 Content-Type: application/json\r\n\
                 {}\r\n\
                 {}",
                method,
                path,
                query_str,
                body_str.len(),
                auth_header,
                body_str
            );

            stream.write_all(request.as_bytes()).await?;
            stream.flush().await?;

            let mut response = Vec::new();
            stream.read_to_end(&mut response).await?;

            let response_str = String::from_utf8_lossy(&response);
            let status_line = response_str.lines().next().unwrap_or_default().to_string();
            if let Some(pos) = response_str.find("\r\n\r\n") {
                let body_bytes = response[pos + 4..].to_vec();
                if let Some(code_str) = status_line.split_whitespace().nth(1) {
                    if let Ok(code) = code_str.parse::<u16>() {
                        if code >= 400 {
                            return Err(super::error::MihomoError::Service(format!(
                                "HTTP error {}: {}",
                                code,
                                String::from_utf8_lossy(&body_bytes)
                            )));
                        }
                    }
                }
                Ok(body_bytes)
            } else {
                Err(super::error::MihomoError::Config(
                    "Invalid HTTP response".into(),
                ))
            }
        }
        #[cfg(windows)]
        {
            use tokio::net::windows::named_pipe::ClientOptions;

            let pipe_path = socket_path.to_string_lossy();
            let pipe_name = if pipe_path.starts_with("\\\\.\\pipe\\") {
                pipe_path.to_string()
            } else {
                format!("\\\\.\\pipe\\{}", pipe_path.trim_start_matches('/'))
            };

            let mut stream = ClientOptions::new().open(&pipe_name)?;

            let query_str = query
                .map(|q| {
                    let mut serializer = url::form_urlencoded::Serializer::new(String::new());
                    for (k, v) in q {
                        serializer.append_pair(k, v);
                    }
                    format!("?{}", serializer.finish())
                })
                .unwrap_or_default();

            let body_str = match body {
                Some(b) => serde_json::to_string(&b)?,
                None => String::new(),
            };

            let auth_header = self
                .secret
                .as_ref()
                .map(|s| format!("Authorization: Bearer {}\r\n", s))
                .unwrap_or_default();

            let request = format!(
                "{} {}{} HTTP/1.1\r\n\
                 Host: localhost\r\n\
                 Content-Length: {}\r\n\
                 Content-Type: application/json\r\n\
                 {}\r\n\
                 {}",
                method,
                path,
                query_str,
                body_str.len(),
                auth_header,
                body_str
            );

            stream.write_all(request.as_bytes()).await?;
            stream.flush().await?;

            let mut response = Vec::new();
            stream.read_to_end(&mut response).await?;

            let response_str = String::from_utf8_lossy(&response);
            let status_line = response_str.lines().next().unwrap_or_default().to_string();
            if let Some(pos) = response_str.find("\r\n\r\n") {
                let body_bytes = response[pos + 4..].to_vec();
                if let Some(code_str) = status_line.split_whitespace().nth(1) {
                    if let Ok(code) = code_str.parse::<u16>() {
                        if code >= 400 {
                            return Err(super::error::MihomoError::Service(format!(
                                "HTTP error {}: {}",
                                code,
                                String::from_utf8_lossy(&body_bytes)
                            )));
                        }
                    }
                }
                Ok(body_bytes)
            } else {
                Err(super::error::MihomoError::Config(
                    "Invalid HTTP response".into(),
                ))
            }
        }
        #[cfg(not(any(unix, windows)))]
        {
            let _ = (method, path, query, body, socket_path);
            Err(super::error::MihomoError::Config(
                "Unix domain sockets are not supported on this platform".into(),
            ))
        }
    }

    fn add_auth(&self, mut req: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        if let Some(secret) = &self.secret {
            req = req.bearer_auth(secret);
        }
        req
    }

    fn encode_path_segment(input: &str) -> String {
        utf8_percent_encode(input, PATH_SEGMENT_ENCODE_SET).to_string()
    }

    pub fn with_ws_connect_timeout(mut self, timeout: Duration) -> Self {
        self.ws_connect_timeout = timeout.max(Duration::from_millis(1));
        self
    }

    fn ws_request_with_auth(
        url: &str,
        secret: Option<&str>,
    ) -> std::result::Result<
        tokio_tungstenite::tungstenite::handshake::client::Request,
        tokio_tungstenite::tungstenite::Error,
    > {
        let mut request = url.into_client_request()?;
        if let Some(secret) = secret {
            let header = format!("Bearer {}", secret);
            if let Ok(value) = header.parse() {
                request.headers_mut().insert("Authorization", value);
            }
        }
        Ok(request)
    }

    fn ws_timeout_error(endpoint: &str) -> super::error::MihomoError {
        super::error::MihomoError::Service(format!("WebSocket connection timeout: {}", endpoint))
    }

    pub async fn get_version(&self) -> Result<Version> {
        let response = self.http_request("GET", "/version", None, None).await?;
        Ok(serde_json::from_slice(&response)?)
    }

    pub async fn get_proxies(&self) -> Result<HashMap<String, ProxyInfo>> {
        log::debug!("Fetching proxies");
        let response = self.http_request("GET", "/proxies", None, None).await?;
        let data: ProxiesResponse = serde_json::from_slice(&response)?;
        log::debug!("Received {} proxies", data.proxies.len());
        Ok(data.proxies)
    }

    pub async fn get_proxy(&self, name: &str) -> Result<ProxyInfo> {
        let encoded_name = Self::encode_path_segment(name);
        let response = self
            .http_request("GET", &format!("/proxies/{}", encoded_name), None, None)
            .await?;
        Ok(serde_json::from_slice(&response)?)
    }

    pub async fn switch_proxy(&self, group: &str, proxy: &str) -> Result<()> {
        let encoded_group = Self::encode_path_segment(group);
        log::debug!("Switching group '{}' to proxy '{}'", group, proxy);
        self.http_request(
            "PUT",
            &format!("/proxies/{}", encoded_group),
            None,
            Some(json!({ "name": proxy })),
        )
        .await?;
        log::debug!("Successfully switched group '{}' to '{}'", group, proxy);
        Ok(())
    }

    pub async fn test_delay(&self, proxy: &str, test_url: &str, timeout: u32) -> Result<u32> {
        let encoded_proxy = Self::encode_path_segment(proxy);
        let response = self
            .http_request(
                "GET",
                &format!("/proxies/{}/delay", encoded_proxy),
                Some(&[
                    ("timeout", timeout.to_string()),
                    ("url", test_url.to_string()),
                ]),
                None,
            )
            .await?;
        let data: DelayTestResponse = serde_json::from_slice(&response)?;
        Ok(data.delay)
    }

    pub async fn reload_config(&self, path: Option<&str>) -> Result<()> {
        let (query, body) = if let Some(p) = path {
            (
                Some(vec![("force", "true".to_string())]),
                Some(json!({ "path": p })),
            )
        } else {
            (Some(vec![("force", "true".to_string())]), None)
        };

        self.http_request("PUT", "/configs", query.as_deref(), body)
            .await?;
        Ok(())
    }

    pub async fn stream_logs(
        &self,
        level: Option<&str>,
    ) -> Result<tokio::sync::mpsc::UnboundedReceiver<String>> {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();

        match &self.transport {
            Transport::Tcp { base_url, .. } => {
                let mut ws_url = base_url.clone();
                ws_url
                    .set_scheme(if ws_url.scheme() == "https" {
                        "wss"
                    } else {
                        "ws"
                    })
                    .ok();
                ws_url.set_path("/logs");
                if let Some(level) = level {
                    let mut serializer = url::form_urlencoded::Serializer::new(String::new());
                    serializer.append_pair("level", level);
                    ws_url.set_query(Some(&serializer.finish()));
                }

                let ws_url_str = ws_url.to_string();
                let request =
                    MihomoClient::ws_request_with_auth(&ws_url_str, self.secret.as_deref())?;
                let (ws_stream, _) =
                    tokio::time::timeout(self.ws_connect_timeout, connect_async(request))
                        .await
                        .map_err(|_| Self::ws_timeout_error("logs"))??;
                tokio::spawn(async move {
                    let (_, mut read) = ws_stream.split();
                    while let Some(msg) = read.next().await {
                        match msg {
                            Ok(Message::Text(text)) => {
                                if tx.send(text.to_string()).is_err() {
                                    break;
                                }
                            }
                            Ok(Message::Close(_)) => break,
                            Err(_) => break,
                            _ => {}
                        }
                    }
                });
            }
            Transport::Unix { socket_path } => {
                let socket_path = socket_path.clone();
                let level = level.map(|s| s.to_string());
                let secret = self.secret.clone();

                #[cfg(unix)]
                {
                    use tokio::net::UnixStream;
                    use tokio_tungstenite::client_async;

                    let stream = tokio::time::timeout(
                        self.ws_connect_timeout,
                        UnixStream::connect(&socket_path),
                    )
                    .await
                    .map_err(|_| Self::ws_timeout_error("logs"))??;
                    let mut path = "/logs".to_string();
                    if let Some(l) = level {
                        let mut serializer = url::form_urlencoded::Serializer::new(String::new());
                        serializer.append_pair("level", &l);
                        path.push('?');
                        path.push_str(&serializer.finish());
                    }

                    let auth_header = secret
                        .as_ref()
                        .map(|s| format!("Authorization: Bearer {}\r\n", s))
                        .unwrap_or_default();
                    let request = format!(
                        "GET {} HTTP/1.1\r\n\
                         Host: localhost\r\n\
                         Upgrade: websocket\r\n\
                         Connection: Upgrade\r\n\
                         Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\n\
                         Sec-WebSocket-Version: 13\r\n\
                         {}\r\n",
                        path, auth_header
                    );

                    let (ws_stream, _) = tokio::time::timeout(
                        self.ws_connect_timeout,
                        client_async(request, stream),
                    )
                    .await
                    .map_err(|_| Self::ws_timeout_error("logs"))??;
                    tokio::spawn(async move {
                        let (_, mut read) = ws_stream.split();
                        while let Some(msg) = read.next().await {
                            match msg {
                                Ok(Message::Text(text)) => {
                                    if tx.send(text.to_string()).is_err() {
                                        break;
                                    }
                                }
                                Ok(Message::Close(_)) => break,
                                Err(_) => break,
                                _ => {}
                            }
                        }
                    });
                }
                #[cfg(windows)]
                {
                    use tokio::net::windows::named_pipe::ClientOptions;
                    use tokio_tungstenite::client_async;

                    let pipe_path = socket_path.to_string_lossy();
                    let pipe_name = if pipe_path.starts_with("\\\\.\\pipe\\") {
                        pipe_path.to_string()
                    } else {
                        format!("\\\\.\\pipe\\{}", pipe_path.trim_start_matches('/'))
                    };

                    let pipe_name_for_open = pipe_name.clone();
                    let stream = tokio::time::timeout(
                        self.ws_connect_timeout,
                        tokio::task::spawn_blocking(move || {
                            ClientOptions::new().open(&pipe_name_for_open)
                        }),
                    )
                    .await
                    .map_err(|_| Self::ws_timeout_error("logs"))?
                    .map_err(|e| {
                        super::error::MihomoError::Service(format!(
                            "Failed to join named pipe connect task: {}",
                            e
                        ))
                    })??;
                    let mut path = "/logs".to_string();
                    if let Some(l) = level {
                        let mut serializer = url::form_urlencoded::Serializer::new(String::new());
                        serializer.append_pair("level", &l);
                        path.push('?');
                        path.push_str(&serializer.finish());
                    }

                    let auth_header = secret
                        .as_ref()
                        .map(|s| format!("Authorization: Bearer {}\r\n", s))
                        .unwrap_or_default();
                    let request = format!(
                        "GET {} HTTP/1.1\r\n\
                         Host: localhost\r\n\
                         Upgrade: websocket\r\n\
                         Connection: Upgrade\r\n\
                         Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\n\
                         Sec-WebSocket-Version: 13\r\n\
                         {}\r\n",
                        path, auth_header
                    );

                    let (ws_stream, _) = tokio::time::timeout(
                        self.ws_connect_timeout,
                        client_async(request, stream),
                    )
                    .await
                    .map_err(|_| Self::ws_timeout_error("logs"))??;
                    tokio::spawn(async move {
                        let (_, mut read) = ws_stream.split();
                        while let Some(msg) = read.next().await {
                            match msg {
                                Ok(Message::Text(text)) => {
                                    if tx.send(text.to_string()).is_err() {
                                        break;
                                    }
                                }
                                Ok(Message::Close(_)) => break,
                                Err(_) => break,
                                _ => {}
                            }
                        }
                    });
                }
            }
        }

        Ok(rx)
    }

    pub async fn stream_traffic(
        &self,
    ) -> Result<tokio::sync::mpsc::UnboundedReceiver<TrafficData>> {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();

        match &self.transport {
            Transport::Tcp { base_url, .. } => {
                let mut ws_url = base_url.clone();
                ws_url
                    .set_scheme(if ws_url.scheme() == "https" {
                        "wss"
                    } else {
                        "ws"
                    })
                    .ok();
                ws_url.set_path("/traffic");

                let ws_url_str = ws_url.to_string();
                let request =
                    MihomoClient::ws_request_with_auth(&ws_url_str, self.secret.as_deref())?;
                let (ws_stream, _) =
                    tokio::time::timeout(self.ws_connect_timeout, connect_async(request))
                        .await
                        .map_err(|_| Self::ws_timeout_error("traffic"))??;
                tokio::spawn(async move {
                    let (_, mut read) = ws_stream.split();
                    while let Some(msg) = read.next().await {
                        match msg {
                            Ok(Message::Text(text)) => {
                                if let Ok(traffic) =
                                    serde_json::from_str::<TrafficData>(text.as_ref())
                                {
                                    if tx.send(traffic).is_err() {
                                        break;
                                    }
                                }
                            }
                            Ok(Message::Close(_)) => break,
                            Err(_) => break,
                            _ => {}
                        }
                    }
                });
            }
            Transport::Unix { socket_path } => {
                let socket_path = socket_path.clone();
                let secret = self.secret.clone();

                #[cfg(unix)]
                {
                    use tokio::net::UnixStream;
                    use tokio_tungstenite::client_async;

                    let stream = tokio::time::timeout(
                        self.ws_connect_timeout,
                        UnixStream::connect(&socket_path),
                    )
                    .await
                    .map_err(|_| Self::ws_timeout_error("traffic"))??;
                    let auth_header = secret
                        .as_ref()
                        .map(|s| format!("Authorization: Bearer {}\r\n", s))
                        .unwrap_or_default();
                    let request = format!(
                        "GET /traffic HTTP/1.1\r\n\
                         Host: localhost\r\n\
                         Upgrade: websocket\r\n\
                         Connection: Upgrade\r\n\
                         Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\n\
                         Sec-WebSocket-Version: 13\r\n\
                         {}\r\n",
                        auth_header
                    );

                    let (ws_stream, _) = tokio::time::timeout(
                        self.ws_connect_timeout,
                        client_async(request, stream),
                    )
                    .await
                    .map_err(|_| Self::ws_timeout_error("traffic"))??;
                    tokio::spawn(async move {
                        let (_, mut read) = ws_stream.split();
                        while let Some(msg) = read.next().await {
                            match msg {
                                Ok(Message::Text(text)) => {
                                    if let Ok(traffic) =
                                        serde_json::from_str::<TrafficData>(text.as_ref())
                                    {
                                        if tx.send(traffic).is_err() {
                                            break;
                                        }
                                    }
                                }
                                Ok(Message::Close(_)) => break,
                                Err(_) => break,
                                _ => {}
                            }
                        }
                    });
                }
                #[cfg(windows)]
                {
                    use tokio::net::windows::named_pipe::ClientOptions;
                    use tokio_tungstenite::client_async;

                    let pipe_path = socket_path.to_string_lossy();
                    let pipe_name = if pipe_path.starts_with("\\\\.\\pipe\\") {
                        pipe_path.to_string()
                    } else {
                        format!("\\\\.\\pipe\\{}", pipe_path.trim_start_matches('/'))
                    };

                    let pipe_name_for_open = pipe_name.clone();
                    let stream = tokio::time::timeout(
                        self.ws_connect_timeout,
                        tokio::task::spawn_blocking(move || {
                            ClientOptions::new().open(&pipe_name_for_open)
                        }),
                    )
                    .await
                    .map_err(|_| Self::ws_timeout_error("traffic"))?
                    .map_err(|e| {
                        super::error::MihomoError::Service(format!(
                            "Failed to join named pipe connect task: {}",
                            e
                        ))
                    })??;
                    let auth_header = secret
                        .as_ref()
                        .map(|s| format!("Authorization: Bearer {}\r\n", s))
                        .unwrap_or_default();
                    let request = format!(
                        "GET /traffic HTTP/1.1\r\n\
                         Host: localhost\r\n\
                         Upgrade: websocket\r\n\
                         Connection: Upgrade\r\n\
                         Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\n\
                         Sec-WebSocket-Version: 13\r\n\
                         {}\r\n",
                        auth_header
                    );

                    let (ws_stream, _) = tokio::time::timeout(
                        self.ws_connect_timeout,
                        client_async(request, stream),
                    )
                    .await
                    .map_err(|_| Self::ws_timeout_error("traffic"))??;
                    tokio::spawn(async move {
                        let (_, mut read) = ws_stream.split();
                        while let Some(msg) = read.next().await {
                            match msg {
                                Ok(Message::Text(text)) => {
                                    if let Ok(traffic) =
                                        serde_json::from_str::<TrafficData>(text.as_ref())
                                    {
                                        if tx.send(traffic).is_err() {
                                            break;
                                        }
                                    }
                                }
                                Ok(Message::Close(_)) => break,
                                Err(_) => break,
                                _ => {}
                            }
                        }
                    });
                }
            }
        }

        Ok(rx)
    }

    pub async fn get_memory(&self) -> Result<MemoryData> {
        let response = self.http_request("GET", "/memory", None, None).await?;
        Ok(serde_json::from_slice(&response)?)
    }

    pub async fn get_connections(&self) -> Result<ConnectionsResponse> {
        log::debug!("Fetching connections");
        let response = self.http_request("GET", "/connections", None, None).await?;
        let data: ConnectionsResponse = serde_json::from_slice(&response)?;
        log::debug!("Received {} connections", data.connections.len());
        Ok(data)
    }

    pub async fn close_all_connections(&self) -> Result<()> {
        log::debug!("Closing all connections");
        self.http_request("DELETE", "/connections", None, None)
            .await?;
        log::debug!("Successfully closed all connections");
        Ok(())
    }

    pub async fn close_connection(&self, id: &str) -> Result<()> {
        let encoded_id = Self::encode_path_segment(id);
        log::debug!("Closing connection '{}'", id);
        self.http_request(
            "DELETE",
            &format!("/connections/{}", encoded_id),
            None,
            None,
        )
        .await?;
        log::debug!("Successfully closed connection '{}'", id);
        Ok(())
    }

    pub async fn stream_connections(
        &self,
    ) -> Result<tokio::sync::mpsc::UnboundedReceiver<ConnectionSnapshot>> {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();

        match &self.transport {
            Transport::Tcp { base_url, .. } => {
                let mut ws_url = base_url.clone();
                ws_url
                    .set_scheme(if ws_url.scheme() == "https" {
                        "wss"
                    } else {
                        "ws"
                    })
                    .ok();
                ws_url.set_path("/connections");

                let ws_url_str = ws_url.to_string();
                let request =
                    MihomoClient::ws_request_with_auth(&ws_url_str, self.secret.as_deref())?;
                let (ws_stream, _) =
                    tokio::time::timeout(self.ws_connect_timeout, connect_async(request))
                        .await
                        .map_err(|_| Self::ws_timeout_error("connections"))??;
                tokio::spawn(async move {
                    let (_, mut read) = ws_stream.split();
                    while let Some(msg) = read.next().await {
                        match msg {
                            Ok(Message::Text(text)) => {
                                if let Ok(snapshot) =
                                    serde_json::from_str::<ConnectionSnapshot>(text.as_ref())
                                {
                                    if tx.send(snapshot).is_err() {
                                        break;
                                    }
                                }
                            }
                            Ok(Message::Close(_)) => break,
                            Err(_) => break,
                            _ => {}
                        }
                    }
                });
            }
            Transport::Unix { socket_path } => {
                let socket_path = socket_path.clone();
                let secret = self.secret.clone();

                #[cfg(unix)]
                {
                    use tokio::net::UnixStream;
                    use tokio_tungstenite::client_async;

                    let stream = tokio::time::timeout(
                        self.ws_connect_timeout,
                        UnixStream::connect(&socket_path),
                    )
                    .await
                    .map_err(|_| Self::ws_timeout_error("connections"))??;
                    let auth_header = secret
                        .as_ref()
                        .map(|s| format!("Authorization: Bearer {}\r\n", s))
                        .unwrap_or_default();
                    let request = format!(
                        "GET /connections HTTP/1.1\r\n\
                         Host: localhost\r\n\
                         Upgrade: websocket\r\n\
                         Connection: Upgrade\r\n\
                         Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\n\
                         Sec-WebSocket-Version: 13\r\n\
                         {}\r\n",
                        auth_header
                    );

                    let (ws_stream, _) = tokio::time::timeout(
                        self.ws_connect_timeout,
                        client_async(request, stream),
                    )
                    .await
                    .map_err(|_| Self::ws_timeout_error("connections"))??;
                    tokio::spawn(async move {
                        let (_, mut read) = ws_stream.split();
                        while let Some(msg) = read.next().await {
                            match msg {
                                Ok(Message::Text(text)) => {
                                    if let Ok(snapshot) =
                                        serde_json::from_str::<ConnectionSnapshot>(text.as_ref())
                                    {
                                        if tx.send(snapshot).is_err() {
                                            break;
                                        }
                                    }
                                }
                                Ok(Message::Close(_)) => break,
                                Err(_) => break,
                                _ => {}
                            }
                        }
                    });
                }
                #[cfg(windows)]
                {
                    use tokio::net::windows::named_pipe::ClientOptions;
                    use tokio_tungstenite::client_async;

                    let pipe_path = socket_path.to_string_lossy();
                    let pipe_name = if pipe_path.starts_with("\\\\.\\pipe\\") {
                        pipe_path.to_string()
                    } else {
                        format!("\\\\.\\pipe\\{}", pipe_path.trim_start_matches('/'))
                    };

                    let pipe_name_for_open = pipe_name.clone();
                    let stream = tokio::time::timeout(
                        self.ws_connect_timeout,
                        tokio::task::spawn_blocking(move || {
                            ClientOptions::new().open(&pipe_name_for_open)
                        }),
                    )
                    .await
                    .map_err(|_| Self::ws_timeout_error("connections"))?
                    .map_err(|e| {
                        super::error::MihomoError::Service(format!(
                            "Failed to join named pipe connect task: {}",
                            e
                        ))
                    })??;
                    let auth_header = secret
                        .as_ref()
                        .map(|s| format!("Authorization: Bearer {}\r\n", s))
                        .unwrap_or_default();
                    let request = format!(
                        "GET /connections HTTP/1.1\r\n\
                         Host: localhost\r\n\
                         Upgrade: websocket\r\n\
                         Connection: Upgrade\r\n\
                         Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\n\
                         Sec-WebSocket-Version: 13\r\n\
                         {}\r\n",
                        auth_header
                    );

                    let (ws_stream, _) = tokio::time::timeout(
                        self.ws_connect_timeout,
                        client_async(request, stream),
                    )
                    .await
                    .map_err(|_| Self::ws_timeout_error("connections"))??;
                    tokio::spawn(async move {
                        let (_, mut read) = ws_stream.split();
                        while let Some(msg) = read.next().await {
                            match msg {
                                Ok(Message::Text(text)) => {
                                    if let Ok(snapshot) =
                                        serde_json::from_str::<ConnectionSnapshot>(text.as_ref())
                                    {
                                        if tx.send(snapshot).is_err() {
                                            break;
                                        }
                                    }
                                }
                                Ok(Message::Close(_)) => break,
                                Err(_) => break,
                                _ => {}
                            }
                        }
                    });
                }
            }
        }

        Ok(rx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::{Matcher, Server};
    use tokio::net::TcpListener;
    use tokio_tungstenite::{accept_async, tungstenite::Message as WsMessage};

    #[test]
    fn test_client_new() {
        let client = MihomoClient::new("http://127.0.0.1:9090", None);
        assert!(client.is_ok());
    }

    #[test]
    fn test_client_new_with_secret() {
        let client = MihomoClient::new("http://127.0.0.1:9090", Some("secret".to_string()));
        assert!(client.is_ok());
    }

    #[test]
    fn test_client_new_invalid_url() {
        let client = MihomoClient::new("not a url", None);
        assert!(client.is_err());
    }

    #[test]
    fn test_client_clone() {
        let client = MihomoClient::new("http://127.0.0.1:9090", None).unwrap();
        let _cloned = client.clone();
        // Just verify that cloning works without panicking
    }

    #[test]
    fn test_with_ws_connect_timeout() {
        use std::time::Duration;

        let client = MihomoClient::new("http://127.0.0.1:9090", None)
            .unwrap()
            .with_ws_connect_timeout(Duration::from_millis(250));
        assert_eq!(client.ws_connect_timeout, Duration::from_millis(250));
    }

    #[test]
    #[cfg(unix)]
    fn test_client_new_unix_socket() {
        let client = MihomoClient::new("/var/run/mihomo.sock", None);
        assert!(client.is_ok());
    }

    #[test]
    #[cfg(unix)]
    fn test_client_new_unix_socket_uri() {
        let client = MihomoClient::new("unix:///var/run/mihomo.sock", None);
        assert!(client.is_ok());
    }

    #[test]
    #[cfg(windows)]
    fn test_client_new_named_pipe() {
        let client = MihomoClient::new("/mihomo", None);
        assert!(client.is_ok());
    }

    #[test]
    #[cfg(windows)]
    fn test_client_new_named_pipe_full_path() {
        let client = MihomoClient::new("\\\\.\\pipe\\mihomo", None);
        assert!(client.is_ok());
    }

    #[tokio::test]
    async fn test_get_version() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("GET", "/version")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"version":"v1.18.0","premium":true,"meta":true}"#)
            .create_async()
            .await;

        let client = MihomoClient::new(&server.url(), None).unwrap();
        let result = client.get_version().await;

        mock.assert_async().await;
        assert!(result.is_ok());
        let version = result.unwrap();
        assert_eq!(version.version, "v1.18.0");
    }

    #[tokio::test]
    async fn test_get_version_returns_error_on_http_status_failure() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("GET", "/version")
            .with_status(500)
            .with_body("internal error")
            .create_async()
            .await;

        let client = MihomoClient::new(&server.url(), None).unwrap();
        let result = client.get_version().await;

        mock.assert_async().await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_proxies() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("GET", "/proxies")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"proxies":{"DIRECT":{"type":"Direct","udp":true,"now":"","all":[],"history":[]}}}"#)
            .create_async()
            .await;

        let client = MihomoClient::new(&server.url(), None).unwrap();
        let result = client.get_proxies().await;

        mock.assert_async().await;
        assert!(result.is_ok());
        let proxies = result.unwrap();
        assert!(proxies.contains_key("DIRECT"));
    }

    #[tokio::test]
    async fn test_get_proxy() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("GET", "/proxies/DIRECT")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"type":"Direct","udp":true,"now":"","all":[],"history":[]}"#)
            .create_async()
            .await;

        let client = MihomoClient::new(&server.url(), None).unwrap();
        let result = client.get_proxy("DIRECT").await;

        mock.assert_async().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_get_proxy_encodes_path_segment() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("GET", "/proxies/group%2Ftest")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"type":"Selector","now":"a","all":["a"],"history":[]}"#)
            .create_async()
            .await;

        let client = MihomoClient::new(&server.url(), None).unwrap();
        let result = client.get_proxy("group/test").await;

        mock.assert_async().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_switch_proxy() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("PUT", "/proxies/GLOBAL")
            .match_body(Matcher::Json(serde_json::json!({"name":"proxy1"})))
            .with_status(204)
            .create_async()
            .await;

        let client = MihomoClient::new(&server.url(), None).unwrap();
        let result = client.switch_proxy("GLOBAL", "proxy1").await;

        mock.assert_async().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_test_delay() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("GET", "/proxies/proxy1/delay")
            .match_query(Matcher::AllOf(vec![
                Matcher::UrlEncoded("timeout".into(), "5000".into()),
                Matcher::UrlEncoded("url".into(), "http://www.gstatic.com/generate_204".into()),
            ]))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"delay":123}"#)
            .create_async()
            .await;

        let client = MihomoClient::new(&server.url(), None).unwrap();
        let result = client
            .test_delay("proxy1", "http://www.gstatic.com/generate_204", 5000)
            .await;

        mock.assert_async().await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 123);
    }

    #[tokio::test]
    async fn test_reload_config_with_path() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("PUT", "/configs")
            .match_query(Matcher::UrlEncoded("force".into(), "true".into()))
            .match_body(Matcher::Json(
                serde_json::json!({"path":"/path/to/config.yaml"}),
            ))
            .with_status(204)
            .create_async()
            .await;

        let client = MihomoClient::new(&server.url(), None).unwrap();
        let result = client.reload_config(Some("/path/to/config.yaml")).await;

        mock.assert_async().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_reload_config_without_path() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("PUT", "/configs")
            .match_query(Matcher::UrlEncoded("force".into(), "true".into()))
            .with_status(204)
            .create_async()
            .await;

        let client = MihomoClient::new(&server.url(), None).unwrap();
        let result = client.reload_config(None).await;

        mock.assert_async().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_get_memory() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("GET", "/memory")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"inuse":12345678,"oslimit":2147483648}"#)
            .create_async()
            .await;

        let client = MihomoClient::new(&server.url(), None).unwrap();
        let result = client.get_memory().await;

        mock.assert_async().await;
        assert!(result.is_ok());
        let memory = result.unwrap();
        assert_eq!(memory.in_use, 12345678);
        assert_eq!(memory.os_limit, 2147483648);
    }

    #[tokio::test]
    async fn test_get_connections() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("GET", "/connections")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"connections":[],"downloadTotal":0,"uploadTotal":0}"#)
            .create_async()
            .await;

        let client = MihomoClient::new(&server.url(), None).unwrap();
        let result = client.get_connections().await;

        mock.assert_async().await;
        assert!(result.is_ok());
        let connections = result.unwrap();
        assert_eq!(connections.connections.len(), 0);
        assert_eq!(connections.download_total, 0);
        assert_eq!(connections.upload_total, 0);
    }

    #[tokio::test]
    async fn test_close_all_connections() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("DELETE", "/connections")
            .with_status(204)
            .create_async()
            .await;

        let client = MihomoClient::new(&server.url(), None).unwrap();
        let result = client.close_all_connections().await;

        mock.assert_async().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_close_connection() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("DELETE", "/connections/test-id-123")
            .with_status(204)
            .create_async()
            .await;

        let client = MihomoClient::new(&server.url(), None).unwrap();
        let result = client.close_connection("test-id-123").await;

        mock.assert_async().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_client_with_auth() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("GET", "/version")
            .match_header("authorization", "Bearer my-secret")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"version":"v1.18.0","premium":true,"meta":true}"#)
            .create_async()
            .await;

        let client = MihomoClient::new(&server.url(), Some("my-secret".to_string())).unwrap();
        let result = client.get_version().await;

        mock.assert_async().await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_ws_request_with_auth_header() {
        let request =
            MihomoClient::ws_request_with_auth("ws://127.0.0.1:9090/logs", Some("my-secret"))
                .expect("request should be built");
        let auth = request
            .headers()
            .get("Authorization")
            .and_then(|v| v.to_str().ok());
        assert_eq!(auth, Some("Bearer my-secret"));
    }

    #[tokio::test]
    async fn test_stream_logs_message_handling() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let ws = accept_async(stream).await.unwrap();
            let (mut tx, _) = ws.split();
            use futures_util::SinkExt;
            tx.send(WsMessage::Text("test log".into())).await.ok();
        });

        let client = MihomoClient::new(&format!("http://{}", addr), None).unwrap();
        let mut rx = client.stream_logs(None).await.unwrap();

        tokio::time::timeout(std::time::Duration::from_secs(1), rx.recv())
            .await
            .ok();
    }

    #[tokio::test]
    async fn test_stream_traffic_message_handling() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let ws = accept_async(stream).await.unwrap();
            let (mut tx, _) = ws.split();
            use futures_util::SinkExt;
            tx.send(WsMessage::Text(r#"{"up":100,"down":200}"#.into()))
                .await
                .ok();
        });

        let client = MihomoClient::new(&format!("http://{}", addr), None).unwrap();
        let mut rx = client.stream_traffic().await.unwrap();

        tokio::time::timeout(std::time::Duration::from_secs(1), rx.recv())
            .await
            .ok();
    }

    #[tokio::test]
    async fn test_stream_connections_message_handling() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let ws = accept_async(stream).await.unwrap();
            let (mut tx, _) = ws.split();
            use futures_util::SinkExt;
            tx.send(WsMessage::Text(
                r#"{"connections":[],"downloadTotal":0,"uploadTotal":0}"#.into(),
            ))
            .await
            .ok();
        });

        let client = MihomoClient::new(&format!("http://{}", addr), None).unwrap();
        let mut rx = client.stream_connections().await.unwrap();

        tokio::time::timeout(std::time::Duration::from_secs(1), rx.recv())
            .await
            .ok();
    }
}
