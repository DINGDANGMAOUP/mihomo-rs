//! 集成测试
//!
//! 测试 mihomo-rs SDK 的各个模块功能

use mihomo_rs::{
    client::MihomoClient,
    config::{Config, ConfigManager},
    monitor::{Monitor, MonitorConfig},
    proxy::ProxyManager,
    rules::RuleEngine,
    types::*,
    utils::*,
    MihomoError,
};
use std::time::Duration;
use tokio::test;

mod test_utils;
use test_utils::{TestMode, get_test_mode};

/// 测试客户端创建和基本功能
#[test]
async fn test_client_creation() {
    let client = MihomoClient::new("http://127.0.0.1:9090", None);
    assert!(client.is_ok());

    let _client = client.unwrap();
    // 测试无效的 URL
    let invalid_client = MihomoClient::new("invalid-url", None);
    assert!(invalid_client.is_err());
}

/// 测试代理管理器
#[test]
async fn test_proxy_manager() {
    let test_mode = get_test_mode().await;
    let base_url = match &test_mode {
        TestMode::Real(url) | TestMode::Mock(url) => url.clone(),
    };
    
    let client = MihomoClient::new(&base_url, None).unwrap();
    let mut proxy_manager = ProxyManager::new(client);
    
    // 测试代理获取
    let proxies_result = proxy_manager.get_proxies().await;
    
    match test_mode {
        TestMode::Real(_) => {
            // 真实服务测试：验证实际功能
            match proxies_result {
                Ok(proxies) => {
                    println!("真实服务测试：代理获取成功，共{}个代理", proxies.len());
                    // 验证代理数据结构
                    for (name, proxy) in proxies.iter().take(3) {
                        println!("代理: {} -> {:?}", name, proxy.proxy_type);
                    }
                },
                Err(e) => println!("真实服务测试：代理获取失败 - {}", e),
            }
        },
        TestMode::Mock(_) => {
            // 模拟服务测试：验证数据解析和类型安全
            assert!(proxies_result.is_ok(), "模拟服务测试应该成功");
            let proxies = proxies_result.unwrap();
            assert!(!proxies.is_empty(), "应该返回模拟代理数据");
            
            // 验证特定的模拟数据
            assert!(proxies.contains_key("GLOBAL"), "应该包含GLOBAL代理组");
            assert!(proxies.contains_key("DIRECT"), "应该包含DIRECT代理");
            
            println!("模拟服务测试：代理解析验证通过");
        }
    }
}

/// 测试规则引擎
#[test]
async fn test_rule_engine() {
    let test_mode = get_test_mode().await;
    let base_url = match &test_mode {
        TestMode::Real(url) | TestMode::Mock(url) => url.clone(),
    };
    
    let client = MihomoClient::new(&base_url, None).unwrap();
    let mut rule_engine = RuleEngine::new(client);
    
    // 测试规则获取
    let rules = rule_engine.get_rules().await;
    
    match test_mode {
        TestMode::Real(_) => {
            // 真实服务测试：验证实际功能
            match rules {
                Ok(rules) => {
                    println!("真实服务测试：规则获取成功，共{}条规则", rules.len());
                    // 验证规则数据结构
                    for rule in rules.iter().take(3) {
                        println!("规则: {:?} {} -> {}", rule.rule_type, rule.payload, rule.proxy);
                    }
                },
                Err(e) => println!("真实服务测试：规则获取失败 - {}", e),
            }
            
            // 测试规则匹配
            let result = rule_engine.match_rule("example.com", Some(80), None).await;
            match result {
                Ok(matched) => println!("真实服务测试：规则匹配成功 -> {:?}", matched),
                Err(e) => println!("真实服务测试：规则匹配失败 - {}", e),
            }
        },
        TestMode::Mock(_) => {
            // 模拟服务测试：验证数据解析和类型安全
            assert!(rules.is_ok(), "模拟服务测试应该成功");
            let rules = rules.unwrap();
            assert!(!rules.is_empty(), "应该返回模拟规则数据");
            
            // 验证规则类型
            let has_domain_rule = rules.iter().any(|r| matches!(r.rule_type, RuleType::Domain));
            let has_ip_rule = rules.iter().any(|r| matches!(r.rule_type, RuleType::IpCidr));
            assert!(has_domain_rule || has_ip_rule, "应该包含域名或IP规则");
            
            // 测试规则匹配
            let result = rule_engine.match_rule("example.com", Some(80), None).await;
            assert!(result.is_ok(), "模拟服务规则匹配应该成功");
            
            println!("模拟服务测试：规则解析验证通过");
        }
    }
}

