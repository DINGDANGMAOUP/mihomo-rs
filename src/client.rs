//! 客户端模块
//!
//! 提供与 mihomo API 通信的核心客户端功能。

use crate::error::{MihomoError, Result};
use crate::types::*;
use futures_util::stream::StreamExt;
use reqwest::{Client, Response};
use serde::de::DeserializeOwned;
use std::collections::HashMap;
use std::pin::Pin;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio_util::io::StreamReader;
use url::Url;

/// Mihomo API 客户端
#[derive(Debug, Clone)]
pub struct MihomoClient {
    /// HTTP 客户端
    client: Client,
    /// 基础 URL
    base_url: Url,
    /// API 密钥
    secret: Option<String>,
}

impl MihomoClient {
    /// 创建新的客户端实例
    ///
    /// # Arguments
    ///
    /// * `base_url` - mihomo 服务的基础 URL
    /// * `secret` - API 访问密钥（可选）
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use mihomo_rs::client::MihomoClient;
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = MihomoClient::new("http://127.0.0.1:9090", Some("your-secret".to_string()))?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn new(base_url: &str, secret: Option<String>) -> Result<Self> {
        let base_url = Url::parse(base_url)
            .map_err(|e| MihomoError::invalid_parameter(format!("Invalid base URL: {}", e)))?;

        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| MihomoError::network(format!("Failed to create HTTP client: {}", e)))?;

