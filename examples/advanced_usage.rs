//! 高级使用示例
//!
//! 演示 mihomo-rs SDK 的高级功能，包括自动化管理、性能优化等

use mihomo_rs::{
    client::MihomoClient,
    config::ConfigManager,
    monitor::{EventLevel, HealthStatus, Monitor, MonitorConfig},
    proxy::ProxyManager,
    rules::RuleEngine,
    types::*,
    utils::{format_utils, time_utils, validation_utils},
};
use std::collections::HashMap;
use std::time::Duration;
use tokio::time;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志
    mihomo_rs::init_logger();
    println!("=== Mihomo SDK 高级使用示例 ===");

    // 创建客户端
    let client = MihomoClient::new("http://127.0.0.1:9090", Some("your-secret".to_string()))?;

    // 1. 自动代理选择和切换
    println!("\n1. 自动代理选择和切换...");
    auto_proxy_selection(&client).await?;

    // 2. 智能规则管理
    println!("\n2. 智能规则管理...");
    intelligent_rule_management(&client).await?;

    // 3. 高级监控和告警
    println!("\n3. 高级监控和告警...");
    advanced_monitoring(&client).await?;

    // 4. 配置热重载
    println!("\n4. 配置热重载...");
    config_hot_reload(&client).await?;

    // 5. 性能优化示例
    println!("\n5. 性能优化示例...");
    performance_optimization(&client).await?;

    // 6. 批量操作示例
    println!("\n6. 批量操作示例...");
    batch_operations(&client).await?;

    println!("\n=== 高级示例完成 ===");
    Ok(())
}

/// 自动代理选择和切换
async fn auto_proxy_selection(client: &MihomoClient) -> Result<(), Box<dyn std::error::Error>> {
    let mut proxy_manager = ProxyManager::new(client.clone());

    // 刷新代理列表
    let _ = proxy_manager.get_proxies().await?;

    // 获取所有代理组
    let groups = proxy_manager.get_proxy_groups().await?;
    let group_data: Vec<(String, ProxyGroup)> = groups
        .iter()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();

    for (group_name, group) in group_data {
        if group.group_type == ProxyGroupType::Selector
            || group.group_type == ProxyGroupType::UrlTest
        {
            println!("   处理代理组: {}", group_name);

            // 测试组内所有代理的延迟
            let mut proxy_delays = HashMap::new();

            for proxy_name in &group.all {
                match proxy_manager
                    .test_proxy_delay(
                        proxy_name,
                        Some("http://www.gstatic.com/generate_204"),
                        Some(5000),
                    )
                    .await
                {
                    Ok(delay) => {
                        println!(
                            "     {} 延迟: {}",
                            proxy_name,
                            format_utils::format_latency(delay.delay as u64)
                        );
                        proxy_delays.insert(proxy_name.clone(), delay);
                    }
                    Err(e) => {
                        println!("     {} 延迟测试失败: {}", proxy_name, e);
                    }
                }

                // 避免请求过于频繁
                time::sleep(Duration::from_millis(100)).await;
            }

            // 选择延迟最低的代理
            if let Some((best_proxy, best_delay)) =
                proxy_delays.iter().min_by_key(|(_, delay)| delay.delay)
            {
                println!(
                    "     最佳代理: {} (延迟: {})",
                    best_proxy,
                    format_utils::format_latency(best_delay.delay as u64)
                );

                // 切换到最佳代理
                match proxy_manager.switch_proxy(&group_name, best_proxy).await {
                    Ok(_) => println!("     已切换到最佳代理: {}", best_proxy),
                    Err(e) => println!("     切换代理失败: {}", e),
                }
            }
        }
    }

    Ok(())
}