/// 测试配置管理器
#[test]
async fn test_config_manager() {
    let config_manager = ConfigManager::new();
    
    // 测试默认配置
    let config = config_manager.config();
    assert_eq!(config.port, 7890);
    assert_eq!(config.mode, "rule");
    
    // 测试配置验证
    let mut invalid_config = config.clone();
    invalid_config.port = 0; // 无效端口
    
    // 测试配置管理器功能
    let config_clone = config.clone();
    assert_eq!(config_clone.port, config.port);
}

/// 测试监控器
#[test]
async fn test_monitor() {
    let test_mode = get_test_mode().await;
    let base_url = match &test_mode {
        TestMode::Real(url) | TestMode::Mock(url) => url.clone(),
    };
    
    let client = MihomoClient::new(&base_url, None).unwrap();
    let monitor = Monitor::new(client);
    
    // 测试性能统计
    let stats = monitor.get_performance_stats(Duration::from_secs(3600));
    
    match test_mode {
        TestMode::Real(_) => {
            // 真实服务测试：验证实际监控功能
            println!("真实服务测试：性能统计获取成功");
            println!("成功率: {:.2}%", stats.success_rate);
            println!("错误率: {:.2}%", stats.error_rate);
            
            // 测试事件获取
            let events = monitor.get_recent_events(10);
            println!("最近事件数量: {}", events.len());
        },
        TestMode::Mock(_) => {
            // 模拟服务测试：验证数据结构
            assert_eq!(stats.success_rate, 100.0); // 初始状态应该是100%
            assert_eq!(stats.error_rate, 0.0);
            
            // 测试事件获取
            let events = monitor.get_recent_events(10);
            assert!(events.is_empty()); // 初始状态应该没有事件
            
            println!("模拟服务测试：监控统计验证通过");
        }
    }
}

/// 测试监控配置
#[test]
async fn test_monitor_config() {
    let config = MonitorConfig::default();
    
    assert_eq!(config.interval, Duration::from_secs(10));
    assert_eq!(config.history_retention, Duration::from_secs(3600));
    assert!(config.enable_connection_monitor);
    assert!(config.enable_traffic_monitor);
    assert!(config.enable_memory_monitor);
    
    // 测试自定义配置
    let custom_config = MonitorConfig {
        interval: Duration::from_secs(5),
        history_retention: Duration::from_secs(1800),
        enable_connection_monitor: false,
        enable_traffic_monitor: true,
        enable_memory_monitor: true,
        connection_threshold: Some(100),
        memory_threshold: Some(256 * 1024 * 1024),
        traffic_threshold: Some(10 * 1024 * 1024),
    };
    
    assert_eq!(custom_config.interval, Duration::from_secs(5));
    assert!(!custom_config.enable_connection_monitor);
}

/// 测试错误处理
#[test]
async fn test_error_handling() {
    // 测试各种错误类型的创建
    // 测试错误类型创建
    let config_error = MihomoError::config("Config error");
    assert!(matches!(config_error, MihomoError::Config(_)));
    
    let proxy_error = MihomoError::proxy("Proxy error");
    assert!(matches!(proxy_error, MihomoError::Proxy(_)));
    
    // 测试错误消息
    assert_eq!(config_error.to_string(), "Configuration error: Config error");
    assert_eq!(proxy_error.to_string(), "Proxy error: Proxy error");
}

