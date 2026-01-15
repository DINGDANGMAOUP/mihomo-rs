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
        let transport = if base_url.starts_with('/') || base_url.starts_with("unix://") {
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
        use tokio::net::UnixStream;

        let mut stream = UnixStream::connect(socket_path).await?;

        let query_str = query
            .map(|q| {
                let params: Vec<String> = q.iter().map(|(k, v)| format!("{}={}", k, v)).collect();
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
                    use tokio::net::UnixStream;
                    use tokio_tungstenite::client_async;

                    if let Ok(stream) = UnixStream::connect(&socket_path).await {
                        let mut path = "/logs".to_string();
                        if let Some(l) = level {
                            path.push_str(&format!("?level={}", l));
                        }

                        let request = format!(
                            "GET {} HTTP/1.1\r\n\
                             Host: localhost\r\n\
                             Upgrade: websocket\r\n\
                             Connection: Upgrade\r\n\
                             Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\n\
                             Sec-WebSocket-Version: 13\r\n\r\n",
                            path
                        );

                        if let Ok((ws_stream, _)) = client_async(request, stream).await {
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
                    use tokio::net::UnixStream;
                    use tokio_tungstenite::client_async;

                    if let Ok(stream) = UnixStream::connect(&socket_path).await {
                        let request = "GET /traffic HTTP/1.1\r\n\
                             Host: localhost\r\n\
                             Upgrade: websocket\r\n\
                             Connection: Upgrade\r\n\
                             Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\n\
                             Sec-WebSocket-Version: 13\r\n\r\n";

                        if let Ok((ws_stream, _)) = client_async(request, stream).await {
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
                    use tokio::net::UnixStream;
                    use tokio_tungstenite::client_async;

                    if let Ok(stream) = UnixStream::connect(&socket_path).await {
                        let request = "GET /connections HTTP/1.1\r\n\
                             Host: localhost\r\n\
                             Upgrade: websocket\r\n\
                             Connection: Upgrade\r\n\
                             Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\n\
                             Sec-WebSocket-Version: 13\r\n\r\n";

                        if let Ok((ws_stream, _)) = client_async(request, stream).await {
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
                });
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
