//! 基本使用示例
//!
//! 演示如何使用 mihomo-rs SDK 的基本功能

use std::alloc::System;
use mihomo_rs::{
    client::MihomoClient,
    config::ConfigManager,
    monitor::Monitor,
    proxy::ProxyManager,
    rules::RuleEngine,
    types::*,
    utils::format_utils,
    MihomoError,
};
use std::time::Duration;
use tokio::time;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志
    mihomo_rs::init_logger();
    
    println!("=== Mihomo SDK 基本使用示例 ===");
    
    // 1. 创建客户端
    println!("\n1. 创建 Mihomo 客户端...");
    let client = MihomoClient::new("http://127.0.0.1:9090", Some("your-secret".to_string()))?;
    
    // 2. 获取版本信息
    println!("\n2. 获取版本信息...");
    match client.version().await {
        Ok(version) => {
            println!("   版本: {}", version.version);
            println!("   高级版本: {}", version.premium);
        }
        Err(e) => println!("   获取版本信息失败: {}", e),
    }
    
    // 3. 获取系统状态
    println!("\n3. 获取系统状态...");
    get_system_status(&client).await;
    
    // 4. 代理管理示例
    println!("\n4. 代理管理示例...");
    proxy_management_example(&client).await?;
    
    // 5. 规则管理示例
    println!("\n5. 规则管理示例...");
    rule_management_example(&client).await?;
    
    // 6. 配置管理示例
    println!("\n6. 配置管理示例...");
    config_management_example().await;
    
    // 7. 监控示例
    println!("\n7. 监控示例...");
    monitoring_example(&client).await;
    
    println!("\n=== 示例完成 ===");
    Ok(())
}

/// 获取系统状态
async fn get_system_status(client: &MihomoClient) {
    // 获取流量信息
    match client.traffic().await {
        Ok(traffic) => {
            println!("   上传速度: {}", format_utils::format_speed(traffic.up));
            println!("   下载速度: {}", format_utils::format_speed(traffic.down));
        }
        Err(e) => println!("   获取流量信息失败: {}", e),
    }
    
    // 获取内存使用情况
    match client.memory().await {
        Ok(memory) => {
            println!("   内存使用: {}", format_utils::format_bytes(memory.in_use));
            println!("   系统限制: {}", format_utils::format_bytes(memory.os_limit));
            let usage_percent = if memory.os_limit > 0 {
                (memory.in_use as f64 / memory.os_limit as f64) * 100.0
            } else {
                0.0
            };
            println!("   使用率: {:.2}%", usage_percent);
        }
        Err(e) => println!("   获取内存信息失败: {}", e),
    }
    
    // 获取连接信息
    match client.connections().await {
        Ok(connections) => {
            println!("   活跃连接数: {}", connections.len());
            if !connections.is_empty() {
                println!("   最近连接:");
                for (i, conn) in connections.iter().take(3).enumerate() {
                    println!("     {}. {} -> {} ({})", 
                        i + 1, 
                        conn.metadata.source_ip, 
                        conn.metadata.destination_ip, 
                        conn.metadata.network
                    );
                }
            }
        }
        Err(e) => println!("   获取连接信息失败: {}", e),
    }
}

/// 代理管理示例
async fn proxy_management_example(client: &MihomoClient) -> Result<(), MihomoError> {
    let mut proxy_manager = ProxyManager::new(client.clone());
    
    // 刷新代理列表
    match proxy_manager.get_proxies().await {
        Ok(_) => println!("   代理列表刷新成功"),
        Err(e) => println!("   代理列表刷新失败: {}", e),
    }
    
    // 获取所有代理
    let proxies = proxy_manager.get_proxies().await?;
    println!("   代理数量: {}", proxies.len());
    
    // 显示前几个代理
    println!("   前5个代理:");
    for (i, (name, proxy)) in proxies.iter().take(5).enumerate() {
        println!("     {}. {} ({:?})", i + 1, name, proxy.proxy_type);
    }
    
    // 克隆第一个代理名称用于后续测试
    let first_proxy_name = proxies.iter().next().map(|(name, _)| name.clone());
    
    // 获取代理组
    let groups = proxy_manager.get_proxy_groups().await?;
    println!("   代理组数量: {}", groups.len());
    
    for (name, group) in groups.iter().take(3) {
        println!("     组: {} (类型: {:?}, 代理数: {})", 
            name, group.group_type, group.all.len());
    }
    
    // 测试代理延迟
    if let Some(proxy_name) = first_proxy_name {
        println!("   测试代理 '{}' 的延迟...", proxy_name);
        let _ = match proxy_manager.test_proxy_delay(&proxy_name, Some("http://www.gstatic.com/generate_204"), Some(5000)).await {
            Ok(delay) => {
                println!("     延迟: {}", format_utils::format_latency(delay.delay as u64));
            }
            Err(e) => {
                println!("     延迟测试失败: {}", e);
            }
        };
    }
    
    Ok(())
}

