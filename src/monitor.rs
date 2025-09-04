//! 监控模块
//! 
//! 提供 mihomo 服务的运行状态监控、性能统计和健康检查功能。

use crate::client::MihomoClient;
use crate::error::{MihomoError, Result};
use crate::types::{Connection, Memory, Traffic, Version};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio::time;

/// 监控管理器
#[derive(Debug)]
pub struct Monitor {
    /// mihomo 客户端
    client: MihomoClient,
    /// 监控配置
    config: MonitorConfig,
    /// 历史数据存储
    history: MonitorHistory,
    /// 监控状态
    is_running: bool,
}

/// 监控配置
#[derive(Debug, Clone)]
pub struct MonitorConfig {
    /// 监控间隔（秒）
    pub interval: Duration,
    /// 历史数据保留时间（秒）
    pub history_retention: Duration,
    /// 是否启用连接监控
    pub enable_connection_monitor: bool,
    /// 是否启用流量监控
    pub enable_traffic_monitor: bool,
    /// 是否启用内存监控
    pub enable_memory_monitor: bool,
    /// 连接数阈值告警
    pub connection_threshold: Option<usize>,
    /// 内存使用阈值告警（字节）
    pub memory_threshold: Option<u64>,
    /// 流量速度阈值告警（字节/秒）
    pub traffic_threshold: Option<u64>,
}

/// 监控历史数据
#[derive(Debug, Clone)]
pub struct MonitorHistory {
    /// 流量历史
    pub traffic_history: Vec<TrafficSnapshot>,
    /// 内存历史
    pub memory_history: Vec<MemorySnapshot>,
    /// 连接数历史
    pub connection_history: Vec<ConnectionSnapshot>,
    /// 系统事件历史
    pub events: Vec<MonitorEvent>,
}

/// 流量快照
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrafficSnapshot {
    /// 时间戳
    pub timestamp: DateTime<Utc>,
    /// 上传速度（字节/秒）
    pub upload_speed: u64,
    /// 下载速度（字节/秒）
    pub download_speed: u64,
    /// 累计上传（字节）
    pub total_upload: u64,
    /// 累计下载（字节）
    pub total_download: u64,
}

/// 内存快照
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemorySnapshot {
    /// 时间戳
    pub timestamp: DateTime<Utc>,
    /// 已使用内存（字节）
    pub used_memory: u64,
    /// 系统限制（字节）
    pub memory_limit: u64,
    /// 使用率（百分比）
    pub usage_percentage: f64,
}

/// 连接快照
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionSnapshot {
    /// 时间戳
    pub timestamp: DateTime<Utc>,
    /// 活跃连接数
    pub active_connections: usize,
    /// 按代理分组的连接数
    pub connections_by_proxy: HashMap<String, usize>,
    /// 按协议分组的连接数
    pub connections_by_protocol: HashMap<String, usize>,
}

/// 监控事件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitorEvent {
    /// 时间戳
    pub timestamp: DateTime<Utc>,
    /// 事件类型
    pub event_type: EventType,
    /// 事件级别
    pub level: EventLevel,
    /// 事件消息
    pub message: String,
    /// 相关数据
    pub data: Option<serde_json::Value>,
}

/// 事件类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum EventType {
    /// 系统启动
    SystemStart,
    /// 系统停止
    SystemStop,
    /// 配置变更
    ConfigChange,
    /// 代理切换
    ProxySwitch,
    /// 连接异常
    ConnectionAnomaly,
    /// 内存告警
    MemoryAlert,
    /// 流量告警
    TrafficAlert,
    /// 健康检查失败
    HealthCheckFailed,
    /// 性能告警
    PerformanceAlert,
}

/// 事件级别
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum EventLevel {
    /// 调试
    Debug,
    /// 信息
    Info,
    /// 警告
    Warning,
    /// 错误
    Error,
    /// 严重错误
    Critical,
}

/// 系统状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemStatus {
    /// 版本信息
    pub version: Version,
    /// 当前流量
    pub traffic: Traffic,
    /// 内存使用
    pub memory: Memory,
    /// 活跃连接数
    pub active_connections: usize,
    /// 系统运行时间
    pub uptime: Duration,
    /// 健康状态
    pub health: HealthStatus,
}

