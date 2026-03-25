use super::error::Result;
use super::types::*;
use futures_util::StreamExt;
use reqwest::Client;
use serde_json::json;
use std::collections::HashMap;
use std::path::PathBuf;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use url::Url;

#[derive(Clone)]
enum Transport {
    Tcp { client: Client, base_url: Url },
    Unix { socket_path: PathBuf },
}

#[derive(Clone)]
pub struct MihomoClient {
    transport: Transport,
    secret: Option<String>,
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

        Ok(Self { transport, secret })
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

                let resp = req.send().await?;
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
                    let params: Vec<String> =
                        q.iter().map(|(k, v)| format!("{}={}", k, v)).collect();
                    format!("?{}", params.join("&"))
                })
                .unwrap_or_default();

            let body_str = body
                .map(|b| serde_json::to_string(&b).unwrap())
                .unwrap_or_default();

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
            if let Some(pos) = response_str.find("\r\n\r\n") {
                Ok(response[pos + 4..].to_vec())
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
                    let params: Vec<String> =
                        q.iter().map(|(k, v)| format!("{}={}", k, v)).collect();
                    format!("?{}", params.join("&"))
                })
                .unwrap_or_default();

            let body_str = body
                .map(|b| serde_json::to_string(&b).unwrap())
                .unwrap_or_default();

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
            if let Some(pos) = response_str.find("\r\n\r\n") {
                Ok(response[pos + 4..].to_vec())
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
        let response = self
            .http_request("GET", &format!("/proxies/{}", name), None, None)
            .await?;
        Ok(serde_json::from_slice(&response)?)
    }

    pub async fn switch_proxy(&self, group: &str, proxy: &str) -> Result<()> {
        log::debug!("Switching group '{}' to proxy '{}'", group, proxy);
        self.http_request(
            "PUT",
            &format!("/proxies/{}", group),
            None,
            Some(json!({ "name": proxy })),
        )
        .await?;
        log::debug!("Successfully switched group '{}' to '{}'", group, proxy);
        Ok(())
    }

    pub async fn test_delay(&self, proxy: &str, test_url: &str, timeout: u32) -> Result<u32> {
        let response = self
            .http_request(
                "GET",
                &format!("/proxies/{}/delay", proxy),
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
                    ws_url.set_query(Some(&format!("level={}", level)));
                }

                let ws_url_str = ws_url.to_string();
                tokio::spawn(async move {
                    if let Ok((ws_stream, _)) = connect_async(&ws_url_str).await {
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
                    }
                });
            }
            Transport::Unix { socket_path } => {
                let socket_path = socket_path.clone();
                let level = level.map(|s| s.to_string());

                tokio::spawn(async move {
                    #[cfg(unix)]
                    {
                        use tokio::net::UnixStream;
                        use tokio_tungstenite::client_async;

                        if let Ok(stream) = UnixStream::connect(&socket_path).await {
                            let mut path = "/logs".to_string();
                            if let Some(l) = level {
                                path.push_str(&format!("?level={}", l));
                            }

                            let request_url = format!("ws://localhost{}", path);
                            if let Ok((ws_stream, _)) = client_async(request_url, stream).await {
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
                            }
                        }
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

                        if let Ok(stream) = ClientOptions::new().open(&pipe_name) {
                            let mut path = "/logs".to_string();
                            if let Some(l) = level {
                                path.push_str(&format!("?level={}", l));
                            }

                            let request_url = format!("ws://localhost{}", path);
                            if let Ok((ws_stream, _)) = client_async(request_url, stream).await {
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
                            }
                        }
                    }
                });
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
                tokio::spawn(async move {
                    if let Ok((ws_stream, _)) = connect_async(&ws_url_str).await {
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
                    }
                });
            }
            Transport::Unix { socket_path } => {
                let socket_path = socket_path.clone();

                tokio::spawn(async move {
                    #[cfg(unix)]
                    {
                        use tokio::net::UnixStream;
                        use tokio_tungstenite::client_async;

                        if let Ok(stream) = UnixStream::connect(&socket_path).await {
                            if let Ok((ws_stream, _)) =
                                client_async("ws://localhost/traffic", stream).await
                            {
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
                            }
                        }
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

                        if let Ok(stream) = ClientOptions::new().open(&pipe_name) {
                            if let Ok((ws_stream, _)) =
                                client_async("ws://localhost/traffic", stream).await
                            {
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
                            }
                        }
                    }
                });
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
        log::debug!("Closing connection '{}'", id);
        self.http_request("DELETE", &format!("/connections/{}", id), None, None)
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
                tokio::spawn(async move {
                    if let Ok((ws_stream, _)) = connect_async(&ws_url_str).await {
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
                    }
                });
            }
            Transport::Unix { socket_path } => {
                let socket_path = socket_path.clone();

                tokio::spawn(async move {
                    #[cfg(unix)]
                    {
                        use tokio::net::UnixStream;
                        use tokio_tungstenite::client_async;

                        if let Ok(stream) = UnixStream::connect(&socket_path).await {
                            if let Ok((ws_stream, _)) =
                                client_async("ws://localhost/connections", stream).await
                            {
                                let (_, mut read) = ws_stream.split();
                                while let Some(msg) = read.next().await {
                                    match msg {
                                        Ok(Message::Text(text)) => {
                                            if let Ok(snapshot) =
                                                serde_json::from_str::<ConnectionSnapshot>(
                                                    text.as_ref(),
                                                )
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
                            }
                        }
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

                        if let Ok(stream) = ClientOptions::new().open(&pipe_name) {
                            if let Ok((ws_stream, _)) =
                                client_async("ws://localhost/connections", stream).await
                            {
                                let (_, mut read) = ws_stream.split();
                                while let Some(msg) = read.next().await {
                                    match msg {
                                        Ok(Message::Text(text)) => {
                                            if let Ok(snapshot) =
                                                serde_json::from_str::<ConnectionSnapshot>(
                                                    text.as_ref(),
                                                )
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
                            }
                        }
                    }
                });
            }
        }

        Ok(rx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures_util::SinkExt;
    use mockito::{Matcher, Server};
    use tempfile::tempdir;
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
    async fn test_stream_logs_ignores_non_text_and_handles_close() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let ws = accept_async(stream).await.unwrap();
            let (mut tx, _) = ws.split();
            tx.send(WsMessage::Binary(vec![1, 2, 3].into())).await.ok();
            tx.send(WsMessage::Close(None)).await.ok();
        });

        let client = MihomoClient::new(&format!("http://{}", addr), None).unwrap();
        let mut rx = client.stream_logs(None).await.unwrap();
        let recv = tokio::time::timeout(std::time::Duration::from_millis(200), rx.recv()).await;
        assert!(recv.is_err() || recv.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_stream_logs_drop_receiver_covers_send_error_branch() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let ws = accept_async(stream).await.unwrap();
            let (mut tx, _) = ws.split();
            tx.send(WsMessage::Text("drop-log".into())).await.ok();
        });

        let client = MihomoClient::new(&format!("http://{}", addr), None).unwrap();
        let rx = client.stream_logs(None).await.unwrap();
        drop(rx);
        tokio::time::sleep(std::time::Duration::from_millis(80)).await;
    }

    #[tokio::test]
    async fn test_stream_logs_https_with_level_query() {
        let client = MihomoClient::new("https://127.0.0.1:65534", None).unwrap();
        let _rx = client.stream_logs(Some("debug")).await.unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
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
    async fn test_stream_traffic_ignores_non_text_and_handles_close() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let ws = accept_async(stream).await.unwrap();
            let (mut tx, _) = ws.split();
            tx.send(WsMessage::Binary(vec![9, 9].into())).await.ok();
            tx.send(WsMessage::Close(None)).await.ok();
        });

        let client = MihomoClient::new(&format!("http://{}", addr), None).unwrap();
        let mut rx = client.stream_traffic().await.unwrap();
        let recv = tokio::time::timeout(std::time::Duration::from_millis(200), rx.recv()).await;
        assert!(recv.is_err() || recv.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_stream_traffic_drop_receiver_covers_send_error_branch() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let ws = accept_async(stream).await.unwrap();
            let (mut tx, _) = ws.split();
            tx.send(WsMessage::Text(r#"{"up":9,"down":8}"#.into()))
                .await
                .ok();
        });

        let client = MihomoClient::new(&format!("http://{}", addr), None).unwrap();
        let rx = client.stream_traffic().await.unwrap();
        drop(rx);
        tokio::time::sleep(std::time::Duration::from_millis(80)).await;
    }

    #[tokio::test]
    async fn test_stream_traffic_https_scheme_branch() {
        let client = MihomoClient::new("https://127.0.0.1:65534", None).unwrap();
        let _rx = client.stream_traffic().await.unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
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
    async fn test_stream_connections_ignores_non_text_and_handles_close() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let ws = accept_async(stream).await.unwrap();
            let (mut tx, _) = ws.split();
            tx.send(WsMessage::Binary(vec![7].into())).await.ok();
            tx.send(WsMessage::Close(None)).await.ok();
        });

        let client = MihomoClient::new(&format!("http://{}", addr), None).unwrap();
        let mut rx = client.stream_connections().await.unwrap();
        let recv = tokio::time::timeout(std::time::Duration::from_millis(200), rx.recv()).await;
        assert!(recv.is_err() || recv.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_stream_connections_drop_receiver_covers_send_error_branch() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let ws = accept_async(stream).await.unwrap();
            let (mut tx, _) = ws.split();
            tx.send(WsMessage::Text(
                r#"{"connections":[],"downloadTotal":9,"uploadTotal":8}"#.into(),
            ))
            .await
            .ok();
        });

        let client = MihomoClient::new(&format!("http://{}", addr), None).unwrap();
        let rx = client.stream_connections().await.unwrap();
        drop(rx);
        tokio::time::sleep(std::time::Duration::from_millis(80)).await;
    }

    #[tokio::test]
    async fn test_stream_connections_https_scheme_branch() {
        let client = MihomoClient::new("https://127.0.0.1:65534", None).unwrap();
        let _rx = client.stream_connections().await.unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }

    #[tokio::test]
    #[cfg(unix)]
    async fn test_unix_stream_logs_message_handling_with_level() {
        let temp = tempdir().unwrap();
        let socket_path = temp.path().join("mihomo-logs.sock");
        let listener = UnixListener::bind(&socket_path).unwrap();

        tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let ws = accept_async(stream).await.unwrap();
            let (mut tx, _) = ws.split();
            tx.send(WsMessage::Text("unix log".into())).await.ok();
        });

        let client = MihomoClient::new(socket_path.to_str().unwrap(), None).unwrap();
        let mut rx = client.stream_logs(Some("debug")).await.unwrap();
        let got = tokio::time::timeout(std::time::Duration::from_secs(1), rx.recv())
            .await
            .ok()
            .flatten();
        assert_eq!(got.as_deref(), Some("unix log"));
    }

    #[tokio::test]
    #[cfg(unix)]
    async fn test_unix_stream_logs_drop_receiver_covers_send_error_branch() {
        let temp = tempdir().unwrap();
        let socket_path = temp.path().join("mihomo-logs-drop.sock");
        let listener = UnixListener::bind(&socket_path).unwrap();

        tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let ws = accept_async(stream).await.unwrap();
            let (mut tx, _) = ws.split();
            tx.send(WsMessage::Text("will-drop".into())).await.ok();
        });

        let client = MihomoClient::new(socket_path.to_str().unwrap(), None).unwrap();
        let rx = client.stream_logs(None).await.unwrap();
        drop(rx);
        tokio::time::sleep(std::time::Duration::from_millis(80)).await;
    }

    #[tokio::test]
    #[cfg(unix)]
    async fn test_unix_stream_logs_ignores_non_text_frame() {
        let temp = tempdir().unwrap();
        let socket_path = temp.path().join("mihomo-logs-binary.sock");
        let listener = UnixListener::bind(&socket_path).unwrap();

        tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let ws = accept_async(stream).await.unwrap();
            let (mut tx, _) = ws.split();
            tx.send(WsMessage::Binary(vec![1, 2, 3].into())).await.ok();
            tx.send(WsMessage::Close(None)).await.ok();
        });

        let client = MihomoClient::new(socket_path.to_str().unwrap(), None).unwrap();
        let mut rx = client.stream_logs(None).await.unwrap();
        let recv = tokio::time::timeout(std::time::Duration::from_millis(200), rx.recv()).await;
        assert!(recv.is_err() || recv.unwrap().is_none());
    }

    #[tokio::test]
    #[cfg(unix)]
    async fn test_unix_stream_logs_handles_close_frame() {
        let temp = tempdir().unwrap();
        let socket_path = temp.path().join("mihomo-logs-close.sock");
        let listener = UnixListener::bind(&socket_path).unwrap();

        tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let ws = accept_async(stream).await.unwrap();
            let (mut tx, _) = ws.split();
            tx.send(WsMessage::Close(None)).await.ok();
        });

        let client = MihomoClient::new(socket_path.to_str().unwrap(), None).unwrap();
        let mut rx = client.stream_logs(None).await.unwrap();
        let recv = tokio::time::timeout(std::time::Duration::from_millis(200), rx.recv()).await;
        assert!(recv.is_err() || recv.unwrap().is_none());
    }

    #[tokio::test]
    #[cfg(unix)]
    async fn test_unix_stream_traffic_message_handling() {
        let temp = tempdir().unwrap();
        let socket_path = temp.path().join("mihomo-traffic.sock");
        let listener = UnixListener::bind(&socket_path).unwrap();

        tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let ws = accept_async(stream).await.unwrap();
            let (mut tx, _) = ws.split();
            tx.send(WsMessage::Text(r#"{"up":123,"down":456}"#.into()))
                .await
                .ok();
        });

        let client = MihomoClient::new(socket_path.to_str().unwrap(), None).unwrap();
        let mut rx = client.stream_traffic().await.unwrap();
        let got = tokio::time::timeout(std::time::Duration::from_secs(1), rx.recv())
            .await
            .ok()
            .flatten();
        assert!(got.is_some());
    }

    #[tokio::test]
    #[cfg(unix)]
    async fn test_unix_stream_traffic_drop_receiver_covers_send_error_branch() {
        let temp = tempdir().unwrap();
        let socket_path = temp.path().join("mihomo-traffic-drop.sock");
        let listener = UnixListener::bind(&socket_path).unwrap();

        tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let ws = accept_async(stream).await.unwrap();
            let (mut tx, _) = ws.split();
            tx.send(WsMessage::Text(r#"{"up":1,"down":2}"#.into()))
                .await
                .ok();
        });

        let client = MihomoClient::new(socket_path.to_str().unwrap(), None).unwrap();
        let rx = client.stream_traffic().await.unwrap();
        drop(rx);
        tokio::time::sleep(std::time::Duration::from_millis(80)).await;
    }

    #[tokio::test]
    #[cfg(unix)]
    async fn test_unix_stream_traffic_ignores_non_text_frame() {
        let temp = tempdir().unwrap();
        let socket_path = temp.path().join("mihomo-traffic-binary.sock");
        let listener = UnixListener::bind(&socket_path).unwrap();

        tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let ws = accept_async(stream).await.unwrap();
            let (mut tx, _) = ws.split();
            tx.send(WsMessage::Binary(vec![4, 5, 6].into())).await.ok();
            tx.send(WsMessage::Close(None)).await.ok();
        });

        let client = MihomoClient::new(socket_path.to_str().unwrap(), None).unwrap();
        let mut rx = client.stream_traffic().await.unwrap();
        let recv = tokio::time::timeout(std::time::Duration::from_millis(200), rx.recv()).await;
        assert!(recv.is_err() || recv.unwrap().is_none());
    }

    #[tokio::test]
    #[cfg(unix)]
    async fn test_unix_stream_traffic_handles_close_frame() {
        let temp = tempdir().unwrap();
        let socket_path = temp.path().join("mihomo-traffic-close.sock");
        let listener = UnixListener::bind(&socket_path).unwrap();

        tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let ws = accept_async(stream).await.unwrap();
            let (mut tx, _) = ws.split();
            tx.send(WsMessage::Close(None)).await.ok();
        });

        let client = MihomoClient::new(socket_path.to_str().unwrap(), None).unwrap();
        let mut rx = client.stream_traffic().await.unwrap();
        let recv = tokio::time::timeout(std::time::Duration::from_millis(200), rx.recv()).await;
        assert!(recv.is_err() || recv.unwrap().is_none());
    }

    #[tokio::test]
    #[cfg(unix)]
    async fn test_unix_stream_connections_message_handling() {
        let temp = tempdir().unwrap();
        let socket_path = temp.path().join("mihomo-connections.sock");
        let listener = UnixListener::bind(&socket_path).unwrap();

        tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let ws = accept_async(stream).await.unwrap();
            let (mut tx, _) = ws.split();
            tx.send(WsMessage::Text(
                r#"{"connections":[],"downloadTotal":11,"uploadTotal":22}"#.into(),
            ))
            .await
            .ok();
        });

        let client = MihomoClient::new(socket_path.to_str().unwrap(), None).unwrap();
        let mut rx = client.stream_connections().await.unwrap();
        let got = tokio::time::timeout(std::time::Duration::from_secs(1), rx.recv())
            .await
            .ok()
            .flatten();
        assert!(got.is_some());
    }

    #[tokio::test]
    #[cfg(unix)]
    async fn test_unix_stream_connections_drop_receiver_covers_send_error_branch() {
        let temp = tempdir().unwrap();
        let socket_path = temp.path().join("mihomo-connections-drop.sock");
        let listener = UnixListener::bind(&socket_path).unwrap();

        tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let ws = accept_async(stream).await.unwrap();
            let (mut tx, _) = ws.split();
            tx.send(WsMessage::Text(
                r#"{"connections":[],"downloadTotal":1,"uploadTotal":2}"#.into(),
            ))
            .await
            .ok();
        });

        let client = MihomoClient::new(socket_path.to_str().unwrap(), None).unwrap();
        let rx = client.stream_connections().await.unwrap();
        drop(rx);
        tokio::time::sleep(std::time::Duration::from_millis(80)).await;
    }

    #[tokio::test]
    #[cfg(unix)]
    async fn test_unix_stream_connections_ignores_non_text_frame() {
        let temp = tempdir().unwrap();
        let socket_path = temp.path().join("mihomo-connections-binary.sock");
        let listener = UnixListener::bind(&socket_path).unwrap();

        tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let ws = accept_async(stream).await.unwrap();
            let (mut tx, _) = ws.split();
            tx.send(WsMessage::Binary(vec![7, 8, 9].into())).await.ok();
            tx.send(WsMessage::Close(None)).await.ok();
        });

        let client = MihomoClient::new(socket_path.to_str().unwrap(), None).unwrap();
        let mut rx = client.stream_connections().await.unwrap();
        let recv = tokio::time::timeout(std::time::Duration::from_millis(200), rx.recv()).await;
        assert!(recv.is_err() || recv.unwrap().is_none());
    }

    #[tokio::test]
    #[cfg(unix)]
    async fn test_unix_stream_connections_handles_close_frame() {
        let temp = tempdir().unwrap();
        let socket_path = temp.path().join("mihomo-connections-close.sock");
        let listener = UnixListener::bind(&socket_path).unwrap();

        tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let ws = accept_async(stream).await.unwrap();
            let (mut tx, _) = ws.split();
            tx.send(WsMessage::Close(None)).await.ok();
        });

        let client = MihomoClient::new(socket_path.to_str().unwrap(), None).unwrap();
        let mut rx = client.stream_connections().await.unwrap();
        let recv = tokio::time::timeout(std::time::Duration::from_millis(200), rx.recv()).await;
        assert!(recv.is_err() || recv.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_http_request_rejects_unsupported_method() {
        let client = MihomoClient::new("http://127.0.0.1:9090", None).unwrap();
        let err = client
            .http_request("POST", "/version", None, None)
            .await
            .expect_err("unsupported method should fail");
        assert!(matches!(err, crate::core::MihomoError::Config(_)));
    }

    #[tokio::test]
    #[cfg(unix)]
    async fn test_unix_http_get_version() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let temp = tempdir().unwrap();
        let socket_path = temp.path().join("mihomo.sock");
        let listener = UnixListener::bind(&socket_path).unwrap();

        tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut buf = vec![0u8; 2048];
            let n = stream.read(&mut buf).await.unwrap();
            let req = String::from_utf8_lossy(&buf[..n]);
            assert!(req.starts_with("GET /version HTTP/1.1"));

            let resp = "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 49\r\n\r\n{\"version\":\"v1.18.0\",\"premium\":true,\"meta\":true}";
            stream.write_all(resp.as_bytes()).await.unwrap();
        });

        let client = MihomoClient::new(socket_path.to_str().unwrap(), None).unwrap();
        let version = client.get_version().await.unwrap();
        assert_eq!(version.version, "v1.18.0");
        assert!(version.premium);
        assert!(version.meta);
    }

    #[tokio::test]
    #[cfg(unix)]
    async fn test_unix_http_delay_query_building() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let temp = tempdir().unwrap();
        let socket_path = temp.path().join("mihomo-delay.sock");
        let listener = UnixListener::bind(&socket_path).unwrap();

        tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut buf = vec![0u8; 4096];
            let n = stream.read(&mut buf).await.unwrap();
            let req = String::from_utf8_lossy(&buf[..n]);
            assert!(req.starts_with("GET /proxies/GLOBAL/delay?"));
            assert!(req.contains("timeout=1234"));
            assert!(req.contains("url=http://example.com"));

            let resp = "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 13\r\n\r\n{\"delay\":42}";
            stream.write_all(resp.as_bytes()).await.unwrap();
        });

        let client = MihomoClient::new(socket_path.to_str().unwrap(), None).unwrap();
        let delay = client
            .test_delay("GLOBAL", "http://example.com", 1234)
            .await
            .unwrap();
        assert_eq!(delay, 42);
    }

    #[tokio::test]
    #[cfg(unix)]
    async fn test_unix_http_put_with_auth_body() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let temp = tempdir().unwrap();
        let socket_path = temp.path().join("mihomo.sock");
        let listener = UnixListener::bind(&socket_path).unwrap();

        tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut buf = vec![0u8; 4096];
            let n = stream.read(&mut buf).await.unwrap();
            let req = String::from_utf8_lossy(&buf[..n]);
            assert!(req.starts_with("PUT /proxies/GLOBAL HTTP/1.1"));
            assert!(req.contains("Authorization: Bearer unix-secret"));
            assert!(req.contains("{\"name\":\"DIRECT\"}"));

            let resp = "HTTP/1.1 204 No Content\r\nContent-Length: 0\r\n\r\n";
            stream.write_all(resp.as_bytes()).await.unwrap();
        });

        let client = MihomoClient::new(
            socket_path.to_str().unwrap(),
            Some("unix-secret".to_string()),
        )
        .unwrap();
        client.switch_proxy("GLOBAL", "DIRECT").await.unwrap();
    }

    #[tokio::test]
    #[cfg(unix)]
    async fn test_unix_http_invalid_response_body() {
        use tokio::io::AsyncWriteExt;

        let temp = tempdir().unwrap();
        let socket_path = temp.path().join("mihomo.sock");
        let listener = UnixListener::bind(&socket_path).unwrap();

        tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            stream.write_all(b"not-http").await.unwrap();
        });

        let client = MihomoClient::new(socket_path.to_str().unwrap(), None).unwrap();
        let err = client
            .get_version()
            .await
            .expect_err("invalid HTTP expected");
        assert!(matches!(err, crate::core::MihomoError::Config(_)));
    }
}
