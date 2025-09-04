//! 工具函数模块
//!
//! 提供 SDK 中使用的各种辅助函数和工具类。

// 移除未使用的全局导入

/// URL 工具函数
pub mod url_utils {
    use crate::error::{MihomoError, Result};
    use std::collections::HashMap;
    use url::Url;

    /// 验证 URL 格式
    ///
    /// # Arguments
    ///
    /// * `url` - 要验证的 URL 字符串
    ///
    /// # Examples
    ///
    /// ```
    /// use mihomo_rs::utils::url_utils::validate_url;
    ///
    /// assert!(validate_url("http://127.0.0.1:9090").is_ok());
    /// assert!(validate_url("invalid-url").is_err());
    /// ```
    pub fn validate_url(url: &str) -> Result<Url> {
        Url::parse(url).map_err(MihomoError::UrlParse)
    }

    /// 构建 API URL
    ///
    /// # Arguments
    ///
    /// * `base_url` - 基础 URL
    /// * `path` - API 路径
    /// * `params` - 查询参数
    pub fn build_api_url(
        base_url: &str,
        path: &str,
        params: Option<&HashMap<String, String>>,
    ) -> Result<String> {
        let mut url = validate_url(base_url)?;

        // 添加路径
        if !path.is_empty() {
            let path = if path.starts_with('/') {
                path
            } else {
                &format!("/{}", path)
            };
            url = url.join(path).map_err(MihomoError::UrlParse)?;
        }

        // 添加查询参数
        if let Some(params) = params {
            let mut query_pairs = url.query_pairs_mut();
            for (key, value) in params {
                query_pairs.append_pair(key, value);
            }
        }

        Ok(url.to_string())
    }

    /// 提取主机和端口
    pub fn extract_host_port(url: &str) -> Result<(String, Option<u16>)> {
        let parsed = validate_url(url)?;
        let host = parsed
            .host_str()
            .ok_or_else(|| MihomoError::internal("No host found in URL"))?
            .to_string();
        let port = parsed.port();
        Ok((host, port))
    }
}

/// 网络工具函数
pub mod network_utils {
    use crate::error::{MihomoError, Result};
    use std::net::IpAddr;
    use std::str::FromStr;

    /// 验证 IP 地址
    ///
    /// # Arguments
    ///
    /// * `ip` - IP 地址字符串
    ///
    /// # Examples
    ///
    /// ```
    /// use mihomo_rs::utils::network_utils::validate_ip;
    ///
    /// assert!(validate_ip("192.168.1.1").is_ok());
    /// assert!(validate_ip("::1").is_ok());
    /// assert!(validate_ip("invalid-ip").is_err());
    /// ```
    pub fn validate_ip(ip: &str) -> Result<IpAddr> {
        IpAddr::from_str(ip)
            .map_err(|e| MihomoError::invalid_parameter(format!("Invalid IP address: {}", e)))
    }

    /// 验证端口号
    pub fn validate_port(port: u16) -> Result<u16> {
        if port == 0 {
            Err(MihomoError::invalid_parameter(
                "Port cannot be 0".to_string(),
            ))
        } else {
            Ok(port)
        }
    }

    /// 检查是否为私有 IP
    pub fn is_private_ip(ip: &IpAddr) -> bool {
        match ip {
            IpAddr::V4(ipv4) => ipv4.is_private() || ipv4.is_loopback() || ipv4.is_link_local(),
            IpAddr::V6(ipv6) => {
                ipv6.is_loopback() || ipv6.is_unicast_link_local() || ipv6.is_unique_local()
            }
        }
    }

    /// 解析 CIDR 网络
    pub fn parse_cidr(cidr: &str) -> Result<(IpAddr, u8)> {
        let parts: Vec<&str> = cidr.split('/').collect();
        if parts.len() != 2 {
            return Err(MihomoError::invalid_parameter(
                "Invalid CIDR format".to_string(),
            ));
        }

        let ip = validate_ip(parts[0])?;
        let prefix_len: u8 = parts[1]
            .parse()
            .map_err(|_| MihomoError::invalid_parameter("Invalid prefix length".to_string()))?;

        // 验证前缀长度
        let max_prefix = match ip {
            IpAddr::V4(_) => 32,
            IpAddr::V6(_) => 128,
        };

        if prefix_len > max_prefix {
            return Err(MihomoError::invalid_parameter(format!(
                "Prefix length {} exceeds maximum {}",
                prefix_len, max_prefix
            )));
        }

        Ok((ip, prefix_len))
    }