/// 健康状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum HealthStatus {
    /// 健康
    Healthy,
    /// 警告
    Warning,
    /// 不健康
    Unhealthy,
    /// 未知
    Unknown,
}

/// 性能统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceStats {
    /// 平均响应时间（毫秒）
    pub avg_response_time: f64,
    /// 最大响应时间（毫秒）
    pub max_response_time: u64,
    /// 最小响应时间（毫秒）
    pub min_response_time: u64,
    /// 成功率（百分比）
    pub success_rate: f64,
    /// 错误率（百分比）
    pub error_rate: f64,
    /// 吞吐量（请求/秒）
    pub throughput: f64,
}

impl Monitor {
    /// 创建新的监控器
    /// 
    /// # Arguments
    /// 
    /// * `client` - mihomo 客户端实例
    /// 
    /// # Examples
    /// 
    /// ```no_run
    /// # use mihomo_rs::{client::MihomoClient, monitor::Monitor};
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = MihomoClient::new("http://127.0.0.1:9090", None)?;
    /// let monitor = Monitor::new(client);
    /// # Ok(())
    /// # }
    /// ```
    pub fn new(client: MihomoClient) -> Self {
        Self {
            client,
            config: MonitorConfig::default(),
            history: MonitorHistory::new(),
            is_running: false,
        }
    }

    /// 使用自定义配置创建监控器
    pub fn with_config(client: MihomoClient, config: MonitorConfig) -> Self {
        Self {
            client,
            config,
            history: MonitorHistory::new(),
            is_running: false,
        }
    }

    /// 启动监控
    pub async fn start(&mut self) -> Result<()> {
        if self.is_running {
            return Err(MihomoError::internal("Monitor is already running"));
        }

        self.is_running = true;
        self.add_event(EventType::SystemStart, EventLevel::Info, "Monitor started".to_string(), None);
        
        log::info!("Monitor started with interval: {:?}", self.config.interval);
        
        // 启动监控循环
        self.monitor_loop().await
    }

    /// 停止监控
    pub fn stop(&mut self) {
        self.is_running = false;
        self.add_event(EventType::SystemStop, EventLevel::Info, "Monitor stopped".to_string(), None);
        log::info!("Monitor stopped");
    }

    /// 监控循环
    async fn monitor_loop(&mut self) -> Result<()> {
        let mut interval = time::interval(self.config.interval);
        
        while self.is_running {
            interval.tick().await;
            
            if let Err(e) = self.collect_metrics().await {
                log::error!("Failed to collect metrics: {}", e);
                self.add_event(
                    EventType::HealthCheckFailed,
                    EventLevel::Error,
                    format!("Metrics collection failed: {}", e),
                    None,
                );
            }
            
            // 清理过期数据
            self.cleanup_history();
        }
        
        Ok(())
    }

