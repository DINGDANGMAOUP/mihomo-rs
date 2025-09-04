//! 规则引擎模块
//! 
//! 提供流量分流和规则匹配功能，支持多种规则类型和自定义规则。

use crate::client::MihomoClient;
use crate::error::{MihomoError, Result};
use crate::types::{Rule, RuleType};
use regex::Regex;
use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr};
use std::str::FromStr;

/// 规则引擎
#[derive(Debug)]
pub struct RuleEngine {
    /// mihomo 客户端
    client: MihomoClient,
    /// 规则缓存
    rules_cache: Vec<Rule>,
    /// 编译后的正则表达式缓存
    regex_cache: HashMap<String, Regex>,
    /// 缓存是否有效
    cache_valid: bool,
}

impl RuleEngine {
    /// 创建新的规则引擎
    /// 
    /// # Arguments
    /// 
    /// * `client` - mihomo 客户端实例
    /// 
    /// # Examples
    /// 
    /// ```no_run
    /// use mihomo_rs::{MihomoClient, rules::RuleEngine};
    /// 
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = MihomoClient::new("http://127.0.0.1:9090", None)?;
    /// let engine = RuleEngine::new(client);
    /// # Ok(())
    /// # }
    /// ```
    pub fn new(client: MihomoClient) -> Self {
        Self {
            client,
            rules_cache: Vec::new(),
            regex_cache: HashMap::new(),
            cache_valid: false,
        }
    }

    /// 刷新规则缓存
    pub async fn refresh_rules(&mut self) -> Result<()> {
        log::debug!("Refreshing rules cache");
        
        self.rules_cache = self.client.rules().await?;
        self.cache_valid = true;
        
        log::debug!("Rules cache refreshed: {} rules loaded", self.rules_cache.len());
        Ok(())
    }

    /// 确保规则缓存有效
    async fn ensure_rules_cache(&mut self) -> Result<()> {
        if !self.cache_valid {
            self.refresh_rules().await?
        }
        Ok(())
    }

    /// 获取所有规则
    pub async fn get_rules(&mut self) -> Result<&Vec<Rule>> {
        self.ensure_rules_cache().await?;
        Ok(&self.rules_cache)
    }

    /// 根据目标匹配规则
    /// 
    /// # Arguments
    /// 
    /// * `target` - 目标地址或域名
    /// * `port` - 目标端口（可选）
    /// * `network` - 网络类型（tcp/udp，可选）
    /// 
    /// # Returns
    /// 
    /// 返回匹配的规则和对应的代理名称
    pub async fn match_rule(
        &mut self,
        target: &str,
        port: Option<u16>,
        _network: Option<&str>,
    ) -> Result<Option<(Rule, String)>> {
        self.ensure_rules_cache().await?;
        
        let rules_cache = self.rules_cache.clone();
        for rule in &rules_cache {
            if self.is_rule_match(rule, target, port, _network)? {
                return Ok(Some((rule.clone(), rule.proxy.clone())));
            }
        }
        
        Ok(None)
    }

    /// 检查规则是否匹配
    fn is_rule_match(
        &mut self,
        rule: &Rule,
        target: &str,
        port: Option<u16>,
        network: Option<&str>,
    ) -> Result<bool> {
        match rule.rule_type {
            RuleType::Domain => self.match_domain(rule, target),
            RuleType::DomainSuffix => self.match_domain_suffix(rule, target),
            RuleType::DomainKeyword => self.match_domain_keyword(rule, target),
            RuleType::Geoip => self.match_geoip(rule, target),
            RuleType::IpCidr => self.match_ip_cidr(rule, target),
            RuleType::SrcIpCidr => Ok(false), // 需要源IP信息，暂不支持
            RuleType::SrcPort => Ok(false),   // 需要源端口信息，暂不支持
            RuleType::DstPort => self.match_dst_port(rule, port),
            RuleType::ProcessName => Ok(false), // 需要进程信息，暂不支持
            RuleType::ProcessPath => Ok(false), // 需要进程信息，暂不支持
            RuleType::Script => Ok(false),      // 脚本规则暂不支持
            RuleType::RuleSet => Ok(false),     // 规则集暂不支持
            RuleType::Match => Ok(true),        // 匹配所有
        }
    }