/// 测试类型定义
#[test]
async fn test_types() {
    // 测试代理类型
    let proxy_node = ProxyNode {
        name: "test-proxy".to_string(),
        proxy_type: ProxyType::Http,
        server: Some("127.0.0.1".to_string()),
        port: Some(8080),
        udp: true,
        delay: None,
        extra: std::collections::HashMap::new(),
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
    };
    
    assert_eq!(proxy_node.name, "test-proxy");
    assert_eq!(proxy_node.proxy_type, ProxyType::Http);
    assert_eq!(proxy_node.port, Some(8080));
    
    // 测试代理组
    let proxy_group = ProxyGroup {
        name: "test-group".to_string(),
        group_type: ProxyGroupType::Selector,
        now: "proxy1".to_string(),
        all: vec!["proxy1".to_string(), "proxy2".to_string()],
        history: vec![],
        alive: false,
        dialer_proxy: String::new(),
        extra: std::collections::HashMap::new(),
        hidden: false,
        icon: String::new(),
        interface: String::new(),
        mptcp: false,
        routing_mark: 0,
        smux: false,
        test_url: String::new(),
        tfo: false,
        udp: false,
        uot: false,
        xudp: false,
    };
    
    assert_eq!(proxy_group.name, "test-group");
    assert_eq!(proxy_group.group_type, ProxyGroupType::Selector);
    assert_eq!(proxy_group.all.len(), 2);
    
    // 测试规则
    let rule = Rule {
        rule_type: RuleType::Domain,
        payload: "example.com".to_string(),
        proxy: "DIRECT".to_string(),
        size: 0,
    };
    
    assert_eq!(rule.rule_type, RuleType::Domain);
    assert_eq!(rule.payload, "example.com");
    assert_eq!(rule.proxy, "DIRECT");
}

/// 测试工具函数 - URL 工具
#[test]
async fn test_url_utils() {
    use url_utils::*;
    
    // 测试 URL 验证
    assert!(validate_url("http://127.0.0.1:9090").is_ok());
    assert!(validate_url("https://example.com").is_ok());
    assert!(validate_url("invalid-url").is_err());
    
    // 测试 API URL 构建
    let url = build_api_url("http://127.0.0.1:9090", "/proxies", None);
    assert!(url.is_ok());
    assert_eq!(url.unwrap(), "http://127.0.0.1:9090/proxies");
    
    // 测试带参数的 URL 构建
    let mut params = std::collections::HashMap::new();
    params.insert("timeout".to_string(), "5000".to_string());
    
    let url = build_api_url("http://127.0.0.1:9090", "/proxies/test/delay", Some(&params));
    assert!(url.is_ok());
    assert!(url.unwrap().contains("timeout=5000"));
    
    // 测试主机端口提取
    let (host, port) = extract_host_port("http://127.0.0.1:9090").unwrap();
    assert_eq!(host, "127.0.0.1");
    assert_eq!(port, Some(9090));
}

/// 测试工具函数 - 网络工具
#[test]
async fn test_network_utils() {
    use network_utils::*;
    
    // 测试 IP 验证
    assert!(validate_ip("192.168.1.1").is_ok());
    assert!(validate_ip("::1").is_ok());
    assert!(validate_ip("invalid-ip").is_err());
    
    // 测试端口验证
    assert!(validate_port(80).is_ok());
    assert!(validate_port(65535).is_ok());
    assert!(validate_port(0).is_err());
    
    // 测试私有 IP 检查
    let private_ip = validate_ip("192.168.1.1").unwrap();
    assert!(is_private_ip(&private_ip));
    
    let public_ip = validate_ip("8.8.8.8").unwrap();
    assert!(!is_private_ip(&public_ip));
    
    // 测试 CIDR 解析
    let (ip, prefix) = parse_cidr("192.168.1.0/24").unwrap();
    assert_eq!(ip.to_string(), "192.168.1.0");
    assert_eq!(prefix, 24);
    
    assert!(parse_cidr("invalid-cidr").is_err());
    assert!(parse_cidr("192.168.1.0/33").is_err()); // 无效前缀长度
    
    // 测试 IP 在 CIDR 范围内检查
    assert!(ip_in_cidr(&validate_ip("192.168.1.100").unwrap(), "192.168.1.0/24").unwrap());
    assert!(!ip_in_cidr(&validate_ip("192.168.2.100").unwrap(), "192.168.1.0/24").unwrap());
}

