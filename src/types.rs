//! 类型定义模块
//! 
//! 定义了 SDK 中使用的核心数据结构和类型。

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use chrono::{DateTime, Utc};

/// 代理类型枚举
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum ProxyType {
    /// HTTP 代理
    Http,
    /// HTTPS 代理
    Https,
    /// SOCKS5 代理
    Socks5,
    /// Shadowsocks 代理
    Ss,
    /// ShadowsocksR 代理
    Ssr,
    /// VMess 代理
    Vmess,
    /// VLESS 代理
    Vless,
    /// Trojan 代理
    Trojan,
    /// Hysteria 代理
    Hysteria,
    /// WireGuard 代理
    Wireguard,
    /// 直连
    Direct,
    /// 拒绝连接
    Reject,
}

/// 代理节点信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyNode {
    /// 节点名称
    pub name: String,
    /// 代理类型
    #[serde(rename = "type")]
    pub proxy_type: ProxyType,
    /// 服务器地址
    pub server: String,
    /// 服务器端口
    pub port: u16,
    /// 是否启用UDP
    #[serde(default)]
    pub udp: bool,
    /// 延迟信息（毫秒）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delay: Option<u32>,
    /// 历史延迟记录
    #[serde(default)]
    pub history: Vec<DelayHistory>,
    /// 额外配置参数
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// 延迟历史记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DelayHistory {
    /// 延迟时间（毫秒）
    pub delay: u32,
    /// 测试时间
    pub time: DateTime<Utc>,
}

/// 代理组信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyGroup {
    /// 组名称
    pub name: String,
    /// 组类型
    #[serde(rename = "type")]
    pub group_type: ProxyGroupType,
    /// 当前选中的代理
    #[serde(skip_serializing_if = "Option::is_none")]
    pub now: Option<String>,
    /// 组内代理列表
    pub all: Vec<String>,
    /// 历史记录
    #[serde(default)]
    pub history: Vec<DelayHistory>,
}

/// 代理组类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "PascalCase")]
pub enum ProxyGroupType {
    /// 选择器组
    Selector,
    /// URL 测试组
    UrlTest,
    /// 故障转移组
    Fallback,
    /// 负载均衡组
    LoadBalance,
    /// 中继组
    Relay,
}

/// 规则类型枚举
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "UPPERCASE")]
pub enum RuleType {
    /// 域名规则
    Domain,
    /// 域名后缀规则
    DomainSuffix,
    /// 域名关键字规则
    DomainKeyword,
    /// GEOIP 规则
    Geoip,
    /// IP-CIDR 规则
    IpCidr,
    /// SRC-IP-CIDR 规则
    SrcIpCidr,
    /// SRC-PORT 规则
    SrcPort,
    /// DST-PORT 规则
    DstPort,
    /// 进程名规则
    ProcessName,
    /// 进程路径规则
    ProcessPath,
    /// 脚本规则
    Script,
    /// 规则集规则
    RuleSet,
    /// 匹配所有
    Match,
}

/// 规则信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rule {
    /// 规则类型
    #[serde(rename = "type")]
    pub rule_type: RuleType,
    /// 规则内容
    pub payload: String,
    /// 目标代理
    pub proxy: String,
    /// 规则大小（字节）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<u64>,
}

/// 连接信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Connection {
    /// 连接ID
    pub id: String,
    /// 元数据
    pub metadata: ConnectionMetadata,
    /// 上传字节数
    pub upload: u64,
    /// 下载字节数
    pub download: u64,
    /// 开始时间
    pub start: DateTime<Utc>,
    /// 规则链
    pub chains: Vec<String>,
    /// 规则
    pub rule: String,
    /// 规则载荷
    #[serde(rename = "rulePayload")]
    pub rule_payload: String,
}

/// 连接元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionMetadata {
    /// 网络类型
    pub network: String,
    /// 连接类型
    #[serde(rename = "type")]
    pub connection_type: String,
    /// 源IP
    #[serde(rename = "sourceIP")]
    pub source_ip: String,
    /// 目标IP
    #[serde(rename = "destinationIP")]
    pub destination_ip: String,
    /// 源端口
    #[serde(rename = "sourcePort")]
    pub source_port: String,
    /// 目标端口
    #[serde(rename = "destinationPort")]
    pub destination_port: String,
    /// 主机名
    pub host: String,
    /// DNS 模式
    #[serde(rename = "dnsMode")]
    pub dns_mode: String,
    /// 进程路径
    #[serde(rename = "processPath")]
    pub process_path: String,
    /// 特殊代理
    #[serde(rename = "specialProxy")]
    pub special_proxy: String,
}

/// 流量统计信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Traffic {
    /// 上传速度（字节/秒）
    pub up: u64,
    /// 下载速度（字节/秒）
    pub down: u64,
}

/// 内存使用信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Memory {
    /// 已使用内存（字节）
    #[serde(rename = "inuse")]
    pub in_use: u64,
    /// 系统占用内存（字节）
    #[serde(rename = "oslimit")]
    pub os_limit: u64,
}

/// 系统版本信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Version {
    /// 版本号
    pub version: String,
    /// 高级版本
    pub premium: bool,
    /// 元数据
    pub meta: bool,
}

/// API 响应包装器
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    /// 响应数据
    #[serde(flatten)]
    pub data: T,
}

/// 空响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmptyResponse {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proxy_type_serialization() {
        let proxy_type = ProxyType::Socks5;
        let json = serde_json::to_string(&proxy_type).unwrap();
        assert_eq!(json, "\"socks5\"");
        
        let deserialized: ProxyType = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, ProxyType::Socks5);
    }

    #[test]
    fn test_proxy_node_creation() {
        let node = ProxyNode {
            name: "test-proxy".to_string(),
            proxy_type: ProxyType::Http,
            server: "127.0.0.1".to_string(),
            port: 8080,
            udp: false,
            delay: Some(100),
            history: vec![],
            extra: HashMap::new(),
        };
        
        assert_eq!(node.name, "test-proxy");
        assert_eq!(node.proxy_type, ProxyType::Http);
    }
}