    /// 匹配域名规则
    fn match_domain(&self, rule: &Rule, target: &str) -> Result<bool> {
        Ok(rule.payload.eq_ignore_ascii_case(target))
    }

    /// 匹配域名后缀规则
    fn match_domain_suffix(&self, rule: &Rule, target: &str) -> Result<bool> {
        let suffix = &rule.payload;
        Ok(target.to_lowercase().ends_with(&suffix.to_lowercase()) &&
           (target.len() == suffix.len() || 
            target.chars().nth(target.len() - suffix.len() - 1) == Some('.')))
    }

    /// 匹配域名关键字规则
    fn match_domain_keyword(&self, rule: &Rule, target: &str) -> Result<bool> {
        Ok(target.to_lowercase().contains(&rule.payload.to_lowercase()))
    }

    /// 匹配 GEOIP 规则
    fn match_geoip(&self, rule: &Rule, target: &str) -> Result<bool> {
        // 检查目标是否为IP地址
        if let Ok(_ip) = IpAddr::from_str(target) {
            // 这里需要实际的 GeoIP 数据库支持
            // 暂时返回 false，实际实现需要集成 GeoIP 库
            log::warn!("GEOIP rule matching not implemented: {}", rule.payload);
            Ok(false)
        } else {
            Ok(false)
        }
    }

    /// 匹配 IP-CIDR 规则
    fn match_ip_cidr(&self, rule: &Rule, target: &str) -> Result<bool> {
        if let Ok(target_ip) = IpAddr::from_str(target) {
            self.is_ip_in_cidr(target_ip, &rule.payload)
        } else {
            Ok(false)
        }
    }

    /// 匹配目标端口规则
    fn match_dst_port(&self, rule: &Rule, port: Option<u16>) -> Result<bool> {
        if let Some(target_port) = port {
            // 支持单个端口和端口范围
            if rule.payload.contains('-') {
                // 端口范围：如 "80-90"
                let parts: Vec<&str> = rule.payload.split('-').collect();
                if parts.len() == 2 {
                    if let (Ok(start), Ok(end)) = (parts[0].parse::<u16>(), parts[1].parse::<u16>()) {
                        return Ok(target_port >= start && target_port <= end);
                    }
                }
            } else if rule.payload.contains(',') {
                // 多个端口：如 "80,443,8080"
                for port_str in rule.payload.split(',') {
                    if let Ok(rule_port) = port_str.trim().parse::<u16>() {
                        if target_port == rule_port {
                            return Ok(true);
                        }
                    }
                }
            } else {
                // 单个端口
                if let Ok(rule_port) = rule.payload.parse::<u16>() {
                    return Ok(target_port == rule_port);
                }
            }
        }
        Ok(false)
    }

    /// 检查IP是否在CIDR范围内
    fn is_ip_in_cidr(&self, ip: IpAddr, cidr: &str) -> Result<bool> {
        let parts: Vec<&str> = cidr.split('/').collect();
        if parts.len() != 2 {
            return Err(MihomoError::rules(format!("Invalid CIDR format: {}", cidr)));
        }

        let network_ip = IpAddr::from_str(parts[0])
            .map_err(|_| MihomoError::rules(format!("Invalid IP in CIDR: {}", parts[0])))?;
        
        let prefix_len: u8 = parts[1].parse()
            .map_err(|_| MihomoError::rules(format!("Invalid prefix length: {}", parts[1])))?;

        match (ip, network_ip) {
            (IpAddr::V4(ip4), IpAddr::V4(net4)) => {
                if prefix_len > 32 {
                    return Err(MihomoError::rules("IPv4 prefix length cannot exceed 32".to_string()));
                }
                let mask = if prefix_len == 0 { 0 } else { !0u32 << (32 - prefix_len) };
                Ok((u32::from(ip4) & mask) == (u32::from(net4) & mask))
            }
            (IpAddr::V6(ip6), IpAddr::V6(net6)) => {
                if prefix_len > 128 {
                    return Err(MihomoError::rules("IPv6 prefix length cannot exceed 128".to_string()));
                }
                let ip6_bytes = ip6.octets();
                let net6_bytes = net6.octets();
                
                let full_bytes = (prefix_len / 8) as usize;
                let remaining_bits = prefix_len % 8;
                
                // 检查完整字节
                if ip6_bytes[..full_bytes] != net6_bytes[..full_bytes] {
                    return Ok(false);
                }
                
                // 检查剩余位
                if remaining_bits > 0 && full_bytes < 16 {
                    let mask = !0u8 << (8 - remaining_bits);
                    if (ip6_bytes[full_bytes] & mask) != (net6_bytes[full_bytes] & mask) {
                        return Ok(false);
                    }
                }
                
                Ok(true)
            }
            _ => Ok(false), // IP版本不匹配
        }
    }