    /// 检查 IP 是否在 CIDR 范围内
    pub fn ip_in_cidr(ip: &IpAddr, cidr: &str) -> Result<bool> {
        let (network_ip, prefix_len) = parse_cidr(cidr)?;

        // 确保 IP 类型匹配
        match (ip, &network_ip) {
            (IpAddr::V4(ip4), IpAddr::V4(net4)) => {
                let ip_bits = u32::from(*ip4);
                let net_bits = u32::from(*net4);
                let mask = !((1u32 << (32 - prefix_len)) - 1);
                Ok((ip_bits & mask) == (net_bits & mask))
            }
            (IpAddr::V6(ip6), IpAddr::V6(net6)) => {
                let ip_bits = u128::from(*ip6);
                let net_bits = u128::from(*net6);
                let mask = !((1u128 << (128 - prefix_len)) - 1);
                Ok((ip_bits & mask) == (net_bits & mask))
            }
            _ => Ok(false), // 不同类型的 IP 不匹配
        }
    }
}

/// 字符串工具函数
pub mod string_utils {
    use crate::error::{MihomoError, Result};
    use base64::{engine::general_purpose, Engine as _};
    use regex::Regex;

    /// Base64 编码
    ///
    /// # Arguments
    ///
    /// * `data` - 要编码的数据
    ///
    /// # Examples
    ///
    /// ```
    /// use mihomo_rs::utils::string_utils::base64_encode;
    ///
    /// let encoded = base64_encode(b"hello world");
    /// assert_eq!(encoded, "aGVsbG8gd29ybGQ=");
    /// ```
    pub fn base64_encode(data: &[u8]) -> String {
        general_purpose::STANDARD.encode(data)
    }

    /// Base64 解码
    pub fn base64_decode(data: &str) -> Result<Vec<u8>> {
        general_purpose::STANDARD
            .decode(data)
            .map_err(|e| MihomoError::invalid_parameter(format!("Base64 decode error: {}", e)))
    }

    /// 验证域名格式
    pub fn validate_domain(domain: &str) -> Result<String> {
        let domain_regex = Regex::new(r"^[a-zA-Z0-9]([a-zA-Z0-9\-]{0,61}[a-zA-Z0-9])?(\.[a-zA-Z0-9]([a-zA-Z0-9\-]{0,61}[a-zA-Z0-9])?)*$")
            .map_err(|e| MihomoError::internal(format!("Regex error: {}", e)))?;

        if domain.is_empty() || domain.len() > 253 {
            return Err(MihomoError::invalid_parameter(
                "Invalid domain length".to_string(),
            ));
        }

        if !domain_regex.is_match(domain) {
            return Err(MihomoError::invalid_parameter(
                "Invalid domain format".to_string(),
            ));
        }

        Ok(domain.to_lowercase())
    }

    /// 清理字符串（移除多余空格）
    pub fn clean_string(s: &str) -> String {
        s.split_whitespace().collect::<Vec<_>>().join(" ")
    }

    /// 检查字符串是否为空或只包含空白字符
    pub fn is_empty_or_whitespace(s: &str) -> bool {
        s.trim().is_empty()
    }

    /// 截断字符串到指定长度
    pub fn truncate_string(s: &str, max_len: usize) -> String {
        if s.len() <= max_len {
            s.to_string()
        } else {
            format!("{}...", &s[..max_len.saturating_sub(3)])
        }
    }
}

/// 时间工具函数
pub mod time_utils {
    use crate::error::{MihomoError, Result};
    use regex::Regex;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};
    // use chrono::{DateTime, Utc};

    /// 获取当前时间戳（秒）
    ///
    /// # Examples
    ///
    /// ```
    /// use mihomo_rs::utils::time_utils::current_timestamp;
    ///
    /// let timestamp = current_timestamp();
    /// assert!(timestamp > 0);
    /// ```
    pub fn current_timestamp() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }

    /// 获取当前时间戳（毫秒）
    pub fn current_timestamp_millis() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
    }

    /// 格式化持续时间
    pub fn format_duration(duration: Duration) -> String {
        let total_seconds = duration.as_secs();
        let hours = total_seconds / 3600;
        let minutes = (total_seconds % 3600) / 60;
        let seconds = total_seconds % 60;

        if hours > 0 {
            format!("{}h {}m {}s", hours, minutes, seconds)
        } else if minutes > 0 {
            format!("{}m {}s", minutes, seconds)
        } else {
            format!("{}s", seconds)
        }
    }

    /// 解析持续时间字符串（如 "1h30m", "45s"）
    pub fn parse_duration(s: &str) -> Result<Duration> {
        let duration_regex = Regex::new(r"(?:(\d+)h)?(?:(\d+)m)?(?:(\d+)s)?")
            .map_err(|e| MihomoError::internal(format!("Regex error: {}", e)))?;

        let captures = duration_regex
            .captures(s)
            .ok_or_else(|| MihomoError::invalid_parameter("Invalid duration format".to_string()))?;

        let hours: u64 = captures
            .get(1)
            .map(|m| m.as_str().parse().unwrap_or(0))
            .unwrap_or(0);
        let minutes: u64 = captures
            .get(2)
            .map(|m| m.as_str().parse().unwrap_or(0))
            .unwrap_or(0);
        let seconds: u64 = captures
            .get(3)
            .map(|m| m.as_str().parse().unwrap_or(0))
            .unwrap_or(0);

        if hours == 0 && minutes == 0 && seconds == 0 {
            return Err(MihomoError::invalid_parameter(
                "Duration cannot be zero".to_string(),
            ));
        }

        Ok(Duration::from_secs(hours * 3600 + minutes * 60 + seconds))
    }

    /// 检查时间是否过期
    pub fn is_expired(timestamp: u64, ttl_seconds: u64) -> bool {
        current_timestamp() > timestamp + ttl_seconds
    }
}

