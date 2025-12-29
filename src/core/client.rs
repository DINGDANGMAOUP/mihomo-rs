use super::error::Result;
use super::types::*;
use futures_util::StreamExt;
use reqwest::Client;
use serde_json::json;
use std::collections::HashMap;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use url::Url;

#[derive(Clone)]
pub struct MihomoClient {
    client: Client,
    base_url: Url,
    secret: Option<String>,
}

impl MihomoClient {
    pub fn new(base_url: &str, secret: Option<String>) -> Result<Self> {
        let base_url = Url::parse(base_url)?;
        let client = Client::new();
        Ok(Self {
            client,
            base_url,
            secret,
        })
    }

    fn build_url(&self, path: &str) -> Result<Url> {
        Ok(self.base_url.join(path)?)
    }

    fn add_auth(&self, mut req: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        if let Some(secret) = &self.secret {
            req = req.bearer_auth(secret);
        }
        req
    }

    pub async fn get_version(&self) -> Result<Version> {
        let url = self.build_url("/version")?;
        let req = self.client.get(url);
        let req = self.add_auth(req);
        let resp = req.send().await?;
        Ok(resp.json().await?)
    }

    pub async fn get_proxies(&self) -> Result<HashMap<String, ProxyInfo>> {
        let url = self.build_url("/proxies")?;
        log::debug!("Fetching proxies from: {}", url);
        let req = self.client.get(url);
        let req = self.add_auth(req);
        let resp = req.send().await?;
        let data: ProxiesResponse = resp.json().await?;
        log::debug!("Received {} proxies", data.proxies.len());
        Ok(data.proxies)
    }

    pub async fn get_proxy(&self, name: &str) -> Result<ProxyInfo> {
        let url = self.build_url(&format!("/proxies/{}", name))?;
        let req = self.client.get(url);
        let req = self.add_auth(req);
        let resp = req.send().await?;
        Ok(resp.json().await?)
    }

    pub async fn switch_proxy(&self, group: &str, proxy: &str) -> Result<()> {
        let url = self.build_url(&format!("/proxies/{}", group))?;
        log::debug!(
            "Switching group '{}' to proxy '{}' at {}",
            group,
            proxy,
            url
        );
        let req = self.client.put(url).json(&json!({ "name": proxy }));
        let req = self.add_auth(req);
        req.send().await?;
        log::debug!("Successfully switched group '{}' to '{}'", group, proxy);
        Ok(())
    }

    pub async fn test_delay(&self, proxy: &str, test_url: &str, timeout: u32) -> Result<u32> {
        let url = self.build_url(&format!("/proxies/{}/delay", proxy))?;
        let req = self.client.get(url).query(&[
            ("timeout", timeout.to_string()),
            ("url", test_url.to_string()),
        ]);
        let req = self.add_auth(req);
        let resp = req.send().await?;
        let data: DelayTestResponse = resp.json().await?;
        Ok(data.delay)
    }

    pub async fn reload_config(&self, path: Option<&str>) -> Result<()> {
        let url = self.build_url("/configs")?;
        let mut req = self.client.put(url);
        if let Some(path) = path {
            req = req
                .query(&[("force", "true")])
                .json(&json!({ "path": path }));
        } else {
            req = req.query(&[("force", "true")]);
        }
        let req = self.add_auth(req);
        req.send().await?;
        Ok(())
    }

    pub async fn stream_logs(
        &self,
        level: Option<&str>,
    ) -> Result<tokio::sync::mpsc::UnboundedReceiver<String>> {
        let mut ws_url = self.base_url.clone();
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

        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
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

        Ok(rx)
    }

    pub async fn stream_traffic(
        &self,
    ) -> Result<tokio::sync::mpsc::UnboundedReceiver<TrafficData>> {
        let mut ws_url = self.base_url.clone();
        ws_url
            .set_scheme(if ws_url.scheme() == "https" {
                "wss"
            } else {
                "ws"
            })
            .ok();
        ws_url.set_path("/traffic");

        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        let ws_url_str = ws_url.to_string();

        tokio::spawn(async move {
            if let Ok((ws_stream, _)) = connect_async(&ws_url_str).await {
                let (_, mut read) = ws_stream.split();
                while let Some(msg) = read.next().await {
                    match msg {
                        Ok(Message::Text(text)) => {
                            if let Ok(traffic) = serde_json::from_str::<TrafficData>(text.as_ref())
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

        Ok(rx)
    }

    pub async fn get_memory(&self) -> Result<MemoryData> {
        let url = self.build_url("/memory")?;
        let req = self.client.get(url);
        let req = self.add_auth(req);
        let resp = req.send().await?;
        Ok(resp.json().await?)
    }

    pub async fn get_connections(&self) -> Result<ConnectionsResponse> {
        let url = self.build_url("/connections")?;
        log::debug!("Fetching connections from: {}", url);
        let req = self.client.get(url);
        let req = self.add_auth(req);
        let resp = req.send().await?;
        let data: ConnectionsResponse = resp.json().await?;
        log::debug!("Received {} connections", data.connections.len());
        Ok(data)
    }

    pub async fn close_all_connections(&self) -> Result<()> {
        let url = self.build_url("/connections")?;
        log::debug!("Closing all connections at: {}", url);
        let req = self.client.delete(url);
        let req = self.add_auth(req);
        req.send().await?;
        log::debug!("Successfully closed all connections");
        Ok(())
    }

    pub async fn close_connection(&self, id: &str) -> Result<()> {
        let url = self.build_url(&format!("/connections/{}", id))?;
        log::debug!("Closing connection '{}' at: {}", id, url);
        let req = self.client.delete(url);
        let req = self.add_auth(req);
        req.send().await?;
        log::debug!("Successfully closed connection '{}'", id);
        Ok(())
    }

    pub async fn stream_connections(
        &self,
    ) -> Result<tokio::sync::mpsc::UnboundedReceiver<ConnectionSnapshot>> {
        let mut ws_url = self.base_url.clone();
        ws_url
            .set_scheme(if ws_url.scheme() == "https" {
                "wss"
            } else {
                "ws"
            })
            .ok();
        ws_url.set_path("/connections");

        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
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

        Ok(rx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::{Matcher, Server};

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
    fn test_build_url() {
        let client = MihomoClient::new("http://127.0.0.1:9090", None).unwrap();
        let url = client.build_url("/version");
        assert!(url.is_ok());
        assert_eq!(url.unwrap().as_str(), "http://127.0.0.1:9090/version");
    }

    #[test]
    fn test_build_url_with_path() {
        let client = MihomoClient::new("http://127.0.0.1:9090", None).unwrap();
        let url = client.build_url("/proxies/GLOBAL");
        assert!(url.is_ok());
        assert_eq!(
            url.unwrap().as_str(),
            "http://127.0.0.1:9090/proxies/GLOBAL"
        );
    }

    #[test]
    fn test_client_clone() {
        let client = MihomoClient::new("http://127.0.0.1:9090", None).unwrap();
        let cloned = client.clone();
        assert_eq!(client.base_url, cloned.base_url);
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
}
