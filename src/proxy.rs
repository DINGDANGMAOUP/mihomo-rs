//! 代理核心功能模块
//!
//! 提供代理服务器管理、连接处理和代理选择功能。

use crate::client::MihomoClient;
use crate::error::{MihomoError, Result};
use crate::types::*;
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// 代理管理器
#[derive(Debug, Clone)]
pub struct ProxyManager {
    /// mihomo 客户端
    client: MihomoClient,
    /// 代理节点缓存
    proxy_cache: HashMap<String, ProxyNode>,
    /// 代理组缓存
    group_cache: HashMap<String, ProxyGroup>,
    /// 缓存更新时间
    cache_updated_at: Option<Instant>,
    /// 缓存有效期（秒）
    cache_ttl: Duration,
}

impl ProxyManager {
    /// 创建新的代理管理器
    ///
    /// # Arguments
    ///
    /// * `client` - mihomo 客户端实例
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use mihomo_rs::{client::MihomoClient, proxy::ProxyManager};
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = MihomoClient::new("http://127.0.0.1:9090", None)?;
    /// let manager = ProxyManager::new(client);
    /// # Ok(())
    /// # }
    /// ```
    pub fn new(client: MihomoClient) -> Self {
        Self {
            client,
            proxy_cache: HashMap::new(),
            group_cache: HashMap::new(),
            cache_updated_at: None,
            cache_ttl: Duration::from_secs(30), // 默认缓存30秒
        }
    }

    /// 设置缓存有效期
    pub fn set_cache_ttl(&mut self, ttl: Duration) {
        self.cache_ttl = ttl;
    }

    /// 检查缓存是否有效
    fn is_cache_valid(&self) -> bool {
        if let Some(updated_at) = self.cache_updated_at {
            updated_at.elapsed() < self.cache_ttl
        } else {
            false
        }
    }

    /// 刷新代理缓存
    async fn refresh_cache(&mut self) -> Result<()> {
        log::debug!("Refreshing proxy cache");

        // 获取所有代理节点
        self.proxy_cache = self.client.proxies().await?;

        // 获取代理组信息
        self.group_cache = self.client.proxy_groups().await?;

        self.cache_updated_at = Some(Instant::now());

        log::debug!(
            "Proxy cache refreshed: {} proxies, {} groups",
            self.proxy_cache.len(),
            self.group_cache.len()
        );

        Ok(())
    }

    /// 确保缓存有效
    async fn ensure_cache(&mut self) -> Result<()> {
        if !self.is_cache_valid() {
            self.refresh_cache().await?
        }
        Ok(())
    }

    /// 获取所有代理节点
    pub async fn get_proxies(&mut self) -> Result<&HashMap<String, ProxyNode>> {
        self.ensure_cache().await?;
        Ok(&self.proxy_cache)
    }

    /// 获取指定代理节点
    pub async fn get_proxy(&mut self, name: &str) -> Result<Option<&ProxyNode>> {
        self.ensure_cache().await?;
        Ok(self.proxy_cache.get(name))
    }

    /// 获取所有代理组
    pub async fn get_proxy_groups(&mut self) -> Result<&HashMap<String, ProxyGroup>> {
        self.ensure_cache().await?;
        Ok(&self.group_cache)
    }

    /// 获取指定代理组
    pub async fn get_proxy_group(&mut self, name: &str) -> Result<Option<&ProxyGroup>> {
        self.ensure_cache().await?;
        Ok(self.group_cache.get(name))
    }

    /// 切换代理组选择
    ///
    /// # Arguments
    ///
    /// * `group_name` - 代理组名称
    /// * `proxy_name` - 要选择的代理名称
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use mihomo_rs::{client::MihomoClient, proxy::ProxyManager};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let client = MihomoClient::new("http://127.0.0.1:9090", None)?;
    /// # let mut manager = ProxyManager::new(client);
    /// manager.switch_proxy("Proxy", "HK-01").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn switch_proxy(&mut self, group_name: &str, proxy_name: &str) -> Result<()> {
        // 验证代理组是否存在
        self.ensure_cache().await?;

        if !self.group_cache.contains_key(group_name) {
            return Err(MihomoError::proxy(format!(
                "Proxy group '{}' not found",
                group_name
            )));
        }

        // 验证代理是否存在于组中
        let group = &self.group_cache[group_name];
        if !group.all.contains(&proxy_name.to_string()) {
            return Err(MihomoError::proxy(format!(
                "Proxy '{}' not found in group '{}'",
                proxy_name, group_name
            )));
        }