/// 数据格式化工具
pub mod format_utils {

    /// 格式化字节大小
    ///
    /// # Arguments
    ///
    /// * `bytes` - 字节数
    ///
    /// # Examples
    ///
    /// ```
    /// use mihomo_rs::utils::format_utils::format_bytes;
    ///
    /// assert_eq!(format_bytes(1024), "1.00 KB");
    /// assert_eq!(format_bytes(1048576), "1.00 MB");
    /// ```
    pub fn format_bytes(bytes: u64) -> String {
        const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB", "PB"];

        if bytes == 0 {
            return "0 B".to_string();
        }

        let mut size = bytes as f64;
        let mut unit_index = 0;

        while size >= 1024.0 && unit_index < UNITS.len() - 1 {
            size /= 1024.0;
            unit_index += 1;
        }

        format!("{:.2} {}", size, UNITS[unit_index])
    }

    /// 格式化速度（字节/秒）
    pub fn format_speed(bytes_per_second: u64) -> String {
        format!("{}/s", format_bytes(bytes_per_second))
    }

    /// 格式化百分比
    pub fn format_percentage(value: f64, total: f64) -> String {
        if total == 0.0 {
            "0.00%".to_string()
        } else {
            format!("{:.2}%", (value / total) * 100.0)
        }
    }

    /// 格式化延迟时间
    pub fn format_latency(millis: u64) -> String {
        if millis < 1000 {
            format!("{}ms", millis)
        } else {
            format!("{:.2}s", millis as f64 / 1000.0)
        }
    }
}

/// 配置验证工具
pub mod validation_utils {
    use crate::error::{MihomoError, Result};
    use crate::types::{ProxyType, RuleType};
    use crate::utils::{network_utils, string_utils};

    /// 验证代理配置
    pub fn validate_proxy_config(proxy_type: &ProxyType, server: &str, port: u16) -> Result<()> {
        // 验证服务器地址
        if string_utils::is_empty_or_whitespace(server) {
            return Err(MihomoError::invalid_parameter(
                "Proxy server cannot be empty".to_string(),
            ));
        }

        // 验证端口
        network_utils::validate_port(port)?;

        // 根据代理类型进行特定验证
        match proxy_type {
            ProxyType::Http | ProxyType::Https => {
                // HTTP 代理通常使用标准端口 设置一个默认
                if port != 80 && port != 8080 && port != 3128 {
                    log::warn!("Unusual port {} for HTTP proxy", port);
                }
            }
            ProxyType::Socks5 => {
                // SOCKS5 代理通常使用 1080 端口
                if port != 1080 {
                    log::warn!("Unusual port {} for SOCKS5 proxy", port);
                }
            }
            _ => {}
        }

        Ok(())
    }

