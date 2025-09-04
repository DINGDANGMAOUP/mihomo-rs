//! 类型定义模块
//!
//! 定义了 SDK 中使用的核心数据结构和类型。

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
    /// 兼容模式代理
    #[serde(rename = "Compatible")]
    Compatible,
    /// 直连
    #[serde(rename = "Direct")]
    Direct,
    /// 拒绝连接
    #[serde(rename = "Reject")]
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server: Option<String>,
    /// 服务器端口
    #[serde(skip_serializing_if = "Option::is_none")]
    pub port: Option<u16>,
    /// 是否启用UDP
    #[serde(default)]
    pub udp: bool,
    /// 延迟信息（毫秒）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delay: Option<u32>,
    /// 历史延迟记录
    #[serde(default)]
    pub history: Vec<DelayHistory>,
    /// 是否存活
    #[serde(default)]
    pub alive: bool,
    /// 额外配置
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
    /// 拨号代理
    #[serde(rename = "dialer-proxy", default)]
    pub dialer_proxy: String,
    /// 接口
    #[serde(default)]
    pub interface: String,
    /// MPTCP支持
    #[serde(default)]
    pub mptcp: bool,
    /// 路由标记
    #[serde(rename = "routing-mark", default)]
    pub routing_mark: u32,
    /// SMUX支持
    #[serde(default)]
    pub smux: bool,
    /// TCP Fast Open
    #[serde(default)]
    pub tfo: bool,
    /// UoT支持
    #[serde(default)]
    pub uot: bool,
    /// XUDP支持
    #[serde(default)]
    pub xudp: bool,
    /// ID
    #[serde(default)]
    pub id: String,
}

/// 延迟历史记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DelayHistory {
    /// 延迟值（毫秒）
    pub delay: u32,
    /// 测试时间戳
    #[serde(alias = "timestamp", skip_serializing_if = "Option::is_none")]
    pub time: Option<String>,
}

/// 代理组信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyGroup {
    /// 代理组名称
    pub name: String,
    /// 代理组类型
    #[serde(rename = "type")]
    pub group_type: ProxyGroupType,
    /// 当前选中的代理
    pub now: String,
    /// 所有可用代理
    pub all: Vec<String>,
    /// 历史延迟记录
    #[serde(default)]
    pub history: Vec<DelayHistory>,
    /// 是否隐藏
    #[serde(default)]
    pub hidden: bool,
    /// 图标
    #[serde(default)]
    pub icon: String,
    /// 是否存活
    #[serde(default)]
    pub alive: bool,
    /// 拨号代理
    #[serde(rename = "dialer-proxy", default)]
    pub dialer_proxy: String,
    /// 额外信息
    #[serde(default)]
    pub extra: std::collections::HashMap<String, serde_json::Value>,
    /// 接口
    #[serde(default)]
    pub interface: String,
    /// MPTCP支持
    #[serde(default)]
    pub mptcp: bool,
    /// 路由标记
    #[serde(rename = "routing-mark", default)]
    pub routing_mark: u32,
    /// SMUX支持
    #[serde(default)]
    pub smux: bool,
    /// 测试URL
    #[serde(rename = "testUrl", default)]
    pub test_url: String,
    /// TCP Fast Open
    #[serde(default)]
    pub tfo: bool,
    /// UDP支持
    #[serde(default)]
    pub udp: bool,
    /// UoT支持
    #[serde(default)]
    pub uot: bool,
    /// XUDP支持
    #[serde(default)]
    pub xudp: bool,
}

/// 通用代理项（可能是代理节点或代理组）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyItem {
    /// 名称
    pub name: String,
    /// 类型
    #[serde(rename = "type")]
    pub item_type: String,
    /// 是否存活
    #[serde(default)]
    pub alive: bool,
    /// 历史延迟记录
    #[serde(default)]
    pub history: Vec<DelayHistory>,
    /// 拨号代理
    #[serde(rename = "dialer-proxy", default)]
    pub dialer_proxy: String,
    /// 接口
    #[serde(default)]
    pub interface: String,
    /// MPTCP支持
    #[serde(default)]
    pub mptcp: bool,
    /// 路由标记
    #[serde(rename = "routing-mark", default)]
    pub routing_mark: u32,
    /// SMUX支持
    #[serde(default)]
    pub smux: bool,
    /// TCP Fast Open
    #[serde(default)]
    pub tfo: bool,
    /// UDP支持
    #[serde(default)]
    pub udp: bool,
    /// UoT支持
    #[serde(default)]
    pub uot: bool,
    /// XUDP支持
    #[serde(default)]
    pub xudp: bool,
    /// ID（代理节点特有）
    #[serde(default)]
    pub id: String,
    /// 服务器地址（代理节点特有）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server: Option<String>,
    /// 端口号（代理节点特有）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub port: Option<u16>,
    /// 延迟信息（代理节点特有）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delay: Option<u32>,
    /// 当前选中的代理（代理组特有）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub now: Option<String>,
    /// 所有可用代理（代理组特有）
    #[serde(default)]
    pub all: Vec<String>,
    /// 是否隐藏（代理组特有）
    #[serde(default)]
    pub hidden: bool,
    /// 图标（代理组特有）
    #[serde(default)]
    pub icon: String,
    /// 测试URL（代理组特有）
    #[serde(rename = "testUrl", default)]
    pub test_url: String,
    /// 额外配置
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

