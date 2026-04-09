use super::error::Result;
use super::types::*;
use percent_encoding::{utf8_percent_encode, AsciiSet, CONTROLS};
use reqwest::Client;
use serde_json::json;
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;
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

    fn encode_path_segment(input: &str) -> String {
        utf8_percent_encode(input, PATH_SEGMENT_ENCODE_SET).to_string()
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
}

mod http {
    use super::Result;
    use crate::core::MihomoError;
    use std::path::PathBuf;
    use std::time::Duration;
    use tokio::io::AsyncRead;

    const HTTP_READ_TIMEOUT: Duration = Duration::from_secs(10);
    const MAX_HTTP_HEADER_BYTES: usize = 64 * 1024;

    impl super::MihomoClient {
        async fn read_http_response<R>(reader: &mut R) -> Result<Vec<u8>>
        where
            R: AsyncRead + Unpin,
        {
            use tokio::io::AsyncReadExt;

            let mut response = Vec::new();
            let mut buf = [0u8; 4096];
            let header_end = loop {
                let n = tokio::time::timeout(HTTP_READ_TIMEOUT, reader.read(&mut buf))
                    .await
                    .map_err(|_| {
                        MihomoError::Service(
                            "Timeout while reading HTTP response headers".to_string(),
                        )
                    })??;

                if n == 0 {
                    return Err(MihomoError::config("Invalid HTTP response"));
                }

                response.extend_from_slice(&buf[..n]);
                if response.len() > MAX_HTTP_HEADER_BYTES {
                    return Err(MihomoError::config(
                        "Invalid HTTP response: headers too large",
                    ));
                }

                if let Some(pos) = response.windows(4).position(|w| w == b"\r\n\r\n") {
                    break pos + 4;
                }
            };

            let headers = &response[..header_end];
            let headers_text = String::from_utf8_lossy(headers);
            let status_line = headers_text.lines().next().unwrap_or_default();
            let status_code = status_line
                .split_whitespace()
                .nth(1)
                .and_then(|code| code.parse::<u16>().ok());

            let content_length = headers_text.lines().find_map(|line| {
                let (name, value) = line.split_once(':')?;
                if name.trim().eq_ignore_ascii_case("content-length") {
                    value.trim().parse::<usize>().ok()
                } else {
                    None
                }
            });

            let mut body = response[header_end..].to_vec();
            match content_length {
                Some(expected) => {
                    while body.len() < expected {
                        let n = tokio::time::timeout(HTTP_READ_TIMEOUT, reader.read(&mut buf))
                            .await
                            .map_err(|_| {
                                MihomoError::Service(
                                    "Timeout while reading HTTP response body".to_string(),
                                )
                            })??;
                        if n == 0 {
                            break;
                        }
                        body.extend_from_slice(&buf[..n]);
                    }
                }
                None => {
                    return Err(MihomoError::config(
                        "Invalid HTTP response: missing Content-Length",
                    ));
                }
            }

            if matches!(status_code, Some(code) if code >= 400) {
                return Err(MihomoError::Service(format!(
                    "HTTP error {}: {}",
                    status_code.unwrap_or_default(),
                    String::from_utf8_lossy(&body)
                )));
            }

            Ok(body)
        }