    /// 验证规则配置
    pub fn validate_rule_config(rule_type: &RuleType, payload: &str, target: &str) -> Result<()> {
        // 验证目标不能为空
        if string_utils::is_empty_or_whitespace(target) {
            return Err(MihomoError::invalid_parameter(
                "Rule target cannot be empty".to_string(),
            ));
        }

        // 根据规则类型验证载荷
        match rule_type {
            RuleType::Domain => {
                string_utils::validate_domain(payload)?;
            }
            RuleType::DomainSuffix => {
                if !payload.starts_with('.') {
                    return Err(MihomoError::invalid_parameter(
                        "Domain suffix should start with '.'".to_string(),
                    ));
                }
                string_utils::validate_domain(&payload[1..])?;
            }
            RuleType::DomainKeyword => {
                if string_utils::is_empty_or_whitespace(payload) {
                    return Err(MihomoError::invalid_parameter(
                        "Domain keyword cannot be empty".to_string(),
                    ));
                }
            }
            RuleType::IpCidr => {
                network_utils::parse_cidr(payload)?;
            }
            RuleType::SrcIpCidr => {
                network_utils::parse_cidr(payload)?;
            }
            RuleType::SrcPort => {
                let port: u16 = payload.parse().map_err(|_| {
                    MihomoError::invalid_parameter("Invalid port number".to_string())
                })?;
                network_utils::validate_port(port)?;
            }
            RuleType::DstPort => {
                let port: u16 = payload.parse().map_err(|_| {
                    MihomoError::invalid_parameter("Invalid port number".to_string())
                })?;
                network_utils::validate_port(port)?;
            }
            _ => {
                // 其他规则类型的基本验证
                if string_utils::is_empty_or_whitespace(payload) {
                    return Err(MihomoError::invalid_parameter(
                        "Rule payload cannot be empty".to_string(),
                    ));
                }
            }
        }

        Ok(())
    }
}

/// 随机工具函数
pub mod random_utils {
    use crate::utils::time_utils::current_timestamp;
    use rand::{thread_rng, Rng};

    /// 生成随机字符串
    ///
    /// # Arguments
    ///
    /// * `length` - 字符串长度
    ///
    /// # Examples
    ///
    /// ```
    /// use mihomo_rs::utils::random_utils::generate_random_string;
    ///
    /// let random_str = generate_random_string(10);
    /// assert_eq!(random_str.len(), 10);
    /// ```
    pub fn generate_random_string(length: usize) -> String {
        const CHARSET: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
        let mut rng = thread_rng();

        (0..length)
            .map(|_| {
                let idx = rng.gen_range(0..CHARSET.len());
                CHARSET[idx] as char
            })
            .collect()
    }

    /// 生成随机 ID
    pub fn generate_id() -> String {
        format!("{}-{}", current_timestamp(), generate_random_string(8))
    }

    /// 从切片中随机选择元素
    pub fn random_choice<T>(items: &[T]) -> Option<&T> {
        if items.is_empty() {
            None
        } else {
            let mut rng = thread_rng();
            let index = rng.gen_range(0..items.len());
            Some(&items[index])
        }
    }
}

// 重新导出常用函数
// use time_utils::current_timestamp;

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_url_validation() {
        assert!(url_utils::validate_url("http://127.0.0.1:9090").is_ok());
        assert!(url_utils::validate_url("https://example.com").is_ok());
        assert!(url_utils::validate_url("invalid-url").is_err());
    }

    #[test]
    fn test_ip_validation() {
        assert!(network_utils::validate_ip("192.168.1.1").is_ok());
        assert!(network_utils::validate_ip("::1").is_ok());
        assert!(network_utils::validate_ip("invalid-ip").is_err());
    }

    #[test]
    fn test_cidr_parsing() {
        assert!(network_utils::parse_cidr("192.168.1.0/24").is_ok());
        assert!(network_utils::parse_cidr("2001:db8::/32").is_ok());
        assert!(network_utils::parse_cidr("invalid-cidr").is_err());
    }

    #[test]
    fn test_base64_encoding() {
        let data = b"hello world";
        let encoded = string_utils::base64_encode(data);
        let decoded = string_utils::base64_decode(&encoded).unwrap();
        assert_eq!(data, decoded.as_slice());
    }

    #[test]
    fn test_domain_validation() {
        assert!(string_utils::validate_domain("example.com").is_ok());
        assert!(string_utils::validate_domain("sub.example.com").is_ok());
        assert!(string_utils::validate_domain("-invalid.com").is_err());
    }

    #[test]
    fn test_duration_parsing() {
        assert_eq!(
            time_utils::parse_duration("1h30m").unwrap(),
            Duration::from_secs(5400)
        );
        assert_eq!(
            time_utils::parse_duration("45s").unwrap(),
            Duration::from_secs(45)
        );
        assert!(time_utils::parse_duration("invalid").is_err());
    }

    #[test]
    fn test_bytes_formatting() {
        assert_eq!(format_utils::format_bytes(1024), "1.00 KB");
        assert_eq!(format_utils::format_bytes(1048576), "1.00 MB");
        assert_eq!(format_utils::format_bytes(0), "0 B");
    }

    #[test]
    fn test_random_string_generation() {
        let random_str = random_utils::generate_random_string(10);
        assert_eq!(random_str.len(), 10);

        let another_str = random_utils::generate_random_string(10);
        assert_ne!(random_str, another_str); // 应该不相同（概率极低）
    }
}
