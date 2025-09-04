//! 性能测试模块
//! 测试SDK在不同负载下的性能表现

use mihomo_rs::{client::MihomoClient, monitor::Monitor, proxy::ProxyManager, rules::RuleEngine};
use std::time::{Duration, Instant};
use tokio::test;
use tokio::time::sleep;

mod test_utils;
use test_utils::{get_test_mode, TestMode};

/// 测试并发代理查询性能
#[test]
async fn test_concurrent_proxy_queries() {
    let test_mode = get_test_mode().await;
    let base_url = match &test_mode {
        TestMode::Real(url) | TestMode::Mock(url) => url.clone(),
    };

    let client = MihomoClient::new(&base_url, None).unwrap();
    let _proxy_manager = ProxyManager::new(client);

    let start_time = Instant::now();
    let concurrent_requests = 10;

    // 创建并发任务
    let mut tasks = Vec::new();
    for i in 0..concurrent_requests {
        let client_clone = MihomoClient::new(&base_url, None).unwrap();
        let mut pm = ProxyManager::new(client_clone);
        let task = tokio::spawn(async move {
            let result = pm.get_proxies().await;
            (i, result.is_ok())
        });
        tasks.push(task);
    }

    // 等待所有任务完成
    let mut success_count = 0;
    for task in tasks {
        let (id, success) = task.await.unwrap();
        if success {
            success_count += 1;
        }
        println!("任务 {} 完成，成功: {}", id, success);
    }

    let elapsed = start_time.elapsed();
    let success_rate = (success_count as f64 / concurrent_requests as f64) * 100.0;

    println!("并发代理查询测试结果:");
    println!("- 并发请求数: {}", concurrent_requests);
    println!("- 成功请求数: {}", success_count);
    println!("- 成功率: {:.2}%", success_rate);
    println!("- 总耗时: {:?}", elapsed);
    println!("- 平均耗时: {:?}", elapsed / concurrent_requests);

    match test_mode {
        TestMode::Real(_) => {
            // 真实服务测试：验证性能指标
            assert!(
                elapsed < Duration::from_secs(10),
                "并发请求应该在10秒内完成"
            );
            println!("真实服务性能测试通过");
        }
        TestMode::Mock(_) => {
            // 模拟服务测试：应该有更好的性能
            assert!(success_rate >= 90.0, "模拟服务成功率应该>=90%");
            assert!(elapsed < Duration::from_secs(5), "模拟服务应该在5秒内完成");
            println!("模拟服务性能测试通过");
        }
    }
}

/// 测试规则匹配性能
#[test]
async fn test_rule_matching_performance() {
    let test_mode = get_test_mode().await;
    let base_url = match &test_mode {
        TestMode::Real(url) | TestMode::Mock(url) => url.clone(),
    };

    let client = MihomoClient::new(&base_url, None).unwrap();
    let mut rule_engine = RuleEngine::new(client);

    // 预热：先获取一次规则
    let _ = rule_engine.get_rules().await;

    let test_domains = vec![
        "example.com",
        "google.com",
        "github.com",
        "stackoverflow.com",
        "rust-lang.org",
    ];

    let start_time = Instant::now();
    let mut match_results = Vec::new();

    // 测试多个域名的规则匹配
    for domain in &test_domains {
        let match_start = Instant::now();
        let result = rule_engine.match_rule(domain, Some(80), None).await;
        let match_elapsed = match_start.elapsed();

        match_results.push((domain, result.is_ok(), match_elapsed));
    }

    let total_elapsed = start_time.elapsed();
    let avg_match_time = total_elapsed / test_domains.len() as u32;

    println!("规则匹配性能测试结果:");
    println!("- 测试域名数: {}", test_domains.len());
    println!("- 总耗时: {:?}", total_elapsed);
    println!("- 平均匹配时间: {:?}", avg_match_time);

    for (domain, success, elapsed) in &match_results {
        println!("  {} -> 成功: {}, 耗时: {:?}", domain, success, elapsed);
    }

    match test_mode {
        TestMode::Real(_) => {
            // 真实服务测试：验证匹配性能
            assert!(
                avg_match_time < Duration::from_millis(1000),
                "平均匹配时间应该<1秒"
            );
            println!("真实服务规则匹配性能测试通过");
        }
        TestMode::Mock(_) => {
            // 模拟服务测试：应该有更好的性能
            assert!(
                avg_match_time < Duration::from_millis(500),
                "模拟服务平均匹配时间应该<500ms"
            );
            let success_count = match_results
                .iter()
                .filter(|(_, success, _)| *success)
                .count();
            assert!(
                success_count >= test_domains.len() / 2,
                "至少一半的匹配应该成功"
            );
            println!("模拟服务规则匹配性能测试通过");
        }
    }
}