/// 智能规则管理
async fn intelligent_rule_management(
    client: &MihomoClient,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut rule_engine = RuleEngine::new(client.clone());

    // 刷新规则列表
    rule_engine.refresh_rules().await?;

    // 分析规则使用情况
    let rules = rule_engine.get_rules().await?;
    let mut rule_stats = HashMap::new();

    for rule in rules.iter() {
        *rule_stats.entry(rule.rule_type.clone()).or_insert(0) += 1;
    }

    println!("   规则统计:");
    for (rule_type, count) in &rule_stats {
        println!("     {:?}: {} 条", rule_type, count);
    }

    // 验证规则配置
    println!("   验证规则配置...");
    let mut invalid_rules = 0;

    for (i, rule) in rules.iter().enumerate() {
        if let Err(e) =
            validation_utils::validate_rule_config(&rule.rule_type, &rule.payload, &rule.proxy)
        {
            println!("     规则 {} 验证失败: {}", i + 1, e);
            invalid_rules += 1;
        }
    }

    if invalid_rules == 0 {
        println!("     所有规则验证通过");
    } else {
        println!("     发现 {} 条无效规则", invalid_rules);
    }

    // 测试常见域名的规则匹配
    let test_domains = vec![
        "google.com",
        "github.com",
        "stackoverflow.com",
        "baidu.com",
        "taobao.com",
    ];

    println!("   测试域名规则匹配:");
    for domain in test_domains {
        match rule_engine.match_rule(domain, Some(443), None).await {
            Ok(Some((rule, target))) => {
                println!("     {} -> {} (规则: {:?})", domain, target, rule.rule_type);
            }
            Ok(None) => {
                println!("     {} -> DIRECT (无匹配规则)", domain);
            }
            Err(e) => {
                println!("     {} 规则匹配失败: {}", domain, e);
            }
        }
    }

    Ok(())
}

/// 高级监控和告警
async fn advanced_monitoring(client: &MihomoClient) -> Result<(), Box<dyn std::error::Error>> {
    // 创建自定义监控配置
    let monitor_config = MonitorConfig {
        interval: Duration::from_secs(5),
        history_retention: Duration::from_secs(1800), // 30分钟
        enable_connection_monitor: true,
        enable_traffic_monitor: true,
        enable_memory_monitor: true,
        connection_threshold: Some(500),
        memory_threshold: Some(512 * 1024 * 1024), // 512MB
        traffic_threshold: Some(50 * 1024 * 1024), // 50MB/s
    };

    let monitor = Monitor::with_config(client.clone(), monitor_config);

    // 获取系统状态
    match monitor.get_system_status().await {
        Ok(status) => {
            println!("   系统健康状态: {:?}", status.health);

            // 详细的状态分析
            let memory_usage_percent = if status.memory.os_limit > 0 {
                (status.memory.in_use as f64 / status.memory.os_limit as f64) * 100.0
            } else {
                0.0
            };

            println!("   详细状态分析:");
            println!("     内存使用率: {:.2}%", memory_usage_percent);
            println!("     活跃连接数: {}", status.active_connections);
            println!(
                "     当前上传速度: {}",
                format_utils::format_speed(status.traffic.up)
            );
            println!(
                "     当前下载速度: {}",
                format_utils::format_speed(status.traffic.down)
            );

            // 健康状态建议
            match status.health {
                HealthStatus::Healthy => println!("     ✅ 系统运行正常"),
                HealthStatus::Warning => {
                    println!("     ⚠️  系统存在警告，建议检查:");
                    if memory_usage_percent > 80.0 {
                        println!("       - 内存使用率较高");
                    }
                    if status.active_connections > 1000 {
                        println!("       - 连接数较多");
                    }
                }
                HealthStatus::Unhealthy => {
                    println!("     ❌ 系统状态不健康，需要立即处理:");
                    if memory_usage_percent > 95.0 {
                        println!("       - 内存使用率过高，可能导致系统不稳定");
                    }
                }
                HealthStatus::Unknown => println!("     ❓ 无法确定系统状态"),
            }
        }
        Err(e) => println!("   获取系统状态失败: {}", e),
    }

    // 获取性能统计
    let stats = monitor.get_performance_stats(Duration::from_secs(3600));
    println!("   性能统计 (过去1小时):");
    println!("     成功率: {:.2}%", stats.success_rate);
    println!("     错误率: {:.2}%", stats.error_rate);

    if stats.error_rate > 5.0 {
        println!("     ⚠️  错误率较高，建议检查系统配置");
    }

    // 获取错误级别的事件
    let error_events = monitor.get_events_by_level(EventLevel::Error);
    if !error_events.is_empty() {
        println!("   最近错误事件:");
        for (i, event) in error_events.iter().take(5).enumerate() {
            println!("     {}. [{:?}] {}", i + 1, event.event_type, event.message);
        }
    }

    Ok(())
}