        // 执行切换
        self.client.switch_proxy(group_name, proxy_name).await?;

        // 更新缓存中的当前选择
        if let Some(group) = self.group_cache.get_mut(group_name) {
            group.now = proxy_name.to_string();
        }

        log::info!("Switched proxy group '{}' to '{}'", group_name, proxy_name);
        Ok(())
    }

    /// 测试代理延迟
    ///
    /// # Arguments
    ///
    /// * `proxy_name` - 代理名称
    /// * `test_url` - 测试URL（可选，默认使用系统配置）
    /// * `timeout` - 超时时间（毫秒，可选）
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use mihomo_rs::{client::MihomoClient, proxy::ProxyManager};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let client = MihomoClient::new("http://127.0.0.1:9090", None)?;
    /// # let manager = ProxyManager::new(client);
    /// let delay = manager.test_proxy_delay("HK-01", None, Some(5000)).await?;
    /// println!("Delay: {}ms", delay.delay);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn test_proxy_delay(
        &self,
        proxy_name: &str,
        test_url: Option<&str>,
        timeout: Option<u32>,
    ) -> Result<DelayHistory> {
        self.client
            .test_proxy_delay(proxy_name, test_url, timeout)
            .await
    }

    /// 批量测试代理延迟
    ///
    /// # Arguments
    ///
    /// * `proxy_names` - 代理名称列表
    /// * `test_url` - 测试URL（可选）
    /// * `timeout` - 超时时间（毫秒，可选）
    ///
    /// # Returns
    ///
    /// 返回代理名称到延迟结果的映射
    pub async fn test_multiple_proxy_delays(
        &self,
        proxy_names: &[String],
        test_url: Option<&str>,
        timeout: Option<u32>,
    ) -> HashMap<String, Result<DelayHistory>> {
        let mut results = HashMap::new();

        // 并发测试所有代理
        let tasks: Vec<_> = proxy_names
            .iter()
            .map(|name| {
                let client = self.client.clone();
                let name = name.clone();
                let test_url = test_url.map(|s| s.to_string());

                tokio::spawn(async move {
                    let result = client
                        .test_proxy_delay(&name, test_url.as_deref(), timeout)
                        .await;
                    (name, result)
                })
            })
            .collect();

        // 等待所有任务完成
        for task in tasks {
            if let Ok((name, result)) = task.await {
                results.insert(name, result);
            }
        }

        results
    }

    /// 自动选择最快的代理
    ///
    /// # Arguments
    ///
    /// * `group_name` - 代理组名称
    /// * `test_url` - 测试URL（可选）
    /// * `timeout` - 超时时间（毫秒，可选）
    ///
    /// # Returns
    ///
    /// 返回选中的代理名称和延迟信息
    pub async fn auto_select_fastest_proxy(
        &mut self,
        group_name: &str,
        test_url: Option<&str>,
        timeout: Option<u32>,
    ) -> Result<(String, DelayHistory)> {
        self.ensure_cache().await?;

        let group = self
            .group_cache
            .get(group_name)
            .ok_or_else(|| MihomoError::proxy(format!("Proxy group '{}' not found", group_name)))?;

        if group.all.is_empty() {
            return Err(MihomoError::proxy(format!(
                "Proxy group '{}' is empty",
                group_name
            )));
        }

        log::info!(
            "Testing {} proxies in group '{}'",
            group.all.len(),
            group_name
        );

        // 测试所有代理的延迟
        let delay_results = self
            .test_multiple_proxy_delays(&group.all, test_url, timeout)
            .await;

        // 找到延迟最小的代理
        let mut best_proxy: Option<(String, DelayHistory)> = None;

        for (proxy_name, result) in delay_results {
            if let Ok(delay_history) = result {
                if let Some((_, ref current_best)) = best_proxy {
                    if delay_history.delay < current_best.delay {
                        best_proxy = Some((proxy_name, delay_history));
                    }
                } else {
                    best_proxy = Some((proxy_name, delay_history));
                }
            } else {
                log::warn!("Failed to test proxy '{}': {:?}", proxy_name, result);
            }
        }

        let (best_proxy_name, best_delay) =
            best_proxy.ok_or_else(|| MihomoError::proxy("No available proxy found"))?;

        // 切换到最快的代理
        self.switch_proxy(group_name, &best_proxy_name).await?;

        log::info!(
            "Auto-selected proxy '{}' with delay {}ms",
            best_proxy_name,
            best_delay.delay
        );

        Ok((best_proxy_name, best_delay))
    }

    /// 获取代理统计信息
    pub async fn get_proxy_stats(&mut self) -> Result<ProxyStats> {
        self.ensure_cache().await?;

        let total_proxies = self.proxy_cache.len();
        let total_groups = self.group_cache.len();

        // 统计各种代理类型
        let mut type_counts = HashMap::new();
        for proxy in self.proxy_cache.values() {
            *type_counts.entry(proxy.proxy_type.clone()).or_insert(0) += 1;
        }

        // 统计有延迟信息的代理数量
        let proxies_with_delay = self
            .proxy_cache
            .values()
            .filter(|p| p.delay.is_some())
            .count();

        Ok(ProxyStats {
            total_proxies,
            total_groups,
            proxies_with_delay,
            type_counts,
        })
    }

    /// 强制刷新缓存
    pub async fn force_refresh(&mut self) -> Result<()> {
        self.cache_updated_at = None;
        self.refresh_cache().await
    }
}

