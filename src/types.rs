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

/// 连接响应结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionsResponse {
    /// 下载总量
    #[serde(rename = "downloadTotal")]
    pub download_total: u64,
    /// 上传总量
    #[serde(rename = "uploadTotal")]
    pub upload_total: u64,
    /// 连接列表（可能为 null）
    pub connections: Option<Vec<Connection>>,
    /// 内存使用量
    pub memory: u64,
}

/// 空响应结构
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EmptyResponse {}

/// 日志级别枚举
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    /// 调试级别
    Debug,
    /// 信息级别
    Info,
    /// 警告级别
    Warning,
    /// 错误级别
    Error,
    /// 静默级别
    Silent,
}

/// 日志条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    /// 日志级别
    #[serde(rename = "type")]
    pub level: LogLevel,
    /// 日志内容
    pub payload: String,
    /// 时间戳
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time: Option<String>,
}

/// 提供者信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Provider {
    /// 提供者名称
    pub name: String,
    /// 提供者类型
    #[serde(rename = "type")]
    pub provider_type: String,
    /// 车辆类型
    #[serde(rename = "vehicleType")]
    pub vehicle_type: String,
    /// 代理数量
    #[serde(rename = "proxies")]
    pub proxy_count: usize,
    /// 更新时间
    #[serde(rename = "updatedAt", skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
    /// 订阅信息
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subscription_info: Option<SubscriptionInfo>,
}

/// 订阅信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriptionInfo {
    /// 上传流量
    #[serde(skip_serializing_if = "Option::is_none")]
    pub upload: Option<u64>,
    /// 下载流量
    #[serde(skip_serializing_if = "Option::is_none")]
    pub download: Option<u64>,
    /// 总流量
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total: Option<u64>,
    /// 过期时间
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expire: Option<u64>,
}

/// DNS查询记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsQuery {
    /// 查询ID
    pub id: String,
    /// 查询域名
    pub name: String,
    /// 查询类型
    #[serde(rename = "qtype")]
    pub query_type: String,
    /// 查询类
    #[serde(rename = "qclass")]
    pub query_class: String,
    /// 查询时间
    pub time: String,
    /// 客户端IP
    pub client: String,
}

/// 健康检查结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckResult {
    /// 是否健康
    pub alive: bool,
    /// 延迟（毫秒）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delay: Option<u32>,
    /// 错误信息
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// 提供者健康检查响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderHealthResponse {
    /// 提供者名称
    pub name: String,
    /// 健康检查结果
    pub proxies: HashMap<String, HealthCheckResult>,
}

/// 规则提供者信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleProvider {
    /// 提供者名称
    pub name: String,
    /// 提供者类型
    #[serde(rename = "type")]
    pub provider_type: String,
    /// 车辆类型
    #[serde(rename = "vehicleType")]
    pub vehicle_type: String,
    /// 规则数量
    #[serde(rename = "ruleCount")]
    pub rule_count: usize,
    /// 更新时间
    #[serde(rename = "updatedAt", skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
    /// 行为
    #[serde(skip_serializing_if = "Option::is_none")]
    pub behavior: Option<String>,
}

/// 规则统计信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleStats {
    /// 总规则数
    pub total: usize,
    /// 按类型分组的规则数
    pub by_type: HashMap<String, usize>,
    /// 按代理分组的规则数
    pub by_proxy: HashMap<String, usize>,
}

// ===== 服务管理相关类型 =====

/// 版本信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionInfo {
    /// 版本号
    pub version: String,
    /// 构建时间
    #[serde(rename = "buildTime")]
    pub build_time: Option<String>,
    /// Git 提交哈希
    #[serde(rename = "gitCommit")]
    pub git_commit: Option<String>,
    /// Go 版本
    #[serde(rename = "goVersion")]
    pub go_version: Option<String>,
    /// 平台信息
    pub platform: Option<String>,
}

/// 运行时信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeInfo {
    /// 运行时间（秒）
    pub uptime: u64,
    /// 内存使用情况
    pub memory: MemoryUsage,
    /// Goroutine 数量
    pub goroutines: u32,
    /// 垃圾回收统计
    pub gc: GcStats,
}

/// 内存使用情况
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryUsage {
    /// 已分配内存（字节）
    pub alloc: u64,
    /// 总分配内存（字节）
    #[serde(rename = "totalAlloc")]
    pub total_alloc: u64,
    /// 系统内存（字节）
    pub sys: u64,
    /// 堆内存（字节）
    #[serde(rename = "heapAlloc")]
    pub heap_alloc: u64,
    /// 堆系统内存（字节）
    #[serde(rename = "heapSys")]
    pub heap_sys: u64,
    /// 堆空闲内存（字节）
    #[serde(rename = "heapIdle")]
    pub heap_idle: u64,
    /// 堆使用中内存（字节）
    #[serde(rename = "heapInuse")]
    pub heap_inuse: u64,
}

/// 垃圾回收统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GcStats {
    /// GC 次数
    #[serde(rename = "numGC")]
    pub num_gc: u32,
    /// 上次 GC 时间
    #[serde(rename = "lastGC")]
    pub last_gc: u64,
    /// GC 暂停时间（纳秒）
    #[serde(rename = "pauseTotal")]
    pub pause_total: u64,
}

/// 服务配置信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceConfigInfo {
    /// 端口
    pub port: u16,
    /// SOCKS5 端口
    #[serde(rename = "socksPort")]
    pub socks_port: Option<u16>,
    /// 重定向端口
    #[serde(rename = "redirPort")]
    pub redir_port: Option<u16>,
    /// TProxy 端口
    #[serde(rename = "tproxyPort")]
    pub tproxy_port: Option<u16>,
    /// 混合端口
    #[serde(rename = "mixedPort")]
    pub mixed_port: Option<u16>,
    /// 允许局域网连接
    #[serde(rename = "allowLan")]
    pub allow_lan: bool,
    /// 绑定地址
    #[serde(rename = "bindAddress")]
    pub bind_address: String,
    /// 模式
    pub mode: String,
    /// 日志级别
    #[serde(rename = "logLevel")]
    pub log_level: String,
    /// IPv6 支持
    pub ipv6: bool,
}

/// 服务配置更新
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceConfigUpdate {
    /// 端口
    pub port: Option<u16>,
    /// SOCKS5 端口
    #[serde(rename = "socksPort")]
    pub socks_port: Option<u16>,
    /// 重定向端口
    #[serde(rename = "redirPort")]
    pub redir_port: Option<u16>,
    /// TProxy 端口
    #[serde(rename = "tproxyPort")]
    pub tproxy_port: Option<u16>,
    /// 混合端口
    #[serde(rename = "mixedPort")]
    pub mixed_port: Option<u16>,
    /// 允许局域网连接
    #[serde(rename = "allowLan")]
    pub allow_lan: Option<bool>,
    /// 绑定地址
    #[serde(rename = "bindAddress")]
    pub bind_address: Option<String>,
    /// 模式
    pub mode: Option<String>,
    /// 日志级别
    #[serde(rename = "logLevel")]
    pub log_level: Option<String>,
    /// IPv6 支持
    pub ipv6: Option<bool>,
}

/// 服务统计信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceStats {
    /// 上传字节数
    pub upload: u64,
    /// 下载字节数
    pub download: u64,
    /// 连接数
    pub connections: u32,
    /// 上传速度（字节/秒）
    #[serde(rename = "uploadSpeed")]
    pub upload_speed: u64,
    /// 下载速度（字节/秒）
    #[serde(rename = "downloadSpeed")]
    pub download_speed: u64,
}

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