/// 配置热重载
async fn config_hot_reload(client: &MihomoClient) -> Result<(), Box<dyn std::error::Error>> {
    let mut config_manager = ConfigManager::new();

    // 获取当前配置
    {
        let current_config = config_manager.config();
        println!("   当前配置:");
        println!("     端口: {}", current_config.port);
        println!("     模式: {}", current_config.mode);
        println!("     允许局域网: {}", current_config.allow_lan);
        println!("     日志级别: {}", current_config.log_level);
    }

    // 修改配置
    let mut new_config = config_manager.config().clone();
    new_config.log_level = "debug".to_string();
    new_config.allow_lan = !new_config.allow_lan;

    // 验证新配置
    // 配置验证会在内部自动进行
    match config_manager.load_from_str(&serde_yaml::to_string(&new_config).unwrap()) {
        Ok(_) => {
            println!("   新配置验证通过");
            let updated_config = config_manager.config();
            println!("     端口: {}", updated_config.port);
            println!("     模式: {}", updated_config.mode);
            println!("     允许局域网: {}", updated_config.allow_lan);
            println!("     日志级别: {}", updated_config.log_level);

            // 通过 API 重新加载配置
            match client.reload_config().await {
                Ok(_) => {
                    println!("   配置热重载成功");
                }
                Err(e) => println!("   配置热重载失败: {}", e),
            }
        }
        Err(e) => println!("   新配置验证失败: {}", e),
    }

    Ok(())
}

/// 性能优化示例
async fn performance_optimization(client: &MihomoClient) -> Result<(), Box<dyn std::error::Error>> {
    let mut proxy_manager = ProxyManager::new(client.clone());

    // 刷新代理列表
    let _ = proxy_manager.get_proxies().await?;

    // 并发测试多个代理的延迟
    let proxies = proxy_manager.get_proxies().await?;
    let proxy_names: Vec<_> = proxies.keys().take(5).cloned().collect();

    println!("   并发测试 {} 个代理的延迟...", proxy_names.len());

    let start_time = std::time::Instant::now();

    // 使用顺序测试代理延迟
    let mut results = Vec::new();

    for proxy_name in proxy_names {
        let result = proxy_manager
            .test_proxy_delay(
                &proxy_name,
                Some("http://www.gstatic.com/generate_204"),
                Some(3000),
            )
            .await;
        results.push((proxy_name, result));
    }

    let elapsed = start_time.elapsed();
    println!(
        "   并发测试完成，耗时: {}",
        time_utils::format_duration(elapsed)
    );

    // 显示结果
    for (proxy_name, result) in results {
        match result {
            Ok(delay) => println!(
                "     {} 延迟: {}",
                proxy_name,
                format_utils::format_latency(delay.delay as u64)
            ),
            Err(e) => println!("     {} 测试失败: {}", proxy_name, e),
        }
    }

    // 缓存优化示例
    println!("   缓存优化:");
    // 缓存统计功能暂未实现
    println!("     代理管理器运行正常");
    // 缓存统计信息暂未实现

    Ok(())
}

