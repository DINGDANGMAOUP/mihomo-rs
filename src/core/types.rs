use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Version {
    pub version: String,
    #[serde(default)]
    pub premium: bool,
    #[serde(default)]
    pub meta: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyNode {
    pub name: String,
    #[serde(rename = "type")]
    pub proxy_type: String,
    #[serde(default)]
    pub delay: Option<u32>,
    #[serde(default)]
    pub alive: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyGroup {
    pub name: String,
    #[serde(rename = "type")]
    pub group_type: String,
    pub now: String,
    pub all: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProxiesResponse {
    pub proxies: HashMap<String, ProxyInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyInfo {
    #[serde(rename = "type")]
    pub proxy_type: String,
    #[serde(default)]
    pub now: Option<String>,
    #[serde(default)]
    pub all: Option<Vec<String>>,
    #[serde(default)]
    pub history: Vec<DelayHistory>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DelayHistory {
    pub time: String,
    pub delay: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DelayTestRequest {
    pub timeout: u32,
    pub url: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DelayTestResponse {
    pub delay: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrafficData {
    pub up: u64,
    pub down: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryData {
    #[serde(rename = "inuse")]
    pub in_use: u64,
    #[serde(rename = "oslimit")]
    pub os_limit: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_serialization() {
        let version = Version {
            version: "v1.18.0".to_string(),
            premium: true,
            meta: false,
        };

        let json = serde_json::to_string(&version).unwrap();
        let deserialized: Version = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.version, "v1.18.0");
        assert!(deserialized.premium);
        assert!(!deserialized.meta);
    }

    #[test]
    fn test_version_default_fields() {
        let json = r#"{"version":"v1.18.0"}"#;
        let version: Version = serde_json::from_str(json).unwrap();

        assert_eq!(version.version, "v1.18.0");
        assert!(!version.premium);
        assert!(!version.meta);
    }

    #[test]
    fn test_proxy_node_serialization() {
        let node = ProxyNode {
            name: "test-proxy".to_string(),
            proxy_type: "ss".to_string(),
            delay: Some(100),
            alive: true,
        };

        let json = serde_json::to_string(&node).unwrap();
        let deserialized: ProxyNode = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.name, "test-proxy");
        assert_eq!(deserialized.proxy_type, "ss");
        assert_eq!(deserialized.delay, Some(100));
        assert!(deserialized.alive);
    }

    #[test]
    fn test_proxy_node_default_fields() {
        let json = r#"{"name":"test","type":"ss"}"#;
        let node: ProxyNode = serde_json::from_str(json).unwrap();

        assert_eq!(node.name, "test");
        assert_eq!(node.proxy_type, "ss");
        assert_eq!(node.delay, None);
        assert!(!node.alive);
    }

    #[test]
    fn test_proxy_group_serialization() {
        let group = ProxyGroup {
            name: "GLOBAL".to_string(),
            group_type: "Selector".to_string(),
            now: "proxy1".to_string(),
            all: vec!["proxy1".to_string(), "proxy2".to_string()],
        };

        let json = serde_json::to_string(&group).unwrap();
        let deserialized: ProxyGroup = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.name, "GLOBAL");
        assert_eq!(deserialized.group_type, "Selector");
        assert_eq!(deserialized.now, "proxy1");
        assert_eq!(deserialized.all.len(), 2);
    }

    #[test]
    fn test_traffic_data_serialization() {
        let traffic = TrafficData {
            up: 1024,
            down: 2048,
        };

        let json = serde_json::to_string(&traffic).unwrap();
        let deserialized: TrafficData = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.up, 1024);
        assert_eq!(deserialized.down, 2048);
    }

    #[test]
    fn test_memory_data_serialization() {
        let json = r#"{"inuse":1048576,"oslimit":4194304}"#;
        let memory: MemoryData = serde_json::from_str(json).unwrap();

        assert_eq!(memory.in_use, 1048576);
        assert_eq!(memory.os_limit, 4194304);
    }

    #[test]
    fn test_memory_data_field_rename() {
        let memory = MemoryData {
            in_use: 1024,
            os_limit: 2048,
        };

        let json = serde_json::to_string(&memory).unwrap();
        assert!(json.contains("\"inuse\":"));
        assert!(json.contains("\"oslimit\":"));
    }

    #[test]
    fn test_delay_history_serialization() {
        let history = DelayHistory {
            time: "2024-01-01T00:00:00Z".to_string(),
            delay: 100,
        };

        let json = serde_json::to_string(&history).unwrap();
        let deserialized: DelayHistory = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.time, "2024-01-01T00:00:00Z");
        assert_eq!(deserialized.delay, 100);
    }

    #[test]
    fn test_proxy_info_with_group_fields() {
        let json = r#"{
            "type": "Selector",
            "now": "proxy1",
            "all": ["proxy1", "proxy2"]
        }"#;

        let info: ProxyInfo = serde_json::from_str(json).unwrap();
        assert_eq!(info.proxy_type, "Selector");
        assert_eq!(info.now, Some("proxy1".to_string()));
        assert_eq!(
            info.all,
            Some(vec!["proxy1".to_string(), "proxy2".to_string()])
        );
    }

    #[test]
    fn test_proxy_info_without_optional_fields() {
        let json = r#"{"type":"ss"}"#;
        let info: ProxyInfo = serde_json::from_str(json).unwrap();

        assert_eq!(info.proxy_type, "ss");
        assert_eq!(info.now, None);
        assert_eq!(info.all, None);
        assert!(info.history.is_empty());
    }
}