        Ok(Self {
            client,
            base_url,
            secret,
        })
    }

    /// 构建完整的 API URL
    fn build_url(&self, path: &str) -> Result<Url> {
        self.base_url
            .join(path)
            .map_err(|e| MihomoError::invalid_parameter(format!("Invalid API path: {}", e)))
    }

    /// 发送 GET 请求
    async fn get<T>(&self, path: &str) -> Result<T>
    where
        T: DeserializeOwned,
    {
        let url = self.build_url(path)?;
        let mut request = self.client.get(url);

        if let Some(ref secret) = self.secret {
            request = request.header("Authorization", format!("Bearer {}", secret));
        }

        let response = request.send().await?;
        self.handle_response(response).await
    }

    /// 发送 POST 请求
    #[allow(dead_code)]
    async fn post<T, B>(&self, path: &str, body: &B) -> Result<T>
    where
        T: DeserializeOwned,
        B: serde::Serialize,
    {
        let url = self.build_url(path)?;
        let mut request = self.client.post(url).json(body);

        if let Some(ref secret) = self.secret {
            request = request.header("Authorization", format!("Bearer {}", secret));
        }

        let response = request.send().await?;
        self.handle_response(response).await
    }

    /// 发送 PUT 请求
    async fn put<T, B>(&self, path: &str, body: &B) -> Result<T>
    where
        T: DeserializeOwned,
        B: serde::Serialize,
    {
        let url = self.build_url(path)?;
        let mut request = self.client.put(url).json(body);

        if let Some(ref secret) = self.secret {
            request = request.header("Authorization", format!("Bearer {}", secret));
        }

        let response = request.send().await?;
        self.handle_response(response).await
    }

    /// 发送 DELETE 请求
    async fn delete<T>(&self, path: &str) -> Result<T>
    where
        T: DeserializeOwned,
    {
        let url = self.build_url(path)?;
        let mut request = self.client.delete(url);

        if let Some(ref secret) = self.secret {
            request = request.header("Authorization", format!("Bearer {}", secret));
        }

        let response = request.send().await?;
        self.handle_response(response).await
    }

    /// 处理 HTTP 响应
    async fn handle_response<T>(&self, response: Response) -> Result<T>
    where
        T: DeserializeOwned,
    {
        let status = response.status();

        if status.is_success() {
            let text = response.text().await?;
            if text.is_empty() {
                // 对于空响应，尝试反序列化为空对象
                serde_json::from_str("{}").map_err(MihomoError::Json)
            } else {
                serde_json::from_str(&text).map_err(MihomoError::Json)
            }
        } else {
            let error_text = response.text().await.unwrap_or_default();
            match status.as_u16() {
                401 => Err(MihomoError::auth("Unauthorized access")),
                403 => Err(MihomoError::auth("Forbidden access")),
                404 => Err(MihomoError::not_found("Resource not found")),
                500..=599 => Err(MihomoError::ServiceUnavailable(format!(
                    "Server error: {} - {}",
                    status, error_text
                ))),
                _ => Err(MihomoError::network(format!(
                    "HTTP error: {} - {}",
                    status, error_text
                ))),
            }
        }
    }

    /// 获取版本信息
    pub async fn version(&self) -> Result<Version> {
        self.get("/version").await
    }

    /// 获取所有代理节点
    pub async fn proxies(&self) -> Result<HashMap<String, ProxyNode>> {
        let response: HashMap<String, HashMap<String, ProxyItem>> = self.get("/proxies").await?;
        let proxies = response.get("proxies").cloned().unwrap_or_default();
        let mut result = HashMap::new();
        for (name, item) in proxies {
            if let Some(node) = item.to_proxy_node() {
                result.insert(name, node);
            }
        }
        Ok(result)
    }

    /// 获取代理组信息
    pub async fn proxy_groups(&self) -> Result<HashMap<String, ProxyGroup>> {
        let response: HashMap<String, HashMap<String, ProxyItem>> = self.get("/proxies").await?;
        let proxies = response.get("proxies").cloned().unwrap_or_default();
        let mut result = HashMap::new();
        for (name, item) in proxies {
            if let Some(group) = item.to_proxy_group() {
                result.insert(name, group);
            }
        }
        Ok(result)
    }

    /// 切换代理组选择
    pub async fn switch_proxy(&self, group_name: &str, proxy_name: &str) -> Result<EmptyResponse> {
        let body = serde_json::json!({
            "name": proxy_name
        });
        self.put(&format!("/proxies/{}", group_name), &body).await
    }

    /// 获取规则列表
    pub async fn rules(&self) -> Result<Vec<Rule>> {
        let response: HashMap<String, Vec<Rule>> = self.get("/rules").await?;
        Ok(response.get("rules").cloned().unwrap_or_default())
    }

    /// 获取连接列表
    pub async fn connections(&self) -> Result<Vec<Connection>> {
        let response: ConnectionsResponse = self.get("/connections").await?;
        Ok(response.connections.unwrap_or_default())
    }

    /// 获取连接详细信息（包含统计数据）
    pub async fn connections_with_stats(&self) -> Result<ConnectionsResponse> {
        self.get("/connections").await
    }

    /// 关闭指定连接
    pub async fn close_connection(&self, connection_id: &str) -> Result<EmptyResponse> {
        self.delete(&format!("/connections/{}", connection_id))
            .await
    }

    /// 关闭所有连接
    pub async fn close_all_connections(&self) -> Result<EmptyResponse> {
        self.delete("/connections").await
    }

    /// 获取流量统计流（持续监控）
    /// 注意：/traffic 接口是流式接口，建议使用此方法进行持续监控
    pub async fn traffic_stream(
        &self,
    ) -> Result<Pin<Box<dyn futures_util::Stream<Item = Result<Traffic>> + Send>>> {
        let url = self.build_url("/traffic")?;
        let mut request = self.client.get(url);

        if let Some(secret) = &self.secret {
            request = request.header("Authorization", format!("Bearer {}", secret));
        }

        let response = request.send().await?;

        if !response.status().is_success() {
            return Err(MihomoError::network(format!(
                "HTTP {} - {}",
                response.status().as_u16(),
                response.text().await.unwrap_or_default()
            )));
        }

        let stream = response.bytes_stream();
        let reader = BufReader::new(StreamReader::new(stream.map(|result| {
            result.map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
        })));

        Ok(Box::pin(futures_util::stream::unfold(
            reader,
            |mut reader| async move {
                let mut line = String::new();
                match reader.read_line(&mut line).await {
                    Ok(0) => None, // EOF
                    Ok(_) => {
                        let line = line.trim();
                        if line.is_empty() {
                            return Some((Err(MihomoError::internal("Empty line")), reader));
                        }
                        match serde_json::from_str::<Traffic>(line) {
                            Ok(traffic) => Some((Ok(traffic), reader)),
                            Err(e) => Some((Err(MihomoError::Json(e)), reader)),
                        }
                    }
                    Err(e) => Some((Err(MihomoError::internal(e.to_string())), reader)),
                }
            },
        )))
    }

    /// 获取内存使用情况流（持续监控）
    /// 注意：/memory 接口是流式接口，建议使用此方法进行持续监控
    pub async fn memory_stream(
        &self,
    ) -> Result<Pin<Box<dyn futures_util::Stream<Item = Result<Memory>> + Send>>> {
        let url = self.build_url("/memory")?;
        let mut request = self.client.get(url);

        if let Some(secret) = &self.secret {
            request = request.header("Authorization", format!("Bearer {}", secret));
        }

        let response = request.send().await?;

        if !response.status().is_success() {
            return Err(MihomoError::network(format!(
                "HTTP {} - {}",
                response.status().as_u16(),
                response.text().await.unwrap_or_default()
            )));
        }

        let stream = response.bytes_stream();
        let reader = BufReader::new(StreamReader::new(stream.map(|result| {
            result.map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
        })));

        Ok(Box::pin(futures_util::stream::unfold(
            reader,
            |mut reader| async move {
                let mut line = String::new();
                match reader.read_line(&mut line).await {
                    Ok(0) => None, // EOF
                    Ok(_) => {
                        let line = line.trim();
                        if line.is_empty() {
                            return Some((Err(MihomoError::internal("Empty line")), reader));
                        }
                        match serde_json::from_str::<Memory>(line) {
                            Ok(memory) => Some((Ok(memory), reader)),
                            Err(e) => Some((Err(MihomoError::Json(e)), reader)),
                        }
                    }
                    Err(e) => Some((Err(MihomoError::internal(e.to_string())), reader)),
                }
            },
        )))
    }

    /// 测试代理延迟
    pub async fn test_proxy_delay(
        &self,
        proxy_name: &str,
        test_url: Option<&str>,
        timeout: Option<u32>,
    ) -> Result<DelayHistory> {
        let mut query_params = vec![];

        if let Some(url) = test_url {
            query_params.push(format!("url={}", url));
        }

        if let Some(timeout_ms) = timeout {
            query_params.push(format!("timeout={}", timeout_ms));
        }

        let query_string = if query_params.is_empty() {
            String::new()
        } else {
            format!("?{}", query_params.join("&"))
        };

        let path = format!("/proxies/{}/delay{}", proxy_name, query_string);
        self.get(&path).await
    }

    /// 重新加载配置
    pub async fn reload_config(&self) -> Result<EmptyResponse> {
        self.put("/configs", &serde_json::json!({})).await
    }

    /// 更新配置
    pub async fn update_config(&self, config: &serde_json::Value) -> Result<EmptyResponse> {
        self.put("/configs", config).await
    }

    /// 获取当前配置
    pub async fn get_config(&self) -> Result<serde_json::Value> {
        self.get("/configs").await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = MihomoClient::new("http://127.0.0.1:9090", None);
        assert!(client.is_ok());
    }

    #[test]
    fn test_invalid_url() {
        let client = MihomoClient::new("invalid-url", None);
        assert!(client.is_err());
    }

    #[tokio::test]
    async fn test_build_url() {
        let client = MihomoClient::new("http://127.0.0.1:9090", None).unwrap();
        let url = client.build_url("/version").unwrap();
        assert_eq!(url.as_str(), "http://127.0.0.1:9090/version");
    }
}