/// 规则管理示例
async fn rule_management_example(client: &MihomoClient) -> Result<(), MihomoError> {
    let mut rule_engine = RuleEngine::new(client.clone());
    
    // 刷新规则列表
    match rule_engine.refresh_rules().await {
        Ok(_) => println!("   规则列表刷新成功"),
        Err(e) => println!("   规则列表刷新失败: {}", e),
    }
    
    // 获取所有规则
    let rules = rule_engine.get_rules().await?;
    println!("   规则数量: {}", rules.len());
    
    // 显示前几个规则
    println!("   前5个规则:");
    for (i, rule) in rules.iter().take(5).enumerate() {
        println!("     {}. {:?}: {} -> {}", 
            i + 1, rule.rule_type, rule.payload, rule.proxy);
    }
    
    // 测试规则匹配
    let test_cases = vec![
        ("example.com", 80),
        ("google.com", 443),
        ("192.168.1.1", 22),
    ];
    
    println!("   测试规则匹配:");
    for (host, port) in test_cases {
        match rule_engine.match_rule(host, Some(port), None).await {
            Ok(Some((rule, proxy))) => println!("     {} -> {} (规则: {:?})", host, proxy, rule.rule_type),
            Ok(None) => println!("     {} -> DIRECT (无匹配规则)", host),
            Err(e) => println!("     {} -> 规则匹配错误: {}", host, e),
        }
    }
    
    Ok(())
}

/// 配置管理示例
async fn config_management_example() {
    let mut config_manager = ConfigManager::new();
    
    // 获取默认配置并克隆
    let mut config = config_manager.config().clone();
    
    // 修改一些配置
    config.port = 7890;
    config.socks_port = 7891;
    config.allow_lan = true;
    config.mode = "rule".to_string();
    config.log_level = "info".to_string();
    
    // 配置 DNS
    config.dns = Some(mihomo_rs::config::DnsConfig {
        enable: true,
        listen: "0.0.0.0:53".to_string(),
        nameserver: vec![
            "8.8.8.8".to_string(),
            "8.8.4.4".to_string(),
        ],
        fallback: vec![],
        enhanced_mode: "fake-ip".to_string(),
        fake_ip_range: "198.18.0.1/16".to_string(),
    });
    
    // 验证配置
    match config_manager.load_from_str(&serde_yaml::to_string(&config).unwrap()) {
        Ok(_) => println!("   配置验证通过"),
        Err(e) => println!("   配置验证失败: {}", e),
    }
    
    // 保存配置到文件
    let config_path = "/tmp/mihomo_example_config.yaml";
    match config_manager.save_to_file(config_path) {
        Ok(_) => println!("   配置已保存到: {}", config_path),
        Err(e) => println!("   保存配置失败: {}", e),
    }
    
    // 从文件加载配置
    match config_manager.load_from_file(config_path) {
        Ok(_) => {
            println!("   从文件加载配置成功");
            let loaded_config = config_manager.config();
            println!("     端口: {}", loaded_config.port);
            println!("     模式: {}", loaded_config.mode);
            println!("     日志级别: {}", loaded_config.log_level);
        }
        Err(e) => println!("   从文件加载配置失败: {}", e),
    }
}