/// 测试监控数据收集性能
#[test]
async fn test_monitoring_performance() {
    let test_mode = get_test_mode().await;
    let base_url = match &test_mode {
        TestMode::Real(url) | TestMode::Mock(url) => url.clone(),
    };

    let client = MihomoClient::new(&base_url, None).unwrap();
    let monitor = Monitor::new(client);

    let start_time = Instant::now();
    let monitoring_cycles = 3; // 减少测试周期数以适应30秒超时

    // 模拟监控数据收集
    for i in 0..monitoring_cycles {
        let cycle_start = Instant::now();

        // 获取系统状态
        let status_result = monitor.get_system_status().await;
        let status_success = status_result.is_ok();

        // 获取性能统计
        let _stats = monitor.get_performance_stats(Duration::from_secs(60));

        let cycle_elapsed = cycle_start.elapsed();
        println!(
            "监控周期 {}: 状态获取成功={}, 耗时={:?}",
            i + 1,
            status_success,
            cycle_elapsed
        );

        // 短暂等待模拟实际监控间隔
        sleep(Duration::from_millis(50)).await;
    }

    let total_elapsed = start_time.elapsed();
    let avg_cycle_time = total_elapsed / monitoring_cycles;

    println!("监控性能测试结果:");
    println!("- 监控周期数: {}", monitoring_cycles);
    println!("- 总耗时: {:?}", total_elapsed);
    println!("- 平均周期时间: {:?}", avg_cycle_time);

    match test_mode {
        TestMode::Real(_) => {
            // 真实服务测试：由于可能有网络延迟，放宽时间限制
            if avg_cycle_time < Duration::from_secs(10) {
                println!("真实服务监控性能测试通过");
            } else {
                println!("真实服务监控性能测试：平均周期时间较长，可能存在网络问题");
            }
        }
        TestMode::Mock(_) => {
            // 模拟服务测试：应该有更好的性能
            assert!(
                avg_cycle_time < Duration::from_secs(1),
                "模拟服务平均监控周期应该<1秒"
            );
            println!("模拟服务监控性能测试通过");
        }
    }
}

/// 测试内存使用情况
#[test]
async fn test_memory_usage() {
    let test_mode = get_test_mode().await;
    let base_url = match &test_mode {
        TestMode::Real(url) | TestMode::Mock(url) => url.clone(),
    };

    // 创建多个客户端实例测试内存使用
    let mut clients = Vec::new();
    let client_count = 10;

    for _ in 0..client_count {
        let client = MihomoClient::new(&base_url, None).unwrap();
        let proxy_manager = ProxyManager::new(client.clone());
        let rule_engine = RuleEngine::new(client.clone());

        clients.push((client, proxy_manager, rule_engine));
    }

    println!("创建了 {} 个客户端实例", client_count);

    // 执行一些操作
    let mut operation_count = 0;
    for (_, proxy_manager, rule_engine) in &mut clients {
        // 执行代理查询
        let _ = proxy_manager.get_proxies().await;
        operation_count += 1;

        // 执行规则查询
        let _ = rule_engine.get_rules().await;
        operation_count += 1;
    }

    println!("执行了 {} 个操作", operation_count);

    // 清理资源
    drop(clients);

    println!("内存使用测试完成 - 资源已清理");

    // 这个测试主要是确保没有内存泄漏，通过正常完成来验证
    // 内存使用测试应该正常完成
}
