//! 客户端模块
//!
//! 提供与 mihomo API 通信的核心客户端功能。

use crate::error::{MihomoError, Result};
use crate::retry::{RetryExecutor, RetryPolicy};
use crate::types::*;
use futures_util::stream::StreamExt;
use reqwest::Client;
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
    /// 重试执行器
    retry_executor: RetryExecutor,
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

        let retry_policy = RetryPolicy::new(3)
            .with_initial_delay(std::time::Duration::from_millis(500))
            .with_max_delay(std::time::Duration::from_secs(10));
        let retry_executor = RetryExecutor::new(retry_policy);

        Ok(Self {
            client,
            base_url,
            secret,
            retry_executor,
        })
    }

    /// 创建带自定义重试策略的客户端实例
    ///
    /// # Arguments
    ///
    /// * `base_url` - mihomo 服务的基础 URL
    /// * `secret` - API 访问密钥（可选）
    /// * `retry_policy` - 重试策略
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use mihomo_rs::client::MihomoClient;
    /// # use mihomo_rs::retry::RetryPolicy;
    /// # use std::time::Duration;
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let retry_policy = RetryPolicy::new(5)
    ///     .with_initial_delay(Duration::from_millis(1000));
    /// let client = MihomoClient::with_retry_policy(
    ///     "http://127.0.0.1:9090",
    ///     Some("your-secret".to_string()),
    ///     retry_policy
    /// )?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn with_retry_policy(
        base_url: &str,
        secret: Option<String>,
        retry_policy: RetryPolicy,
    ) -> Result<Self> {
        let base_url = Url::parse(base_url)
            .map_err(|e| MihomoError::invalid_parameter(format!("Invalid base URL: {}", e)))?;

        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| MihomoError::network(format!("Failed to create HTTP client: {}", e)))?;

        let retry_executor = RetryExecutor::new(retry_policy);

        Ok(Self {
            client,
            base_url,
            secret,
            retry_executor,
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
        let client = self.client.clone();
        let secret = self.secret.clone();

        self.retry_executor
            .execute(move || {
                let client = client.clone();
                let url = url.clone();
                let secret = secret.clone();

                async move {
                    let mut request = client.get(url);

                    if let Some(ref secret) = secret {
                        request = request.header("Authorization", format!("Bearer {}", secret));
                    }

                    let response = request
                        .send()
                        .await
                        .map_err(|e| MihomoError::network(format!("HTTP请求失败: {}", e)))?;

                    if response.status().is_success() {
                        let text = response
                            .text()
                            .await
                            .map_err(|e| MihomoError::network(format!("读取响应失败: {}", e)))?;
                        if text.is_empty() {
                            serde_json::from_str("{}").map_err(MihomoError::Json)
                        } else {
                            serde_json::from_str(&text).map_err(MihomoError::Json)
                        }
                    } else {
                        let status = response.status();
                        let text = response.text().await.unwrap_or_default();
                        Err(MihomoError::network(format!(
                            "API请求失败: {} - {}",
                            status, text
                        )))
                    }
                }
            })
            .await
    }

    /// 发送 POST 请求
    #[allow(dead_code)]
    async fn post<T, B>(&self, path: &str, body: &B) -> Result<T>
    where
        T: DeserializeOwned,
        B: serde::Serialize,
    {
        let url = self.build_url(path)?;
        let body_json = serde_json::to_value(body).map_err(MihomoError::Json)?;
        let client = self.client.clone();
        let secret = self.secret.clone();

        self.retry_executor
            .execute(move || {
                let client = client.clone();
                let url = url.clone();
                let secret = secret.clone();
                let body_json = body_json.clone();

                async move {
                    let mut request = client.post(url).json(&body_json);

                    if let Some(ref secret) = secret {
                        request = request.header("Authorization", format!("Bearer {}", secret));
                    }

                    let response = request
                        .send()
                        .await
                        .map_err(|e| MihomoError::network(format!("HTTP请求失败: {}", e)))?;

                    if response.status().is_success() {
                        let text = response
                            .text()
                            .await
                            .map_err(|e| MihomoError::network(format!("读取响应失败: {}", e)))?;
                        if text.is_empty() {
                            serde_json::from_str("{}").map_err(MihomoError::Json)
                        } else {
                            serde_json::from_str(&text).map_err(MihomoError::Json)
                        }
                    } else {
                        let status = response.status();
                        let text = response.text().await.unwrap_or_default();
                        Err(MihomoError::network(format!(
                            "API请求失败: {} - {}",
                            status, text
                        )))
                    }
                }
            })
            .await
    }

    /// 发送 PUT 请求
    async fn put<T, B>(&self, path: &str, body: &B) -> Result<T>
    where
        T: DeserializeOwned,
        B: serde::Serialize,
    {
        let url = self.build_url(path)?;
        let body_json = serde_json::to_value(body).map_err(MihomoError::Json)?;
        let client = self.client.clone();
        let secret = self.secret.clone();

        self.retry_executor
            .execute(move || {
                let client = client.clone();
                let url = url.clone();
                let secret = secret.clone();
                let body_json = body_json.clone();

                async move {
                    let mut request = client.put(url).json(&body_json);

                    if let Some(ref secret) = secret {
                        request = request.header("Authorization", format!("Bearer {}", secret));
                    }

                    let response = request
                        .send()
                        .await
                        .map_err(|e| MihomoError::network(format!("HTTP请求失败: {}", e)))?;

                    if response.status().is_success() {
                        let text = response
                            .text()
                            .await
                            .map_err(|e| MihomoError::network(format!("读取响应失败: {}", e)))?;
                        if text.is_empty() {
                            serde_json::from_str("{}").map_err(MihomoError::Json)
                        } else {
                            serde_json::from_str(&text).map_err(MihomoError::Json)
                        }
                    } else {
                        let status = response.status();
                        let text = response.text().await.unwrap_or_default();
                        Err(MihomoError::network(format!(
                            "API请求失败: {} - {}",
                            status, text
                        )))
                    }
                }
            })
            .await
    }

    /// 发送 DELETE 请求
    async fn delete<T>(&self, path: &str) -> Result<T>
    where
        T: DeserializeOwned + Default,
    {
        let url = self.build_url(path)?;
        let client = self.client.clone();
        let secret = self.secret.clone();

        self.retry_executor
            .execute(move || {
                let client = client.clone();
                let url = url.clone();
                let secret = secret.clone();

                async move {
                    let mut request = client.delete(url);

                    if let Some(ref secret) = secret {
                        request = request.header("Authorization", format!("Bearer {}", secret));
                    }

                    let response = request
                        .send()
                        .await
                        .map_err(|e| MihomoError::network(format!("HTTP请求失败: {}", e)))?;

                    if response.status().is_success() {
                        let text = response
                            .text()
                            .await
                            .map_err(|e| MihomoError::network(format!("读取响应失败: {}", e)))?;
                        if text.is_empty() {
                            Ok(T::default())
                        } else {
                            serde_json::from_str(&text).map_err(MihomoError::Json)
                        }
                    } else {
                        let status = response.status();
                        let text = response.text().await.unwrap_or_default();
                        Err(MihomoError::network(format!(
                            "API请求失败: {} - {}",
                            status, text
                        )))
                    }
                }
            })
            .await
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
        let reader = BufReader::new(StreamReader::new(
            stream.map(|result| result.map_err(std::io::Error::other)),
        ));

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
        let reader = BufReader::new(StreamReader::new(
            stream.map(|result| result.map_err(std::io::Error::other)),
        ));

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

    /// 获取系统日志
    pub async fn get_logs(&self, level: Option<&str>) -> Result<Vec<LogEntry>> {
        let path = match level {
            Some(level) => format!("/logs?level={}", level),
            None => "/logs".to_string(),
        };
        self.get(&path).await
    }

    /// 获取代理提供者信息
    pub async fn get_providers(&self) -> Result<HashMap<String, Provider>> {
        self.get("/providers/proxies").await
    }

    /// 更新代理提供者
    pub async fn update_provider(&self, provider_name: &str) -> Result<EmptyResponse> {
        self.put(
            &format!("/providers/proxies/{}", provider_name),
            &serde_json::json!({}),
        )
        .await
    }

    /// 健康检查代理提供者
    pub async fn health_check_provider(&self, provider_name: &str) -> Result<EmptyResponse> {
        self.get(&format!("/providers/proxies/{}/healthcheck", provider_name))
            .await
    }

    /// 获取DNS查询记录
    pub async fn get_dns_queries(&self) -> Result<Vec<DnsQuery>> {
        self.get("/dns/query").await
    }

    /// 刷新DNS缓存
    pub async fn flush_dns_cache(&self) -> Result<EmptyResponse> {
        self.delete("/dns/cache").await
    }

    /// 获取规则提供者列表
    pub async fn get_rule_providers(&self) -> Result<HashMap<String, RuleProvider>> {
        self.get("/providers/rules").await
    }

    /// 更新规则提供者
    pub async fn update_rule_provider(&self, provider_name: &str) -> Result<EmptyResponse> {
        self.put(
            &format!("/providers/rules/{}", provider_name),
            &serde_json::json!({}),
        )
        .await
    }

    /// 健康检查规则提供者
    pub async fn health_check_rule_provider(&self, provider_name: &str) -> Result<EmptyResponse> {
        self.get(&format!("/providers/rules/{}/healthcheck", provider_name))
            .await
    }

    // ===== 服务管理 API =====

    /// 获取服务版本信息
    pub async fn get_version(&self) -> Result<VersionInfo> {
        self.get("/version").await
    }

    /// 获取服务运行时信息
    pub async fn get_runtime_info(&self) -> Result<RuntimeInfo> {
        self.get("/runtime").await
    }

    /// 重启服务
    pub async fn restart_service(&self) -> Result<EmptyResponse> {
        self.post("/restart", &serde_json::json!({})).await
    }

    /// 停止服务
    pub async fn shutdown_service(&self) -> Result<EmptyResponse> {
        self.post("/shutdown", &serde_json::json!({})).await
    }

    /// 获取服务配置
    pub async fn get_service_config(&self) -> Result<ServiceConfigInfo> {
        self.get("/configs").await
    }

    /// 更新服务配置
    pub async fn update_service_config(
        &self,
        config: &ServiceConfigUpdate,
    ) -> Result<EmptyResponse> {
        self.put("/configs", config).await
    }

    /// 重新加载服务配置
    pub async fn reload_service_config(&self) -> Result<EmptyResponse> {
        self.put("/configs/reload", &serde_json::json!({})).await
    }

    /// 获取服务统计信息
    pub async fn get_service_stats(&self) -> Result<ServiceStats> {
        self.get("/stats").await
    }

    /// 清理服务缓存
    pub async fn clear_service_cache(&self) -> Result<EmptyResponse> {
        self.delete("/cache").await
    }

    /// 获取服务内存使用情况
    pub async fn get_memory_usage(&self) -> Result<MemoryUsage> {
        self.get("/memory").await
    }

    /// 强制垃圾回收
    pub async fn force_gc(&self) -> Result<EmptyResponse> {
        self.post("/gc", &serde_json::json!({})).await
    }

    /// 获取服务日志
    pub async fn get_service_logs(
        &self,
        level: Option<&str>,
        lines: Option<u32>,
    ) -> Result<Vec<LogEntry>> {
        let mut url = "/logs".to_string();
        let mut params = Vec::new();

        if let Some(level) = level {
            params.push(format!("level={}", level));
        }
        if let Some(lines) = lines {
            params.push(format!("lines={}", lines));
        }

        if !params.is_empty() {
            url.push('?');
            url.push_str(&params.join("&"));
        }

        self.get(&url).await
    }

    /// 设置日志级别
    pub async fn set_log_level(&self, level: LogLevel) -> Result<EmptyResponse> {
        self.put("/configs/log-level", &serde_json::json!({"level": level}))
            .await
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