/// 监控示例
async fn monitoring_example(client: &MihomoClient) {
    let monitor = Monitor::new(client.clone());
    
    // 获取系统状态
    match monitor.get_system_status().await {
        Ok(status) => {
            println!("   系统状态:");
            println!("     版本: {}", status.version.version);
            println!("     健康状态: {:?}", status.health);
            println!("     活跃连接: {}", status.active_connections);
            println!("     内存使用: {}", format_utils::format_bytes(status.memory.in_use));
            println!("     上传速度: {}", format_utils::format_speed(status.traffic.up));
            println!("     下载速度: {}", format_utils::format_speed(status.traffic.down));
        }
        Err(e) => println!("   获取系统状态失败: {}", e),
    }
    
    // 获取性能统计
    let stats = monitor.get_performance_stats(Duration::from_secs(3600));
    println!("   性能统计 (过去1小时):");
    println!("     成功率: {:.2}%", stats.success_rate);
    println!("     错误率: {:.2}%", stats.error_rate);
    println!("     平均响应时间: {:.2}ms", stats.avg_response_time);
    
    // 获取最近事件
    let recent_events = monitor.get_recent_events(5);
    if !recent_events.is_empty() {
        println!("   最近事件:");
        for (i, event) in recent_events.iter().enumerate() {
            println!("     {}. [{:?}] {:?}: {}", 
                i + 1, event.level, event.event_type, event.message);
        }
    } else {
        println!("   暂无事件记录");
    }
}

/// 错误处理示例
#[allow(dead_code)]
fn error_handling_example() {
    println!("\n=== 错误处理示例 ===");
    
    // 创建各种类型的错误
    let errors = vec![
        MihomoError::network("HTTP 请求失败".to_string()),
        MihomoError::internal("JSON 解析错误".to_string()),
        MihomoError::config("配置文件格式错误".to_string()),
        MihomoError::proxy("代理连接失败".to_string()),
        MihomoError::timeout("请求超时".to_string()),
        MihomoError::invalid_parameter("无效参数".to_string()),
        MihomoError::not_found("资源未找到".to_string()),
    ];
    
    for (i, error) in errors.iter().enumerate() {
        println!("   {}. 错误类型: {:?}", i + 1, error);
        println!("      错误信息: {}", error);
    }
}

/// 工具函数使用示例
#[allow(dead_code)]
fn utils_example() {
    use mihomo_rs::utils::*;
    
    println!("\n=== 工具函数示例 ===");
    
    // URL 工具
    println!("\n1. URL 工具:");
    let url = "http://127.0.0.1:9090/proxies";
    match url_utils::validate_url(url) {
        Ok(parsed) => println!("   URL 验证成功: {}", parsed),
        Err(e) => println!("   URL 验证失败: {}", e),
    }
    
    // 网络工具
    println!("\n2. 网络工具:");
    let ip = "192.168.1.1";
    match network_utils::validate_ip(ip) {
        Ok(addr) => {
            println!("   IP 验证成功: {}", addr);
            println!("   是否为私有IP: {}", network_utils::is_private_ip(&addr));
        }
        Err(e) => println!("   IP 验证失败: {}", e),
    }
    
    // 字符串工具
    println!("\n3. 字符串工具:");
    let data = b"Hello, Mihomo!";
    let encoded = string_utils::base64_encode(data);
    println!("   Base64 编码: {}", encoded);
    
    match string_utils::base64_decode(&encoded) {
        Ok(decoded) => println!("   Base64 解码: {}", String::from_utf8_lossy(&decoded)),
        Err(e) => println!("   Base64 解码失败: {}", e),
    }
    
    // 时间工具
    println!("\n4. 时间工具:");
    let timestamp = time_utils::current_timestamp();
    println!("   当前时间戳: {}", timestamp);
    
    let duration = Duration::from_secs(3661);
    println!("   格式化时长: {}", time_utils::format_duration(duration));
    
    // 格式化工具
    println!("\n5. 格式化工具:");
    println!("   格式化字节: {}", format_utils::format_bytes(1048576));
    println!("   格式化速度: {}", format_utils::format_speed(1024000));
    println!("   格式化延迟: {}", format_utils::format_latency(150));
    
    // 随机工具
    println!("\n6. 随机工具:");
    let random_str = random_utils::generate_random_string(10);
    println!("   随机字符串: {}", random_str);
    
    let random_id = random_utils::generate_id();
    println!("   随机ID: {}", random_id);
}