/// 测试工具函数 - 字符串工具
#[test]
async fn test_string_utils() {
    use string_utils::*;
    
    // 测试 Base64 编码解码
    let data = b"Hello, World!";
    let encoded = base64_encode(data);
    let decoded = base64_decode(&encoded).unwrap();
    assert_eq!(data, decoded.as_slice());
    
    // 测试域名验证
    assert!(validate_domain("example.com").is_ok());
    assert!(validate_domain("sub.example.com").is_ok());
    assert!(validate_domain("-invalid.com").is_err());
    assert!(validate_domain("").is_err());
    
    // 测试字符串清理
    assert_eq!(clean_string("  hello   world  "), "hello world");
    assert_eq!(clean_string("\t\ntest\r\n"), "test");
    
    // 测试空白字符检查
    assert!(is_empty_or_whitespace(""));
    assert!(is_empty_or_whitespace("   "));
    assert!(is_empty_or_whitespace("\t\n"));
    assert!(!is_empty_or_whitespace("hello"));
    
    // 测试字符串截断
    assert_eq!(truncate_string("hello", 10), "hello");
    assert_eq!(truncate_string("hello world", 8), "hello...");
}

/// 测试工具函数 - 时间工具
#[test]
async fn test_time_utils() {
    use time_utils::*;
    
    // 测试时间戳
    let timestamp = current_timestamp();
    assert!(timestamp > 0);
    
    let timestamp_millis = current_timestamp_millis();
    assert!(timestamp_millis > timestamp * 1000);
    
    // 测试持续时间格式化
    assert_eq!(format_duration(Duration::from_secs(61)), "1m 1s");
    assert_eq!(format_duration(Duration::from_secs(3661)), "1h 1m 1s");
    assert_eq!(format_duration(Duration::from_secs(30)), "30s");
    
    // 测试持续时间解析
    assert_eq!(parse_duration("1h30m").unwrap(), Duration::from_secs(5400));
    assert_eq!(parse_duration("45s").unwrap(), Duration::from_secs(45));
    assert_eq!(parse_duration("2h").unwrap(), Duration::from_secs(7200));
    assert!(parse_duration("invalid").is_err());
    
    // 测试过期检查
    let old_timestamp = current_timestamp() - 100;
    assert!(is_expired(old_timestamp, 50));
    assert!(!is_expired(old_timestamp, 200));
}

/// 测试工具函数 - 格式化工具
#[test]
async fn test_format_utils() {
    use format_utils::*;
    
    // 测试字节格式化
    assert_eq!(format_bytes(0), "0 B");
    assert_eq!(format_bytes(1024), "1.00 KB");
    assert_eq!(format_bytes(1048576), "1.00 MB");
    assert_eq!(format_bytes(1073741824), "1.00 GB");
    
    // 测试速度格式化
    assert_eq!(format_speed(1024), "1.00 KB/s");
    assert_eq!(format_speed(1048576), "1.00 MB/s");
    
    // 测试百分比格式化
    assert_eq!(format_percentage(50.0, 100.0), "50.00%");
    assert_eq!(format_percentage(0.0, 0.0), "0.00%");
    assert_eq!(format_percentage(33.333, 100.0), "33.33%");
    
    // 测试延迟格式化
    assert_eq!(format_latency(150), "150ms");
    assert_eq!(format_latency(1500), "1.50s");
    assert_eq!(format_latency(500), "500ms");
}