impl ProxyItem {
    /// 判断是否为代理组（通过all字段是否为空来判断）
    pub fn is_group(&self) -> bool {
        !self.all.is_empty()
    }

    /// 判断是否为代理节点
    pub fn is_node(&self) -> bool {
        self.all.is_empty()
    }

    /// 转换为代理节点
    pub fn to_proxy_node(&self) -> Option<ProxyNode> {
        if self.is_node() {
            Some(ProxyNode {
                name: self.name.clone(),
                proxy_type: serde_json::from_str(&format!("\"{}\"", self.item_type)).ok()?,
                server: self.server.clone(),
                port: self.port,
                udp: self.udp,
                delay: self.delay,
                history: self.history.clone(),
                alive: self.alive,
                dialer_proxy: self.dialer_proxy.clone(),
                interface: self.interface.clone(),
                mptcp: self.mptcp,
                routing_mark: self.routing_mark,
                smux: self.smux,
                tfo: self.tfo,
                uot: self.uot,
                xudp: self.xudp,
                id: self.id.clone(),
                extra: self.extra.clone(),
            })
        } else {
            None
        }
    }

    /// 转换为代理组
    pub fn to_proxy_group(&self) -> Option<ProxyGroup> {
        if self.is_group() {
            Some(ProxyGroup {
                name: self.name.clone(),
                group_type: serde_json::from_str(&format!("\"{}\"", self.item_type)).ok()?,
                now: self.now.clone().unwrap_or_default(),
                all: self.all.clone(),
                history: self.history.clone(),
                hidden: self.hidden,
                icon: self.icon.clone(),
                alive: self.alive,
                dialer_proxy: self.dialer_proxy.clone(),
                extra: self.extra.clone(),
                interface: self.interface.clone(),
                mptcp: self.mptcp,
                routing_mark: self.routing_mark,
                smux: self.smux,
                test_url: self.test_url.clone(),
                tfo: self.tfo,
                udp: self.udp,
                uot: self.uot,
                xudp: self.xudp,
            })
        } else {
            None
        }
    }
}

/// 代理组类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ProxyGroupType {
    /// 选择器
    #[serde(rename = "Selector")]
    Selector,
    /// URL测试
    #[serde(rename = "URLTest")]
    UrlTest,
    /// 故障转移
    #[serde(rename = "Fallback")]
    Fallback,
    /// 负载均衡
    #[serde(rename = "LoadBalance")]
    LoadBalance,
    /// 中继
    #[serde(rename = "Relay")]
    Relay,
}

/// 规则类型枚举
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum RuleType {
    /// 域名规则
    #[serde(rename = "DOMAIN")]
    Domain,
    /// 域名后缀规则
    #[serde(rename = "DOMAIN-SUFFIX")]
    DomainSuffix,
    /// 域名关键字规则
    #[serde(rename = "DOMAIN-KEYWORD")]
    DomainKeyword,
    /// GEOIP 规则
    #[serde(rename = "GEOIP")]
    Geoip,
    /// IP-CIDR 规则
    #[serde(rename = "IP-CIDR")]
    IpCidr,
    /// SRC-IP-CIDR 规则
    #[serde(rename = "SRC-IP-CIDR")]
    SrcIpCidr,
    /// SRC-PORT 规则
    #[serde(rename = "SRC-PORT")]
    SrcPort,
    /// DST-PORT 规则
    #[serde(rename = "DST-PORT")]
    DstPort,
    /// 进程名规则
    #[serde(rename = "PROCESS-NAME")]
    ProcessName,
    /// 进程路径规则
    #[serde(rename = "PROCESS-PATH")]
    ProcessPath,
    /// 脚本规则
    #[serde(rename = "SCRIPT")]
    Script,
    /// 规则集规则
    #[serde(rename = "RULE-SET")]
    RuleSet,
    /// 匹配所有
    #[serde(rename = "Match")]
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
    pub size: i64,
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
    #[serde(default)]
    pub premium: bool,
    /// 元数据
    pub meta: bool,
}

/// 系统信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemInfo {
    /// 版本信息
    pub version: String,
    /// 元数据标识
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
            server: Some("127.0.0.1".to_string()),
            port: Some(8080),
            udp: false,
            delay: Some(100),
            history: vec![],
            alive: false,
            dialer_proxy: String::new(),
            interface: String::new(),
            mptcp: false,
            routing_mark: 0,
            smux: false,
            tfo: false,
            uot: false,
            xudp: false,
            id: String::new(),
            extra: HashMap::new(),
        };

        assert_eq!(node.name, "test-proxy");
        assert_eq!(node.proxy_type, ProxyType::Http);
        assert_eq!(node.server, Some("127.0.0.1".to_string()));
        assert_eq!(node.port, Some(8080));
    }
}