    /// 收集监控指标
    async fn collect_metrics(&mut self) -> Result<()> {
        let now = Utc::now();
        
        // 收集流量数据
        if self.config.enable_traffic_monitor {
            if let Ok(traffic) = self.client.traffic().await {
                let snapshot = TrafficSnapshot {
                    timestamp: now,
                    upload_speed: traffic.up,
                    download_speed: traffic.down,
                    total_upload: 0, // 需要累计计算
                    total_download: 0, // 需要累计计算
                };
                
                self.history.traffic_history.push(snapshot);
                
                // 检查流量阈值
                if let Some(threshold) = self.config.traffic_threshold {
                    if traffic.up > threshold || traffic.down > threshold {
                        self.add_event(
                            EventType::TrafficAlert,
                            EventLevel::Warning,
                            format!("High traffic detected: up={}, down={}", traffic.up, traffic.down),
                            Some(serde_json::to_value(&traffic).unwrap()),
                        );
                    }
                }
            }
        }
        
        // 收集内存数据
        if self.config.enable_memory_monitor {
            if let Ok(memory) = self.client.memory().await {
                let usage_percentage = if memory.os_limit > 0 {
                    (memory.in_use as f64 / memory.os_limit as f64) * 100.0
                } else {
                    0.0
                };
                
                let snapshot = MemorySnapshot {
                    timestamp: now,
                    used_memory: memory.in_use,
                    memory_limit: memory.os_limit,
                    usage_percentage,
                };
                
                self.history.memory_history.push(snapshot);
                
                // 检查内存阈值
                if let Some(threshold) = self.config.memory_threshold {
                    if memory.in_use > threshold {
                        self.add_event(
                            EventType::MemoryAlert,
                            EventLevel::Warning,
                            format!("High memory usage: {} bytes ({}%)", memory.in_use, usage_percentage),
                            Some(serde_json::to_value(&memory).unwrap()),
                        );
                    }
                }
            }
        }
        
        // 收集连接数据
        if self.config.enable_connection_monitor {
            if let Ok(connections) = self.client.connections().await {
                let mut connections_by_proxy = HashMap::new();
                let mut connections_by_protocol = HashMap::new();
                
                for conn in &connections {
                    // 统计代理使用情况
                    if !conn.chains.is_empty() {
                        *connections_by_proxy.entry(conn.chains[0].clone()).or_insert(0) += 1;
                    }
                    
                    // 统计协议使用情况
                    *connections_by_protocol.entry(conn.metadata.network.clone()).or_insert(0) += 1;
                }
                
                let snapshot = ConnectionSnapshot {
                    timestamp: now,
                    active_connections: connections.len(),
                    connections_by_proxy,
                    connections_by_protocol,
                };
                
                self.history.connection_history.push(snapshot);
                
                // 检查连接数阈值
                if let Some(threshold) = self.config.connection_threshold {
                    if connections.len() > threshold {
                        self.add_event(
                            EventType::ConnectionAnomaly,
                            EventLevel::Warning,
                            format!("High connection count: {}", connections.len()),
                            Some(serde_json::json!({"count": connections.len()})),
                        );
                    }
                }
            }
        }
        
        Ok(())
    }

    /// 获取当前系统状态
    pub async fn get_system_status(&self) -> Result<SystemStatus> {
        let version = self.client.version().await?;
        let traffic = self.client.traffic().await?;
        let memory = self.client.memory().await?;
        let connections = self.client.connections().await?;
        
        // 计算健康状态
        let health = self.calculate_health_status(&traffic, &memory, connections.len());
        
        Ok(SystemStatus {
            version,
            traffic,
            memory,
            active_connections: connections.len(),
            uptime: Duration::from_secs(0), // 需要从服务获取
            health,
        })
    }

    /// 计算健康状态
    fn calculate_health_status(&self, traffic: &Traffic, memory: &Memory, connection_count: usize) -> HealthStatus {
        let mut warnings = 0;
        let mut errors = 0;
        
        // 检查内存使用
        if let Some(threshold) = self.config.memory_threshold {
            if memory.in_use > threshold {
                if memory.in_use > threshold * 2 {
                    errors += 1;
                } else {
                    warnings += 1;
                }
            }
        }
        
        // 检查连接数
        if let Some(threshold) = self.config.connection_threshold {
            if connection_count > threshold {
                if connection_count > threshold * 2 {
                    errors += 1;
                } else {
                    warnings += 1;
                }
            }
        }
        
        // 检查流量
        if let Some(threshold) = self.config.traffic_threshold {
            if traffic.up > threshold || traffic.down > threshold {
                warnings += 1;
            }
        }
        
        if errors > 0 {
            HealthStatus::Unhealthy
        } else if warnings > 0 {
            HealthStatus::Warning
        } else {
            HealthStatus::Healthy
        }
    }

    /// 获取性能统计
    pub fn get_performance_stats(&self, duration: Duration) -> PerformanceStats {
        let cutoff_time = Utc::now() - chrono::Duration::from_std(duration).unwrap();
        
        // 从历史数据计算性能统计
        let recent_events: Vec<_> = self.history.events
            .iter()
            .filter(|e| e.timestamp > cutoff_time)
            .collect();
        
        let total_events = recent_events.len() as f64;
        let error_events = recent_events
            .iter()
            .filter(|e| e.level >= EventLevel::Error)
            .count() as f64;
        
        PerformanceStats {
            avg_response_time: 0.0, // 需要实际测量
            max_response_time: 0,
            min_response_time: 0,
            success_rate: if total_events > 0.0 {
                ((total_events - error_events) / total_events) * 100.0
            } else {
                100.0
            },
            error_rate: if total_events > 0.0 {
                (error_events / total_events) * 100.0
            } else {
                0.0
            },
            throughput: 0.0, // 需要实际测量
        }
    }

