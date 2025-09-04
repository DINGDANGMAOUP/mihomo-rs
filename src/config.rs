//! 配置管理模块
//!
//! 提供 mihomo 配置文件的解析、验证和管理功能。

use crate::error::{MihomoError, Result};
use crate::types::ProxyType;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// mihomo 主配置结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// 端口配置
    pub port: u16,
    /// SOCKS5 端口
    #[serde(rename = "socks-port")]
    pub socks_port: u16,
    /// 重定向端口
    #[serde(rename = "redir-port", skip_serializing_if = "Option::is_none")]
    pub redir_port: Option<u16>,
    /// TProxy 端口
    #[serde(rename = "tproxy-port", skip_serializing_if = "Option::is_none")]
    pub tproxy_port: Option<u16>,
    /// 混合端口
    #[serde(rename = "mixed-port", skip_serializing_if = "Option::is_none")]
    pub mixed_port: Option<u16>,
    /// 允许局域网连接
    #[serde(rename = "allow-lan", default)]
    pub allow_lan: bool,
    /// 绑定地址
    #[serde(rename = "bind-address", default = "default_bind_address")]
    pub bind_address: String,
    /// 运行模式
    #[serde(default = "default_mode")]
    pub mode: String,
    /// 日志级别
    #[serde(rename = "log-level", default = "default_log_level")]
    pub log_level: String,
    /// 外部控制器
    #[serde(
        rename = "external-controller",
        skip_serializing_if = "Option::is_none"
    )]
    pub external_controller: Option<String>,
    /// 外部控制器密钥
    #[serde(rename = "secret", skip_serializing_if = "Option::is_none")]
    pub secret: Option<String>,
    /// DNS 配置
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dns: Option<DnsConfig>,
    /// 代理配置
    #[serde(default)]
    pub proxies: Vec<ProxyConfig>,
    /// 代理组配置
    #[serde(rename = "proxy-groups", default)]
    pub proxy_groups: Vec<ProxyGroupConfig>,
    /// 规则配置
    #[serde(default)]
    pub rules: Vec<RuleConfig>,
}

/// DNS 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsConfig {
    /// 是否启用 DNS
    #[serde(default = "default_dns_enable")]
    pub enable: bool,
    /// 监听地址
    #[serde(default = "default_dns_listen")]
    pub listen: String,
    /// 默认 nameserver
    #[serde(default)]
    pub nameserver: Vec<String>,
    /// 备用 nameserver
    #[serde(default)]
    pub fallback: Vec<String>,
    /// 增强模式
    #[serde(rename = "enhanced-mode", default = "default_enhanced_mode")]
    pub enhanced_mode: String,
    /// 假 IP 范围
    #[serde(rename = "fake-ip-range", default = "default_fake_ip_range")]
    pub fake_ip_range: String,
}

/// 代理配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyConfig {
    /// 代理名称
    pub name: String,
    /// 代理类型
    #[serde(rename = "type")]
    pub proxy_type: ProxyType,
    /// 服务器地址
    pub server: String,
    /// 服务器端口
    pub port: u16,
    /// 用户名（可选）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    /// 密码（可选）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
    /// 是否启用 UDP
    #[serde(default)]
    pub udp: bool,
    /// 跳过证书验证
    #[serde(rename = "skip-cert-verify", default)]
    pub skip_cert_verify: bool,
    /// 额外配置参数
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// 代理组配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyGroupConfig {
    /// 组名称
    pub name: String,
    /// 组类型
    #[serde(rename = "type")]
    pub group_type: String,
    /// 代理列表
    pub proxies: Vec<String>,
    /// 测试 URL（可选）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// 测试间隔（可选）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interval: Option<u32>,
    /// 容忍度（可选）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tolerance: Option<u32>,
}

/// 规则配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleConfig {
    /// 规则字符串（格式：TYPE,PAYLOAD,TARGET）
    #[serde(flatten)]
    pub rule: String,
}

/// 配置管理器
#[derive(Debug)]
pub struct ConfigManager {
    /// 当前配置
    config: Config,
    /// 配置文件路径
    config_path: Option<String>,
}

impl ConfigManager {
    /// 创建新的配置管理器
    pub fn new() -> Self {
        Self {
            config: Config::default(),
            config_path: None,
        }
    }

    /// 从文件加载配置
    ///
    /// # Arguments
    ///
    /// * `path` - 配置文件路径
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use mihomo_rs::config::ConfigManager;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut manager = ConfigManager::new();
    /// manager.load_from_file("config.yaml")?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn load_from_file<P: AsRef<Path>>(&mut self, path: P) -> Result<()> {
        let path = path.as_ref();
        let content = fs::read_to_string(path)
            .map_err(|e| MihomoError::config(format!("Failed to read config file: {}", e)))?;

        self.config = serde_yaml::from_str(&content)
            .map_err(|e| MihomoError::config(format!("Failed to parse config file: {}", e)))?;

        self.config_path = Some(path.to_string_lossy().to_string());