        pub(super) async fn http_request(
            &self,
            method: &str,
            path: &str,
            query: Option<&[(&str, String)]>,
            body: Option<serde_json::Value>,
        ) -> Result<Vec<u8>> {
            match &self.transport {
                super::Transport::Tcp { client, base_url } => {
                    let url = base_url.join(path)?;
                    let mut req = match method {
                        "GET" => client.get(url),
                        "PUT" => client.put(url),
                        "DELETE" => client.delete(url),
                        _ => return Err(MihomoError::config("Unsupported method")),
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
                super::Transport::Unix { socket_path } => {
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
            use tokio::io::AsyncWriteExt;

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
                Self::read_http_response(&mut stream).await
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
                Self::read_http_response(&mut stream).await
            }
            #[cfg(not(any(unix, windows)))]
            {
                let _ = (method, path, query, body, socket_path);
                Err(MihomoError::config(
                    "Unix domain sockets are not supported on this platform",
                ))
            }
        }

        fn add_auth(&self, mut req: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
            if let Some(secret) = &self.secret {
                req = req.bearer_auth(secret);
            }
            req
        }
    }
}

mod ws {
    use super::Result;
    use super::{ConnectionSnapshot, TrafficData};
    use futures_util::StreamExt;
    use std::time::Duration;
    use tokio_tungstenite::tungstenite::client::IntoClientRequest;
    use tokio_tungstenite::{connect_async, tungstenite::Message};
    use url::Url;

    impl super::MihomoClient {
        pub fn with_ws_connect_timeout(mut self, timeout: Duration) -> Self {
            self.ws_connect_timeout = timeout.max(Duration::from_millis(1));
            self
        }

        pub(super) fn ws_request_with_auth(
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

        fn ws_timeout_error(endpoint: &str) -> crate::core::MihomoError {
            crate::core::MihomoError::Service(format!("WebSocket connection timeout: {}", endpoint))
        }

        fn spawn_ws_reader<S, T, F>(
            ws_stream: tokio_tungstenite::WebSocketStream<S>,
            tx: tokio::sync::mpsc::UnboundedSender<T>,
            mut parse_text: F,
        ) where
            S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + 'static,
            T: Send + 'static,
            F: FnMut(String) -> Option<T> + Send + 'static,
        {
            tokio::spawn(async move {
                let (_, mut read) = ws_stream.split();
                while let Some(msg) = read.next().await {
                    match msg {
                        Ok(Message::Text(text)) => {
                            if let Some(item) = parse_text(text.to_string()) {
                                if tx.send(item).is_err() {
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

        fn build_ws_path(endpoint: &str, query: Option<&Vec<(String, String)>>) -> String {
            let mut path = endpoint.to_string();
            if let Some(query) = query {
                let mut serializer = url::form_urlencoded::Serializer::new(String::new());
                for (k, v) in query {
                    serializer.append_pair(k, v);
                }
                path.push('?');
                path.push_str(&serializer.finish());
            }
            path
        }

        fn build_tcp_ws_url(
            base_url: &Url,
            endpoint: &str,
            query: Option<&Vec<(String, String)>>,
        ) -> String {
            let mut ws_url = base_url.clone();
            ws_url
                .set_scheme(if ws_url.scheme() == "https" {
                    "wss"
                } else {
                    "ws"
                })
                .ok();
            ws_url.set_path(endpoint);
            if let Some(query) = query {
                let mut serializer = url::form_urlencoded::Serializer::new(String::new());
                for (k, v) in query {
                    serializer.append_pair(k, v);
                }
                ws_url.set_query(Some(&serializer.finish()));
            }
            ws_url.to_string()
        }

        async fn stream_with_parser<T, F>(
            &self,
            endpoint: &str,
            query: Option<Vec<(String, String)>>,
            parser: F,
        ) -> Result<tokio::sync::mpsc::UnboundedReceiver<T>>
        where
            T: Send + 'static,
            F: FnMut(String) -> Option<T> + Send + Clone + 'static,
        {
            let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
            let endpoint_name = endpoint.trim_start_matches('/');

            match &self.transport {
                super::Transport::Tcp { base_url, .. } => {
                    let ws_url = Self::build_tcp_ws_url(base_url, endpoint, query.as_ref());
                    let request = Self::ws_request_with_auth(&ws_url, self.secret.as_deref())?;
                    let (ws_stream, _) =
                        tokio::time::timeout(self.ws_connect_timeout, connect_async(request))
                            .await
                            .map_err(|_| Self::ws_timeout_error(endpoint_name))??;
                    Self::spawn_ws_reader(ws_stream, tx, parser);
                }
                super::Transport::Unix { socket_path } => {
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
                        .map_err(|_| Self::ws_timeout_error(endpoint_name))??;

                        let path = Self::build_ws_path(endpoint, query.as_ref());
                        let ws_url = format!("ws://localhost{}", path);
                        let request = Self::ws_request_with_auth(&ws_url, secret.as_deref())?;

                        let (ws_stream, _) = tokio::time::timeout(
                            self.ws_connect_timeout,
                            client_async(request, stream),
                        )
                        .await
                        .map_err(|_| Self::ws_timeout_error(endpoint_name))??;
                        Self::spawn_ws_reader(ws_stream, tx, parser);
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
                        .map_err(|_| Self::ws_timeout_error(endpoint_name))?
                        .map_err(|e| {
                            crate::core::MihomoError::Service(format!(
                                "Failed to join named pipe connect task: {}",
                                e
                            ))
                        })??;

                        let path = Self::build_ws_path(endpoint, query.as_ref());
                        let ws_url = format!("ws://localhost{}", path);
                        let request = Self::ws_request_with_auth(&ws_url, secret.as_deref())?;

                        let (ws_stream, _) = tokio::time::timeout(
                            self.ws_connect_timeout,
                            client_async(request, stream),
                        )
                        .await
                        .map_err(|_| Self::ws_timeout_error(endpoint_name))??;
                        Self::spawn_ws_reader(ws_stream, tx, parser);
                    }
                    #[cfg(not(any(unix, windows)))]
                    {
                        let _ = (socket_path, secret, query, parser);
                        return Err(crate::core::MihomoError::config(
                            "Unix domain sockets are not supported on this platform",
                        ));
                    }
                }
            }

            Ok(rx)
        }

        pub async fn stream_logs(
            &self,
            level: Option<&str>,
        ) -> Result<tokio::sync::mpsc::UnboundedReceiver<String>> {
            let query = level.map(|l| vec![("level".to_string(), l.to_string())]);
            self.stream_with_parser("/logs", query, Some).await
        }

        pub async fn stream_traffic(
            &self,
        ) -> Result<tokio::sync::mpsc::UnboundedReceiver<TrafficData>> {
            self.stream_with_parser("/traffic", None, |text| {
                serde_json::from_str::<TrafficData>(&text).ok()
            })
            .await
        }

        pub async fn stream_connections(
            &self,
        ) -> Result<tokio::sync::mpsc::UnboundedReceiver<ConnectionSnapshot>> {
            self.stream_with_parser("/connections", None, |text| {
                serde_json::from_str::<ConnectionSnapshot>(&text).ok()
            })
            .await
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures_util::StreamExt;
    use mockito::{Matcher, Server};
    #[cfg(any(unix, windows))]
    use std::time::{SystemTime, UNIX_EPOCH};
    #[cfg(any(unix, windows))]
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    #[cfg(windows)]
    use tokio::net::windows::named_pipe::ServerOptions;
    use tokio::net::TcpListener;
    #[cfg(unix)]
    use tokio::net::UnixListener;
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
    fn test_with_ws_connect_timeout_clamps_to_minimum() {
        let client = MihomoClient::new("http://127.0.0.1:9090", None)
            .unwrap()
            .with_ws_connect_timeout(Duration::from_millis(0));
        assert_eq!(client.ws_connect_timeout, Duration::from_millis(1));
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

    #[cfg(unix)]
    fn unique_socket_path(prefix: &str) -> std::path::PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        std::path::PathBuf::from(format!(
            "/tmp/mihomo-rs-{}-{}-{}.sock",
            prefix,
            std::process::id(),
            nanos
        ))
    }

    #[cfg(windows)]
    fn unique_pipe_name(prefix: &str) -> String {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        format!(
            r"\\.\pipe\mihomo-rs-{}-{}-{}",
            prefix,
            std::process::id(),
            nanos
        )
    }

    #[tokio::test]
    #[cfg(unix)]
    async fn test_unix_http_get_version() {
        let socket = unique_socket_path("version");
        let _ = std::fs::remove_file(&socket);
        let listener = UnixListener::bind(&socket).expect("bind unix socket");

        let server_task = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.expect("accept");
            let mut buf = [0u8; 4096];
            let n = stream.read(&mut buf).await.expect("read request");
            let request = String::from_utf8_lossy(&buf[..n]).to_string();
            assert!(request.starts_with("GET /version HTTP/1.1"));

            let response = "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 50\r\n\r\n{\"version\":\"v1.20.0\",\"premium\":false,\"meta\":false}";
            stream
                .write_all(response.as_bytes())
                .await
                .expect("write response");
        });

        let client = MihomoClient::new(socket.to_str().expect("socket str"), None).unwrap();
        let version = client.get_version().await.expect("get version");
        assert_eq!(version.version, "v1.20.0");

        server_task.await.expect("server task");
        let _ = std::fs::remove_file(&socket);
    }

    #[tokio::test]
    #[cfg(unix)]
    async fn test_unix_http_error_response() {
        let socket = unique_socket_path("http-error");
        let _ = std::fs::remove_file(&socket);
        let listener = UnixListener::bind(&socket).expect("bind unix socket");

        let server_task = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.expect("accept");
            let mut buf = [0u8; 1024];
            let _ = stream.read(&mut buf).await.expect("read request");
            let response = "HTTP/1.1 500 Internal Server Error\r\nContent-Type: text/plain\r\nContent-Length: 4\r\n\r\nboom";
            stream
                .write_all(response.as_bytes())
                .await
                .expect("write error response");
        });

        let client = MihomoClient::new(socket.to_str().expect("socket str"), None).unwrap();
        let err = client.get_version().await.expect_err("expect HTTP error");
        assert!(err.to_string().contains("HTTP error 500"));

        server_task.await.expect("server task");
        let _ = std::fs::remove_file(&socket);
    }

    #[tokio::test]
    #[cfg(unix)]
    async fn test_unix_reload_config_with_auth_and_query() {
        let socket = unique_socket_path("reload");
        let _ = std::fs::remove_file(&socket);
        let listener = UnixListener::bind(&socket).expect("bind unix socket");

        let server_task = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.expect("accept");
            let mut buf = [0u8; 4096];
            let n = stream.read(&mut buf).await.expect("read request");
            let request = String::from_utf8_lossy(&buf[..n]).to_string();
            assert!(request.starts_with("PUT /configs?force=true HTTP/1.1"));
            assert!(request.contains("Authorization: Bearer secret-token"));
            assert!(request.contains("\"path\":\"/tmp/test-config.yaml\""));

            let response = "HTTP/1.1 204 No Content\r\nContent-Length: 0\r\n\r\n";
            stream
                .write_all(response.as_bytes())
                .await
                .expect("write response");
        });

        let client = MihomoClient::new(
            socket.to_str().expect("socket str"),
            Some("secret-token".to_string()),
        )
        .unwrap();
        client
            .reload_config(Some("/tmp/test-config.yaml"))
            .await
            .expect("reload config");

        server_task.await.expect("server task");
        let _ = std::fs::remove_file(&socket);
    }

    #[tokio::test]
    async fn test_http_request_put_with_query_and_body_over_tcp() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("PUT", "/raw")
            .match_query(Matcher::UrlEncoded("k".into(), "v".into()))
            .match_header("authorization", "Bearer token-1")
            .match_body(Matcher::JsonString(r#"{"value":1}"#.to_string()))
            .with_status(200)
            .with_body("ok")
            .create_async()
            .await;

        let client = MihomoClient::new(&server.url(), Some("token-1".to_string())).unwrap();
        let body = client
            .http_request(
                "PUT",
                "/raw",
                Some(&[("k", "v".to_string())]),
                Some(json!({"value": 1})),
            )
            .await
            .expect("http request should succeed");

        mock.assert_async().await;
        assert_eq!(body, b"ok");
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
    #[cfg(windows)]
    async fn test_windows_http_get_version_over_named_pipe() {
        let pipe_name = unique_pipe_name("version");
        let mut server = ServerOptions::new()
            .create(&pipe_name)
            .expect("create named pipe server");

        let server_task = tokio::spawn(async move {
            server.connect().await.expect("connect named pipe");
            let mut buf = [0u8; 4096];
            let n = server.read(&mut buf).await.expect("read request");
            let request = String::from_utf8_lossy(&buf[..n]).to_string();
            assert!(request.starts_with("GET /version HTTP/1.1"));

            let response = "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 50\r\n\r\n{\"version\":\"v1.20.0\",\"premium\":false,\"meta\":false}";
            server
                .write_all(response.as_bytes())
                .await
                .expect("write response");
        });

        let client = MihomoClient::new(&pipe_name, None).expect("create client");
        let version = client.get_version().await.expect("get version");
        assert_eq!(version.version, "v1.20.0");
        assert!(!version.premium);
        assert!(!version.meta);
        server_task.await.expect("server task");
    }

    #[tokio::test]
    #[cfg(windows)]
    async fn test_windows_stream_logs_over_named_pipe() {
        use futures_util::SinkExt;

        let pipe_name = unique_pipe_name("stream-logs");
        let mut server = ServerOptions::new()
            .create(&pipe_name)
            .expect("create named pipe server");

        tokio::spawn(async move {
            server.connect().await.expect("connect named pipe");
            let ws = accept_async(server).await.expect("accept ws");
            let (mut tx, _) = ws.split();
            tx.send(WsMessage::Text("windows log line".into()))
                .await
                .expect("send log");
        });

        let client = MihomoClient::new(&pipe_name, None).expect("create client");
        let mut rx = client
            .stream_logs(Some("debug"))
            .await
            .expect("stream logs over named pipe");
        let got = tokio::time::timeout(std::time::Duration::from_secs(1), rx.recv())
            .await
            .expect("recv timeout")
            .expect("log message");
        assert_eq!(got, "windows log line");
    }

    #[tokio::test]
    #[cfg(windows)]
    async fn test_windows_stream_traffic_over_named_pipe() {
        use futures_util::SinkExt;

        let pipe_name = unique_pipe_name("stream-traffic");
        let mut server = ServerOptions::new()
            .create(&pipe_name)
            .expect("create named pipe server");

        tokio::spawn(async move {
            server.connect().await.expect("connect named pipe");
            let ws = accept_async(server).await.expect("accept ws");
            let (mut tx, _) = ws.split();
            tx.send(WsMessage::Text(r#"{"up":7,"down":9}"#.into()))
                .await
                .expect("send traffic");
        });

        let client = MihomoClient::new(&pipe_name, None).expect("create client");
        let mut rx = client
            .stream_traffic()
            .await
            .expect("stream traffic over named pipe");
        let got = tokio::time::timeout(std::time::Duration::from_secs(1), rx.recv())
            .await
            .expect("recv timeout")
            .expect("traffic message");
        assert_eq!(got.up, 7);
        assert_eq!(got.down, 9);
    }

    #[tokio::test]
    #[cfg(windows)]
    async fn test_windows_stream_connections_over_named_pipe() {
        use futures_util::SinkExt;

        let pipe_name = unique_pipe_name("stream-connections");
        let mut server = ServerOptions::new()
            .create(&pipe_name)
            .expect("create named pipe server");

        tokio::spawn(async move {
            server.connect().await.expect("connect named pipe");
            let ws = accept_async(server).await.expect("accept ws");
            let (mut tx, _) = ws.split();
            tx.send(WsMessage::Text(
                r#"{"connections":[],"downloadTotal":0,"uploadTotal":0}"#.into(),
            ))
            .await
            .expect("send connection snapshot");
        });

        let client = MihomoClient::new(&pipe_name, None).expect("create client");
        let mut rx = client
            .stream_connections()
            .await
            .expect("stream connections over named pipe");
        let got = tokio::time::timeout(std::time::Duration::from_secs(1), rx.recv())
            .await
            .expect("recv timeout")
            .expect("connection snapshot");
        assert_eq!(got.connections.len(), 0);
        assert_eq!(got.download_total, 0);
        assert_eq!(got.upload_total, 0);
    }

    #[tokio::test]
    #[cfg(windows)]
    async fn test_windows_get_version_returns_error_when_pipe_missing() {
        let missing_pipe = unique_pipe_name("missing");
        let client = MihomoClient::new(&missing_pipe, None).expect("create client");
        let result = client.get_version().await;
        assert!(result.is_err());
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
    async fn test_http_request_rejects_unsupported_method() {
        let client = MihomoClient::new("http://127.0.0.1:9090", None).unwrap();
        let err = client
            .http_request("POST", "/version", None, None)
            .await
            .expect_err("unsupported method should fail");
        assert!(err.to_string().contains("Unsupported method"));
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
            .with_body(
                r#"{"proxies":{"DIRECT":{"type":"Direct","udp":true,"now":"","all":[],"history":[]}}}"#,
            )
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

    #[test]
    fn test_ws_request_with_invalid_header_value_is_ignored() {
        let request =
            MihomoClient::ws_request_with_auth("ws://127.0.0.1:9090/logs", Some("bad\r\nsecret"))
                .expect("request should still be built");
        assert!(request.headers().get("Authorization").is_none());
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

    #[tokio::test]
    #[cfg(unix)]
    async fn test_stream_logs_over_unix_socket() {
        let socket = unique_socket_path("stream-logs");
        let _ = std::fs::remove_file(&socket);
        let listener = UnixListener::bind(&socket).expect("bind unix socket");

        tokio::spawn(async move {
            let (stream, _) = listener.accept().await.expect("accept");
            let ws = accept_async(stream).await.expect("accept ws");
            let (mut tx, _) = ws.split();
            use futures_util::SinkExt;
            tx.send(WsMessage::Text("unix log line".into()))
                .await
                .expect("send unix log");
        });

        let client = MihomoClient::new(
            socket.to_str().expect("socket path"),
            Some("secret-token".to_string()),
        )
        .unwrap();
        let mut rx = client
            .stream_logs(Some("debug"))
            .await
            .expect("unix logs stream");
        let got = tokio::time::timeout(std::time::Duration::from_secs(1), rx.recv())
            .await
            .expect("recv timeout")
            .expect("log item");
        assert_eq!(got, "unix log line");
        let _ = std::fs::remove_file(&socket);
    }

    #[tokio::test]
    #[cfg(unix)]
    async fn test_stream_traffic_over_unix_socket() {
        let socket = unique_socket_path("stream-traffic");
        let _ = std::fs::remove_file(&socket);
        let listener = UnixListener::bind(&socket).expect("bind unix socket");

        tokio::spawn(async move {
            let (stream, _) = listener.accept().await.expect("accept");
            let ws = accept_async(stream).await.expect("accept ws");
            let (mut tx, _) = ws.split();
            use futures_util::SinkExt;
            tx.send(WsMessage::Text(r#"{"up":7,"down":9}"#.into()))
                .await
                .expect("send unix traffic");
        });

        let client = MihomoClient::new(
            socket.to_str().expect("socket path"),
            Some("secret-token".to_string()),
        )
        .unwrap();
        let mut rx = client.stream_traffic().await.expect("unix traffic stream");
        let got = tokio::time::timeout(std::time::Duration::from_secs(1), rx.recv())
            .await
            .expect("recv timeout")
            .expect("traffic item");
        assert_eq!(got.up, 7);
        assert_eq!(got.down, 9);
        let _ = std::fs::remove_file(&socket);
    }

    #[tokio::test]
    #[cfg(unix)]
    async fn test_stream_connections_over_unix_socket() {
        let socket = unique_socket_path("stream-connections");
        let _ = std::fs::remove_file(&socket);
        let listener = UnixListener::bind(&socket).expect("bind unix socket");

        tokio::spawn(async move {
            let (stream, _) = listener.accept().await.expect("accept");
            let ws = accept_async(stream).await.expect("accept ws");
            let (mut tx, _) = ws.split();
            use futures_util::SinkExt;
            tx.send(WsMessage::Text(
                r#"{"connections":[],"downloadTotal":0,"uploadTotal":0}"#.into(),
            ))
            .await
            .expect("send unix connections");
        });

        let client = MihomoClient::new(
            socket.to_str().expect("socket path"),
            Some("secret-token".to_string()),
        )
        .unwrap();
        let mut rx = client
            .stream_connections()
            .await
            .expect("unix connections stream");
        let got = tokio::time::timeout(std::time::Duration::from_secs(1), rx.recv())
            .await
            .expect("recv timeout")
            .expect("snapshot item");
        assert_eq!(got.connections.len(), 0);
        let _ = std::fs::remove_file(&socket);
    }

    #[tokio::test]
    #[cfg(unix)]
    async fn test_stream_logs_over_unix_socket_sender_breaks_when_receiver_dropped() {
        use futures_util::SinkExt;
        let socket = unique_socket_path("stream-logs-drop");
        let _ = std::fs::remove_file(&socket);
        let listener = UnixListener::bind(&socket).expect("bind unix socket");

        tokio::spawn(async move {
            let (stream, _) = listener.accept().await.expect("accept");
            let ws = accept_async(stream).await.expect("accept ws");
            let (mut tx, _) = ws.split();
            tokio::time::sleep(std::time::Duration::from_millis(30)).await;
            tx.send(WsMessage::Text("drop".into())).await.ok();
        });

        let client = MihomoClient::new(socket.to_str().expect("socket path"), None).unwrap();
        let rx = client.stream_logs(None).await.expect("stream logs");
        drop(rx);
        tokio::time::sleep(std::time::Duration::from_millis(80)).await;
        let _ = std::fs::remove_file(&socket);
    }

    #[tokio::test]
    #[cfg(unix)]
    async fn test_stream_traffic_over_unix_socket_sender_breaks_when_receiver_dropped() {
        use futures_util::SinkExt;
        let socket = unique_socket_path("stream-traffic-drop");
        let _ = std::fs::remove_file(&socket);
        let listener = UnixListener::bind(&socket).expect("bind unix socket");

        tokio::spawn(async move {
            let (stream, _) = listener.accept().await.expect("accept");
            let ws = accept_async(stream).await.expect("accept ws");
            let (mut tx, _) = ws.split();
            tokio::time::sleep(std::time::Duration::from_millis(30)).await;
            tx.send(WsMessage::Text(r#"{"up":1,"down":2}"#.into()))
                .await
                .ok();
        });

        let client = MihomoClient::new(socket.to_str().expect("socket path"), None).unwrap();
        let rx = client.stream_traffic().await.expect("stream traffic");
        drop(rx);
        tokio::time::sleep(std::time::Duration::from_millis(80)).await;
        let _ = std::fs::remove_file(&socket);
    }

    #[tokio::test]
    #[cfg(unix)]
    async fn test_stream_connections_over_unix_socket_sender_breaks_when_receiver_dropped() {
        use futures_util::SinkExt;
        let socket = unique_socket_path("stream-connections-drop");
        let _ = std::fs::remove_file(&socket);
        let listener = UnixListener::bind(&socket).expect("bind unix socket");

        tokio::spawn(async move {
            let (stream, _) = listener.accept().await.expect("accept");
            let ws = accept_async(stream).await.expect("accept ws");
            let (mut tx, _) = ws.split();
            tokio::time::sleep(std::time::Duration::from_millis(30)).await;
            tx.send(WsMessage::Text(
                r#"{"connections":[],"downloadTotal":0,"uploadTotal":0}"#.into(),
            ))
            .await
            .ok();
        });

        let client = MihomoClient::new(socket.to_str().expect("socket path"), None).unwrap();
        let rx = client
            .stream_connections()
            .await
            .expect("stream connections");
        drop(rx);
        tokio::time::sleep(std::time::Duration::from_millis(80)).await;
        let _ = std::fs::remove_file(&socket);
    }
}