/// 代理统计信息
#[derive(Debug, Clone)]
pub struct ProxyStats {
    /// 总代理数量
    pub total_proxies: usize,
    /// 总代理组数量
    pub total_groups: usize,
    /// 有延迟信息的代理数量
    pub proxies_with_delay: usize,
    /// 各类型代理数量统计
    pub type_counts: HashMap<ProxyType, usize>,
}

/// 代理选择器
#[derive(Debug)]
pub struct ProxySelector {
    /// 代理管理器
    manager: ProxyManager,
}

impl ProxySelector {
    /// 创建新的代理选择器
    pub fn new(manager: ProxyManager) -> Self {
        Self { manager }
    }

    /// 根据延迟选择代理
    ///
    /// # Arguments
    ///
    /// * `group_name` - 代理组名称
    /// * `max_delay` - 最大允许延迟（毫秒）
    ///
    /// # Returns
    ///
    /// 返回符合条件的代理列表，按延迟排序
    pub async fn select_by_delay(
        &mut self,
        group_name: &str,
        max_delay: u32,
    ) -> Result<Vec<(String, u32)>> {
        let group = {
            let group = self
                .manager
                .get_proxy_group(group_name)
                .await?
                .ok_or_else(|| {
                    MihomoError::proxy(format!("Proxy group '{}' not found", group_name))
                })?;
            group.clone()
        };

        let mut candidates = Vec::new();

        for proxy_name in &group.all {
            if let Some(proxy) = self.manager.get_proxy(proxy_name).await? {
                if let Some(delay) = proxy.delay {
                    if delay <= max_delay {
                        candidates.push((proxy_name.clone(), delay));
                    }
                }
            }
        }

        // 按延迟排序
        candidates.sort_by_key(|(_, delay)| *delay);

        Ok(candidates)
    }

    /// 根据地区选择代理
    ///
    /// # Arguments
    ///
    /// * `group_name` - 代理组名称
    /// * `region_keywords` - 地区关键字列表
    ///
    /// # Returns
    ///
    /// 返回包含指定地区关键字的代理列表
    pub async fn select_by_region(
        &mut self,
        group_name: &str,
        region_keywords: &[&str],
    ) -> Result<Vec<String>> {
        let group = {
            let group = self
                .manager
                .get_proxy_group(group_name)
                .await?
                .ok_or_else(|| {
                    MihomoError::proxy(format!("Proxy group '{}' not found", group_name))
                })?;
            group.clone()
        };

        let mut candidates = Vec::new();

        for proxy_name in &group.all {
            for keyword in region_keywords {
                if proxy_name.to_lowercase().contains(&keyword.to_lowercase()) {
                    candidates.push(proxy_name.clone());
                    break;
                }
            }
        }

        Ok(candidates)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MihomoClient;

    #[test]
    fn test_proxy_manager_creation() {
        let client = MihomoClient::new("http://127.0.0.1:9090", None).unwrap();
        let manager = ProxyManager::new(client);
        assert_eq!(manager.cache_ttl, Duration::from_secs(30));
    }

    #[test]
    fn test_cache_validity() {
        let client = MihomoClient::new("http://127.0.0.1:9090", None).unwrap();
        let manager = ProxyManager::new(client);
        assert!(!manager.is_cache_valid()); // 初始状态缓存无效
    }

    #[test]
    fn test_proxy_stats_creation() {
        let stats = ProxyStats {
            total_proxies: 10,
            total_groups: 3,
            proxies_with_delay: 8,
            type_counts: HashMap::new(),
        };

        assert_eq!(stats.total_proxies, 10);
        assert_eq!(stats.total_groups, 3);
    }
}