    /// 获取规则统计信息
    pub async fn get_rule_stats(&mut self) -> Result<RuleStats> {
        self.ensure_rules_cache().await?;
        
        let total_rules = self.rules_cache.len();
        let mut type_counts = HashMap::new();
        let mut proxy_counts = HashMap::new();
        
        for rule in &self.rules_cache {
            *type_counts.entry(rule.rule_type.clone()).or_insert(0) += 1;
            *proxy_counts.entry(rule.proxy.clone()).or_insert(0) += 1;
        }
        
        Ok(RuleStats {
            total_rules,
            type_counts,
            proxy_counts,
        })
    }

    /// 查找使用指定代理的规则
    pub async fn find_rules_by_proxy(&mut self, proxy_name: &str) -> Result<Vec<Rule>> {
        self.ensure_rules_cache().await?;
        
        Ok(self.rules_cache
            .iter()
            .filter(|rule| rule.proxy == proxy_name)
            .cloned()
            .collect())
    }

    /// 查找指定类型的规则
    pub async fn find_rules_by_type(&mut self, rule_type: RuleType) -> Result<Vec<Rule>> {
        self.ensure_rules_cache().await?;
        
        Ok(self.rules_cache
            .iter()
            .filter(|rule| rule.rule_type == rule_type)
            .cloned()
            .collect())
    }

    /// 验证规则格式
    pub fn validate_rule(&self, rule_str: &str) -> Result<ParsedRule> {
        let parts: Vec<&str> = rule_str.split(',').collect();
        
        if parts.len() < 3 {
            return Err(MihomoError::rules("Rule must have at least 3 parts: TYPE,PAYLOAD,TARGET".to_string()));
        }
        
        let rule_type = match parts[0].to_uppercase().as_str() {
            "DOMAIN" => RuleType::Domain,
            "DOMAIN-SUFFIX" => RuleType::DomainSuffix,
            "DOMAIN-KEYWORD" => RuleType::DomainKeyword,
            "GEOIP" => RuleType::Geoip,
            "IP-CIDR" => RuleType::IpCidr,
            "SRC-IP-CIDR" => RuleType::SrcIpCidr,
            "SRC-PORT" => RuleType::SrcPort,
            "DST-PORT" => RuleType::DstPort,
            "PROCESS-NAME" => RuleType::ProcessName,
            "PROCESS-PATH" => RuleType::ProcessPath,
            "SCRIPT" => RuleType::Script,
            "RULE-SET" => RuleType::RuleSet,
            "MATCH" => RuleType::Match,
            _ => return Err(MihomoError::rules(format!("Unknown rule type: {}", parts[0]))),
        };
        
        let payload = parts[1].to_string();
        let target = parts[2].to_string();
        let options = if parts.len() > 3 {
            Some(parts[3..].join(","))
        } else {
            None
        };
        
        // 验证载荷格式
        self.validate_payload(&rule_type, &payload)?;
        
        Ok(ParsedRule {
            rule_type,
            payload,
            target,
            options,
        })
    }