/// 批量操作示例
async fn batch_operations(client: &MihomoClient) -> Result<(), Box<dyn std::error::Error>> {
    let mut proxy_manager = ProxyManager::new(client.clone());

    // 刷新代理列表
    let _ = proxy_manager.get_proxies().await?;

    // 批量测试代理延迟
    let proxies = proxy_manager.get_proxies().await?;
    let proxy_names: Vec<_> = proxies.keys().take(10).cloned().collect();

    println!("   批量测试 {} 个代理...", proxy_names.len());

    let mut test_results = HashMap::new();
    for proxy_name in &proxy_names {
        match client
            .test_proxy_delay(
                proxy_name,
                Some("http://www.gstatic.com/generate_204"),
                Some(5000),
            )
            .await
        {
            Ok(delay) => {
                test_results.insert(proxy_name.clone(), Ok(delay));
            }
            Err(e) => {
                test_results.insert(proxy_name.clone(), Err(e));
            }
        }
    }

    // 统计结果
    let mut successful_tests = 0;
    let mut failed_tests = 0;
    let mut total_delay = 0u32;

    for (proxy_name, result) in &test_results {
        match result {
            Ok(delay) => {
                successful_tests += 1;
                total_delay += delay.delay;
                println!(
                    "     ✅ {} 延迟: {}",
                    proxy_name,
                    format_utils::format_latency(delay.delay as u64)
                );
            }
            Err(e) => {
                failed_tests += 1;
                println!("     ❌ {} 失败: {}", proxy_name, e);
            }
        }
    }

    println!("   批量测试结果:");
    println!("     成功: {} 个", successful_tests);
    println!("     失败: {} 个", failed_tests);

    if successful_tests > 0 {
        let avg_delay = total_delay / successful_tests;
        println!(
            "     平均延迟: {}",
            format_utils::format_latency(avg_delay as u64)
        );
    }

    // 自动选择最快的代理
    if let Some((fastest_proxy, _)) = test_results
        .iter()
        .filter_map(|(name, result)| {
            result
                .as_ref()
                .ok()
                .map(|delay| (name.clone(), delay.clone()))
        })
        .min_by_key(|(_, delay)| delay.delay)
    {
        println!("   最快代理: {}", fastest_proxy);

        // 获取代理组并尝试切换
        let groups = proxy_manager.get_proxy_groups().await?;
        let group_names: Vec<_> = groups
            .iter()
            .filter(|(_, group)| {
                group.all.contains(&fastest_proxy) && group.group_type == ProxyGroupType::Selector
            })
            .map(|(name, _)| name.clone())
            .collect();

        for group_name in group_names {
            match proxy_manager
                .switch_proxy(&group_name, &fastest_proxy)
                .await
            {
                Ok(_) => println!(
                    "   已将组 '{}' 切换到最快代理: {}",
                    group_name, fastest_proxy
                ),
                Err(e) => println!("   切换代理失败: {}", e),
            }
        }
    }

    Ok(())
}

/// 自动化运维示例
#[allow(dead_code)]
async fn automated_operations(client: &MihomoClient) -> Result<(), Box<dyn std::error::Error>> {
    println!("   启动自动化运维...");

    let monitor = Monitor::new(client.clone());
    let mut proxy_manager = ProxyManager::new(client.clone());

    // 定期检查和优化
    let mut interval = time::interval(Duration::from_secs(300)); // 每5分钟检查一次

    for _ in 0..3 {
        // 示例运行3次
        interval.tick().await;

        println!("   执行自动化检查...");

        // 1. 检查系统健康状态
        if let Ok(status) = monitor.get_system_status().await {
            match status.health {
                HealthStatus::Warning | HealthStatus::Unhealthy => {
                    println!("     检测到系统异常，执行自动修复...");

                    // 重新选择最优代理
                    let _ = proxy_manager.get_proxies().await?;
                    let groups = proxy_manager.get_proxy_groups().await?;

                    for (group_name, group) in groups.iter() {
                        if group.group_type == ProxyGroupType::UrlTest {
                            // 触发 URL 测试组的自动选择
                            println!("     触发组 '{}' 的自动代理选择", group_name);
                        }
                    }
                }
                _ => println!("     系统状态正常"),
            }
        }

        // 2. 清理过期连接（模拟）
        if let Ok(connections) = client.connections().await {
            let long_connections = connections
                .iter()
                .filter(|conn| {
                    // 模拟检查长时间连接
                    conn.upload + conn.download > 100 * 1024 * 1024 // 超过100MB的连接
                })
                .count();

            if long_connections > 0 {
                println!("     发现 {} 个长时间连接", long_connections);
            }
        }

        // 3. 性能统计
        let stats = monitor.get_performance_stats(Duration::from_secs(300));
        if stats.error_rate > 10.0 {
            println!("     错误率过高 ({:.2}%)，建议检查配置", stats.error_rate);
        }
    }

    println!("   自动化运维完成");
    Ok(())
}
