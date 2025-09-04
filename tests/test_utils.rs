//! 测试工具模块
//! 提供模拟服务器和服务检测功能

use std::time::Duration;
use tokio::time::timeout;
use wiremock::{MockServer, Mock, ResponseTemplate};
use wiremock::matchers::{method, path};

/// 检测mihomo服务是否可用
pub async fn is_mihomo_available(base_url: &str) -> bool {
    let client = reqwest::Client::new();
    let url = format!("{}/version", base_url);
    
    match timeout(Duration::from_secs(2), client.get(&url).send()).await {
        Ok(Ok(response)) => response.status().is_success(),
        _ => false,
    }
}

/// 创建模拟mihomo服务器
pub async fn create_mock_server() -> MockServer {
    let mock_server = MockServer::start().await;
    
    // 模拟 /version 端点
    Mock::given(method("GET"))
        .and(path("/version"))
        .respond_with(ResponseTemplate::new(200)
            .set_body_json(serde_json::json!({
                "version": "v1.19.13",
                "meta": true
            })))
        .mount(&mock_server)
        .await;
    
    // 模拟 /proxies 端点
    Mock::given(method("GET"))
        .and(path("/proxies"))
        .respond_with(ResponseTemplate::new(200)
            .set_body_json(serde_json::json!({
                "proxies": {
                    "GLOBAL": {
                        "name": "GLOBAL",
                        "type": "Selector",
                        "now": "DIRECT",
                        "all": ["DIRECT", "REJECT"],
                        "history": [],
                        "alive": true,
                        "extra": {},
                        "hidden": false,
                        "icon": "",
                        "dialer_proxy": "",
                        "interface": "",
                        "mptcp": false,
                        "routing_mark": 0,
                        "smux": false,
                        "test_url": "",
                        "tfo": false,
                        "udp": false,
                        "uot": false,
                        "xudp": false
                    },
                    "DIRECT": {
                        "name": "DIRECT",
                        "type": "Direct",
                        "udp": true,
                        "delay": null,
                        "history": [],
                        "alive": true,
                        "extra": {},
                        "server": null,
                        "port": null,
                        "dialer_proxy": "",
                        "interface": "",
                        "mptcp": false,
                        "routing_mark": 0,
                        "smux": false,
                        "tfo": false,
                        "uot": false,
                        "xudp": false,
                        "id": ""
                    }
                }
            })))
        .mount(&mock_server)
        .await;
    
    // 模拟 /rules 端点
    Mock::given(method("GET"))
        .and(path("/rules"))
        .respond_with(ResponseTemplate::new(200)
            .set_body_json(serde_json::json!({
                "rules": [
                    {
                        "type": "DOMAIN",
                        "payload": "example.com",
                        "proxy": "DIRECT",
                        "size": 0
                    },
                    {
                        "type": "IP-CIDR",
                        "payload": "192.168.1.0/24",
                        "proxy": "DIRECT",
                        "size": 0
                    },
                    {
                        "type": "Match",
                        "payload": "",
                        "proxy": "GLOBAL",
                        "size": 0
                    }
                ]
            })))
        .mount(&mock_server)
        .await;
    
    // 模拟 /traffic 端点
    Mock::given(method("GET"))
        .and(path("/traffic"))
        .respond_with(ResponseTemplate::new(200)
            .set_body_json(serde_json::json!({
                "up": 1024,
                "down": 2048
            })))
        .mount(&mock_server)
        .await;
    
    // 模拟 /memory 端点
    Mock::given(method("GET"))
        .and(path("/memory"))
        .respond_with(ResponseTemplate::new(200)
            .set_body_json(serde_json::json!({
                "inuse": 50331648,
                "oslimit": 134217728
            })))
        .mount(&mock_server)
        .await;
    
    // 模拟 /connections 端点
    Mock::given(method("GET"))
        .and(path("/connections"))
        .respond_with(ResponseTemplate::new(200)
            .set_body_json(serde_json::json!({
                "downloadTotal": 1024,
                "uploadTotal": 512,
                "connections": []
            })))
        .mount(&mock_server)
        .await;
    
    mock_server
}

/// 测试模式枚举
#[derive(Debug, Clone)]
pub enum TestMode {
    /// 使用真实的mihomo服务
    Real(String),
    /// 使用模拟服务器
    Mock(String),
}

/// 获取测试模式
pub async fn get_test_mode() -> TestMode {
    let real_url = "http://127.0.0.1:9090";
    
    if is_mihomo_available(real_url).await {
        println!("检测到mihomo服务运行中，使用真实服务测试");
        TestMode::Real(real_url.to_string())
    } else {
        println!("未检测到mihomo服务，使用模拟服务器测试");
        let mock_server = create_mock_server().await;
        TestMode::Mock(mock_server.uri())
    }
}