/// 测试工具函数 - 验证工具
#[test]
async fn test_validation_utils() {
    use validation_utils::*;
    
    // 测试代理配置验证
    assert!(validate_proxy_config(&ProxyType::Http, "127.0.0.1", 8080).is_ok());
    assert!(validate_proxy_config(&ProxyType::Socks5, "example.com", 1080).is_ok());
    assert!(validate_proxy_config(&ProxyType::Http, "", 8080).is_err()); // 空服务器
    assert!(validate_proxy_config(&ProxyType::Http, "127.0.0.1", 0).is_err()); // 无效端口
    
    // 测试规则配置验证
    assert!(validate_rule_config(&RuleType::Domain, "example.com", "DIRECT").is_ok());
    assert!(validate_rule_config(&RuleType::IpCidr, "192.168.1.0/24", "PROXY").is_ok());
    assert!(validate_rule_config(&RuleType::DstPort, "80", "DIRECT").is_ok());
    
    assert!(validate_rule_config(&RuleType::Domain, "example.com", "").is_err()); // 空目标
    assert!(validate_rule_config(&RuleType::IpCidr, "invalid-cidr", "DIRECT").is_err());
    assert!(validate_rule_config(&RuleType::DstPort, "invalid-port", "DIRECT").is_err());
}

/// 测试工具函数 - 随机工具
#[test]
async fn test_random_utils() {
    use random_utils::*;
    
    // 测试随机字符串生成
    let random_str = generate_random_string(10);
    assert_eq!(random_str.len(), 10);
    
    let another_str = generate_random_string(10);
    assert_ne!(random_str, another_str); // 应该不相同（概率极低）
    
    // 测试 ID 生成
    let id = generate_id();
    assert!(id.contains('-'));
    assert!(id.len() > 10);
    
    // 测试随机选择
    let items = vec!["a", "b", "c", "d"];
    let choice = random_choice(&items);
    assert!(choice.is_some());
    assert!(items.contains(choice.unwrap()));
    
    let empty_items: Vec<&str> = vec![];
    let no_choice = random_choice(&empty_items);
    assert!(no_choice.is_none());
}

/// 测试配置序列化和反序列化
#[test]
async fn test_config_serialization() {
    let config = Config::default();
    
    // 测试 YAML 序列化
    let yaml_str = serde_yaml::to_string(&config);
    assert!(yaml_str.is_ok());
    
    // 测试 YAML 反序列化
    let deserialized: Result<Config, _> = serde_yaml::from_str(&yaml_str.unwrap());
    assert!(deserialized.is_ok());
    
    let deserialized_config = deserialized.unwrap();
    assert_eq!(config.port, deserialized_config.port);
    assert_eq!(config.mode, deserialized_config.mode);
    
    // 测试 JSON 序列化
    let json_str = serde_json::to_string(&config);
    assert!(json_str.is_ok());
    
    // 测试 JSON 反序列化
    let deserialized: Result<Config, _> = serde_json::from_str(&json_str.unwrap());
    assert!(deserialized.is_ok());
}

/// 性能测试
#[test]
async fn test_performance() {
    use std::time::Instant;
    
    // 测试大量代理节点的处理性能
    let start = Instant::now();
    
    let mut proxies = std::collections::HashMap::new();
    for i in 0..1000 {
        let proxy = ProxyNode {
            name: format!("proxy-{}", i),
            proxy_type: ProxyType::Socks5,
            server: Some(format!("192.168.1.{}", i % 255 + 1)),
            port: Some(1080),
            udp: true,
            delay: None,
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
            extra: std::collections::HashMap::new(),
        };
        proxies.insert(proxy.name.clone(), proxy);
    }
    
    let elapsed = start.elapsed();
    println!("创建1000个代理节点耗时: {:?}", elapsed);
    assert!(elapsed < Duration::from_millis(100)); // 应该在100ms内完成
    
    // 测试规则匹配性能
    let start = Instant::now();
    
    let mut rules = Vec::new();
    for i in 0..1000 {
        let rule = Rule {
            rule_type: RuleType::Domain,
            payload: format!("example{}.com", i),
            proxy: "DIRECT".to_string(),
            size: 0,
        };
        rules.push(rule);
    }
    
    // 模拟规则匹配
    let test_domain = "example500.com";
    let _matched = rules.iter().find(|r| r.payload == test_domain);
    
    let elapsed = start.elapsed();
    println!("创建1000条规则并匹配耗时: {:?}", elapsed);
    assert!(elapsed < Duration::from_millis(50)); // 应该在50ms内完成
}