    /// 添加监控事件
    fn add_event(&mut self, event_type: EventType, level: EventLevel, message: String, data: Option<serde_json::Value>) {
        let event = MonitorEvent {
            timestamp: Utc::now(),
            event_type,
            level,
            message,
            data,
        };
        
        self.history.events.push(event);
        
        // 限制事件数量
        if self.history.events.len() > 1000 {
            self.history.events.remove(0);
        }
    }

    /// 清理过期历史数据
    fn cleanup_history(&mut self) {
        let cutoff_time = Utc::now() - chrono::Duration::from_std(self.config.history_retention).unwrap();
        
        self.history.traffic_history.retain(|s| s.timestamp > cutoff_time);
        self.history.memory_history.retain(|s| s.timestamp > cutoff_time);
        self.history.connection_history.retain(|s| s.timestamp > cutoff_time);
        self.history.events.retain(|e| e.timestamp > cutoff_time);
    }

    /// 获取历史数据
    pub fn get_history(&self) -> &MonitorHistory {
        &self.history
    }

    /// 获取最近的事件
    pub fn get_recent_events(&self, count: usize) -> Vec<&MonitorEvent> {
        self.history.events
            .iter()
            .rev()
            .take(count)
            .collect()
    }

    /// 获取指定级别的事件
    pub fn get_events_by_level(&self, level: EventLevel) -> Vec<&MonitorEvent> {
        self.history.events
            .iter()
            .filter(|e| e.level >= level)
            .collect()
    }
}

impl Default for MonitorConfig {
    fn default() -> Self {
        Self {
            interval: Duration::from_secs(10),
            history_retention: Duration::from_secs(3600), // 1小时
            enable_connection_monitor: true,
            enable_traffic_monitor: true,
            enable_memory_monitor: true,
            connection_threshold: Some(1000),
            memory_threshold: Some(1024 * 1024 * 1024), // 1GB
            traffic_threshold: Some(100 * 1024 * 1024),  // 100MB/s
        }
    }
}

impl MonitorHistory {
    fn new() -> Self {
        Self {
            traffic_history: Vec::new(),
            memory_history: Vec::new(),
            connection_history: Vec::new(),
            events: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MihomoClient;

    #[test]
    fn test_monitor_creation() {
        let client = MihomoClient::new("http://127.0.0.1:9090", None).unwrap();
        let monitor = Monitor::new(client);
        assert!(!monitor.is_running);
        assert_eq!(monitor.config.interval, Duration::from_secs(10));
    }

    #[test]
    fn test_monitor_config_default() {
        let config = MonitorConfig::default();
        assert_eq!(config.interval, Duration::from_secs(10));
        assert!(config.enable_traffic_monitor);
        assert!(config.enable_memory_monitor);
        assert!(config.enable_connection_monitor);
    }

    #[test]
    fn test_health_status_calculation() {
        let client = MihomoClient::new("http://127.0.0.1:9090", None).unwrap();
        let monitor = Monitor::new(client);
        
        let traffic = Traffic { up: 1000, down: 2000 };
        let memory = Memory { in_use: 500_000_000, os_limit: 1_000_000_000 };
        
        let health = monitor.calculate_health_status(&traffic, &memory, 100);
        assert_eq!(health, HealthStatus::Healthy);
    }

    #[test]
    fn test_event_creation() {
        let event = MonitorEvent {
            timestamp: Utc::now(),
            event_type: EventType::SystemStart,
            level: EventLevel::Info,
            message: "Test event".to_string(),
            data: None,
        };
        
        assert_eq!(event.event_type, EventType::SystemStart);
        assert_eq!(event.level, EventLevel::Info);
    }
}