    /// 验证规则载荷格式
    fn validate_payload(&self, rule_type: &RuleType, payload: &str) -> Result<()> {
        match rule_type {
            RuleType::IpCidr | RuleType::SrcIpCidr => {
                // 验证 CIDR 格式
                let parts: Vec<&str> = payload.split('/').collect();
                if parts.len() != 2 {
                    return Err(MihomoError::rules("CIDR must be in format IP/PREFIX".to_string()));
                }
                
                IpAddr::from_str(parts[0])
                    .map_err(|_| MihomoError::rules("Invalid IP address in CIDR".to_string()))?;
                
                let prefix: u8 = parts[1].parse()
                    .map_err(|_| MihomoError::rules("Invalid prefix length".to_string()))?;
                
                match IpAddr::from_str(parts[0])? {
                    IpAddr::V4(_) if prefix > 32 => {
                        return Err(MihomoError::rules("IPv4 prefix cannot exceed 32".to_string()));
                    }
                    IpAddr::V6(_) if prefix > 128 => {
                        return Err(MihomoError::rules("IPv6 prefix cannot exceed 128".to_string()));
                    }
                    _ => {}
                }
            }
            RuleType::DstPort | RuleType::SrcPort => {
                // 验证端口格式
                if payload.contains('-') {
                    let parts: Vec<&str> = payload.split('-').collect();
                    if parts.len() != 2 {
                        return Err(MihomoError::rules("Port range must be in format START-END".to_string()));
                    }
                    
                    let start: u16 = parts[0].parse()
                        .map_err(|_| MihomoError::rules("Invalid start port".to_string()))?;
                    let end: u16 = parts[1].parse()
                        .map_err(|_| MihomoError::rules("Invalid end port".to_string()))?;
                    
                    if start > end {
                        return Err(MihomoError::rules("Start port cannot be greater than end port".to_string()));
                    }
                } else if payload.contains(',') {
                    for port_str in payload.split(',') {
                        port_str.trim().parse::<u16>()
                            .map_err(|_| MihomoError::rules(format!("Invalid port: {}", port_str)))?;
                    }
                } else {
                    payload.parse::<u16>()
                        .map_err(|_| MihomoError::rules("Invalid port number".to_string()))?;
                }
            }
            _ => {} // 其他类型暂不验证
        }
        
        Ok(())
    }
}

/// 解析后的规则
#[derive(Debug, Clone)]
pub struct ParsedRule {
    /// 规则类型
    pub rule_type: RuleType,
    /// 规则载荷
    pub payload: String,
    /// 目标代理
    pub target: String,
    /// 额外选项
    pub options: Option<String>,
}

/// 规则统计信息
#[derive(Debug, Clone)]
pub struct RuleStats {
    /// 总规则数量
    pub total_rules: usize,
    /// 各类型规则数量统计
    pub type_counts: HashMap<RuleType, usize>,
    /// 各代理使用的规则数量统计
    pub proxy_counts: HashMap<String, usize>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MihomoClient;

    #[test]
    fn test_rule_engine_creation() {
        let client = MihomoClient::new("http://127.0.0.1:9090", None).unwrap();
        let engine = RuleEngine::new(client);
        assert!(!engine.cache_valid);
    }

    #[test]
    fn test_domain_suffix_match() {
        let client = MihomoClient::new("http://127.0.0.1:9090", None).unwrap();
        let engine = RuleEngine::new(client);
        
        let rule = Rule {
            rule_type: RuleType::DomainSuffix,
            payload: "google.com".to_string(),
            proxy: "Proxy".to_string(),
            size: 0,
        };
        
        assert!(engine.match_domain_suffix(&rule, "www.google.com").unwrap());
        assert!(engine.match_domain_suffix(&rule, "google.com").unwrap());
        assert!(!engine.match_domain_suffix(&rule, "google.com.cn").unwrap());
    }

    #[test]
    fn test_ip_cidr_validation() {
        let client = MihomoClient::new("http://127.0.0.1:9090", None).unwrap();
        let engine = RuleEngine::new(client);
        
        let ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100));
        assert!(engine.is_ip_in_cidr(ip, "192.168.1.0/24").unwrap());
        assert!(!engine.is_ip_in_cidr(ip, "192.168.2.0/24").unwrap());
    }

    #[test]
    fn test_rule_validation() {
        let client = MihomoClient::new("http://127.0.0.1:9090", None).unwrap();
        let engine = RuleEngine::new(client);
        
        let valid_rule = "DOMAIN-SUFFIX,google.com,Proxy";
        assert!(engine.validate_rule(valid_rule).is_ok());
        
        let invalid_rule = "INVALID-TYPE,google.com,Proxy";
        assert!(engine.validate_rule(invalid_rule).is_err());
    }
}