        self.validate_config()?;
        Ok(())
    }

    /// 从字符串加载配置
    pub fn load_from_str(&mut self, content: &str) -> Result<()> {
        self.config = serde_yaml::from_str(content)
            .map_err(|e| MihomoError::config(format!("Failed to parse config: {}", e)))?;

        self.validate_config()?;
        Ok(())
    }

    /// 保存配置到文件
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let content = serde_yaml::to_string(&self.config)
            .map_err(|e| MihomoError::config(format!("Failed to serialize config: {}", e)))?;

        fs::write(path, content)
            .map_err(|e| MihomoError::config(format!("Failed to write config file: {}", e)))?;

        Ok(())
    }

    /// 保存配置到当前文件路径
    pub fn save(&self) -> Result<()> {
        if let Some(ref path) = self.config_path {
            self.save_to_file(path)
        } else {
            Err(MihomoError::config("No config file path specified"))
        }
    }

    /// 获取当前配置的引用
    pub fn config(&self) -> &Config {
        &self.config
    }

    /// 获取当前配置的可变引用
    pub fn config_mut(&mut self) -> &mut Config {
        &mut self.config
    }

    /// 验证配置
    fn validate_config(&self) -> Result<()> {
        // 验证端口范围
        if self.config.port == 0 {
            return Err(MihomoError::config("Invalid port number"));
        }

        if self.config.socks_port == 0 {
            return Err(MihomoError::config("Invalid SOCKS port number"));
        }

        // 验证代理配置
        for proxy in &self.config.proxies {
            if proxy.name.is_empty() {
                return Err(MihomoError::config("Proxy name cannot be empty"));
            }

            if proxy.server.is_empty() {
                return Err(MihomoError::config("Proxy server cannot be empty"));
            }

            if proxy.port == 0 {
                return Err(MihomoError::config("Invalid proxy port number"));
            }
        }

        // 验证代理组配置
        for group in &self.config.proxy_groups {
            if group.name.is_empty() {
                return Err(MihomoError::config("Proxy group name cannot be empty"));
            }

            if group.proxies.is_empty() {
                return Err(MihomoError::config(
                    "Proxy group must contain at least one proxy",
                ));
            }
        }

        Ok(())
    }

    /// 添加代理
    pub fn add_proxy(&mut self, proxy: ProxyConfig) -> Result<()> {
        // 检查名称是否重复
        if self.config.proxies.iter().any(|p| p.name == proxy.name) {
            return Err(MihomoError::config(format!(
                "Proxy '{}' already exists",
                proxy.name
            )));
        }

        self.config.proxies.push(proxy);
        Ok(())
    }

    /// 删除代理
    pub fn remove_proxy(&mut self, name: &str) -> Result<()> {
        let index = self
            .config
            .proxies
            .iter()
            .position(|p| p.name == name)
            .ok_or_else(|| MihomoError::config(format!("Proxy '{}' not found", name)))?;

        self.config.proxies.remove(index);
        Ok(())
    }

    /// 添加代理组
    pub fn add_proxy_group(&mut self, group: ProxyGroupConfig) -> Result<()> {
        // 检查名称是否重复
        if self
            .config
            .proxy_groups
            .iter()
            .any(|g| g.name == group.name)
        {
            return Err(MihomoError::config(format!(
                "Proxy group '{}' already exists",
                group.name
            )));
        }

        self.config.proxy_groups.push(group);
        Ok(())
    }

    /// 删除代理组
    pub fn remove_proxy_group(&mut self, name: &str) -> Result<()> {
        let index = self
            .config
            .proxy_groups
            .iter()
            .position(|g| g.name == name)
            .ok_or_else(|| MihomoError::config(format!("Proxy group '{}' not found", name)))?;

        self.config.proxy_groups.remove(index);
        Ok(())
    }

    /// 添加规则
    pub fn add_rule(&mut self, rule: RuleConfig) {
        self.config.rules.push(rule);
    }

    /// 清空规则
    pub fn clear_rules(&mut self) {
        self.config.rules.clear();
    }
}

// 默认值函数
fn default_bind_address() -> String {
    "*".to_string()
}

fn default_mode() -> String {
    "rule".to_string()
}

fn default_log_level() -> String {
    "info".to_string()
}

fn default_dns_enable() -> bool {
    false
}

fn default_dns_listen() -> String {
    "0.0.0.0:53".to_string()
}

fn default_enhanced_mode() -> String {
    "fake-ip".to_string()
}

fn default_fake_ip_range() -> String {
    "198.18.0.1/16".to_string()
}

impl Default for Config {
    fn default() -> Self {
        Self {
            port: 7890,
            socks_port: 7891,
            redir_port: None,
            tproxy_port: None,
            mixed_port: None,
            allow_lan: false,
            bind_address: default_bind_address(),
            mode: default_mode(),
            log_level: default_log_level(),
            external_controller: None,
            secret: None,
            dns: None,
            proxies: Vec::new(),
            proxy_groups: Vec::new(),
            rules: Vec::new(),
        }
    }
}

impl Default for ConfigManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = Config::default();
        assert_eq!(config.port, 7890);
        assert_eq!(config.socks_port, 7891);
        assert_eq!(config.mode, "rule");
    }

    #[test]
    fn test_config_manager_creation() {
        let manager = ConfigManager::new();
        assert_eq!(manager.config().port, 7890);
    }

    #[test]
    fn test_add_proxy() {
        let mut manager = ConfigManager::new();
        let proxy = ProxyConfig {
            name: "test-proxy".to_string(),
            proxy_type: ProxyType::Http,
            server: "127.0.0.1".to_string(),
            port: 8080,
            username: None,
            password: None,
            udp: false,
            skip_cert_verify: false,
            extra: HashMap::new(),
        };

        assert!(manager.add_proxy(proxy).is_ok());
        assert_eq!(manager.config().proxies.len(), 1);
    }
}
