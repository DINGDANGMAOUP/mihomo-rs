use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// Helper function to deserialize null as empty vec
fn deserialize_null_as_empty_vec<'de, D, T>(deserializer: D) -> Result<Vec<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    let opt = Option::<Vec<T>>::deserialize(deserializer)?;
    Ok(opt.unwrap_or_default())
}


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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Connection {
    pub id: String,
    #[serde(default)]
    pub metadata: ConnectionMetadata,
    #[serde(default)]
    pub upload: u64,
    #[serde(default)]
    pub download: u64,
    #[serde(default)]
    pub start: String,
    #[serde(default, deserialize_with = "deserialize_null_as_empty_vec")]
    pub chains: Vec<String>,
    #[serde(default)]
    pub rule: String,
    #[serde(rename = "rulePayload")]
    #[serde(default)]
    pub rule_payload: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ConnectionMetadata {
    #[serde(default)]
    pub network: String,
    #[serde(rename = "type")]
    #[serde(default)]
    pub connection_type: String,
    #[serde(rename = "sourceIP")]
    #[serde(default)]
    pub source_ip: String,
    #[serde(rename = "destinationIP")]
    #[serde(default)]
    pub destination_ip: String,
    #[serde(rename = "sourcePort")]
    #[serde(default)]
    pub source_port: String,
    #[serde(rename = "destinationPort")]
    #[serde(default)]
    pub destination_port: String,
    #[serde(default)]
    pub host: String,
    #[serde(rename = "dnsMode")]
    #[serde(default)]
    pub dns_mode: String,
    #[serde(rename = "processPath")]
    #[serde(default)]
    pub process_path: String,
    #[serde(rename = "specialProxy")]
    #[serde(default)]
    pub special_proxy: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ConnectionsResponse {
    #[serde(rename = "downloadTotal")]
    #[serde(default)]
    pub download_total: u64,
    #[serde(rename = "uploadTotal")]
    #[serde(default)]
    pub upload_total: u64,
    #[serde(default, deserialize_with = "deserialize_null_as_empty_vec")]
    pub connections: Vec<Connection>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionSnapshot {
    #[serde(rename = "downloadTotal")]
    #[serde(default)]
    pub download_total: u64,
    #[serde(rename = "uploadTotal")]
    #[serde(default)]
    pub upload_total: u64,
    #[serde(default, deserialize_with = "deserialize_null_as_empty_vec")]
    pub connections: Vec<Connection>,
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

    #[test]
    fn test_connections_response_with_empty_connections() {
        let json = r#"{
            "downloadTotal": 1024,
            "uploadTotal": 2048,
            "connections": []
        }"#;
        let response: ConnectionsResponse = serde_json::from_str(json).unwrap();

        assert_eq!(response.download_total, 1024);
        assert_eq!(response.upload_total, 2048);
        assert_eq!(response.connections.len(), 0);
    }

    #[test]
    fn test_connections_response_with_null_connections() {
        let json = r#"{
            "downloadTotal": 1024,
            "uploadTotal": 2048,
            "connections": null
        }"#;
        let response: ConnectionsResponse = serde_json::from_str(json).unwrap();

        assert_eq!(response.download_total, 1024);
        assert_eq!(response.upload_total, 2048);
        assert_eq!(response.connections.len(), 0);
    }

    #[test]
    fn test_connections_response_minimal() {
        let json = r#"{}"#;
        let response: ConnectionsResponse = serde_json::from_str(json).unwrap();

        assert_eq!(response.download_total, 0);
        assert_eq!(response.upload_total, 0);
        assert_eq!(response.connections.len(), 0);
    }

    #[test]
    fn test_connection_with_minimal_fields() {
        let json = r#"{"id":"test-id"}"#;
        let conn: Connection = serde_json::from_str(json).unwrap();

        assert_eq!(conn.id, "test-id");
        assert_eq!(conn.upload, 0);
        assert_eq!(conn.download, 0);
        assert_eq!(conn.start, "");
        assert!(conn.chains.is_empty());
        assert_eq!(conn.rule, "");
    }

    #[test]
    fn test_connection_snapshot_serialization() {
        let snapshot = ConnectionSnapshot {
            download_total: 1024,
            upload_total: 2048,
            connections: vec![],
        };

        let json = serde_json::to_string(&snapshot).unwrap();
        let deserialized: ConnectionSnapshot = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.download_total, 1024);
        assert_eq!(deserialized.upload_total, 2048);
        assert_eq!(deserialized.connections.len(), 0);
    }

    #[test]
    fn test_connection_full_serialization() {
        let json = r#"{
            "id": "test-connection-id",
            "metadata": {
                "network": "tcp",
                "type": "HTTP",
                "sourceIP": "192.168.1.100",
                "destinationIP": "1.1.1.1",
                "sourcePort": "54321",
                "destinationPort": "443",
                "host": "example.com",
                "dnsMode": "normal",
                "processPath": "/usr/bin/chrome",
                "specialProxy": ""
            },
            "upload": 1024,
            "download": 2048,
            "start": "2024-01-01T00:00:00Z",
            "chains": ["DIRECT"],
            "rule": "DOMAIN,example.com",
            "rulePayload": "example.com"
        }"#;

        let conn: Connection = serde_json::from_str(json).unwrap();

        assert_eq!(conn.id, "test-connection-id");
        assert_eq!(conn.metadata.network, "tcp");
        assert_eq!(conn.metadata.connection_type, "HTTP");
        assert_eq!(conn.metadata.source_ip, "192.168.1.100");
        assert_eq!(conn.metadata.destination_ip, "1.1.1.1");
        assert_eq!(conn.metadata.host, "example.com");
        assert_eq!(conn.upload, 1024);
        assert_eq!(conn.download, 2048);
        assert_eq!(conn.chains, vec!["DIRECT"]);
        assert_eq!(conn.rule, "DOMAIN,example.com");
    }

    #[test]
    fn test_connection_with_null_chains() {
        let json = r#"{
            "id": "test-id",
            "chains": null
        }"#;
        let conn: Connection = serde_json::from_str(json).unwrap();

        assert_eq!(conn.id, "test-id");
        assert!(conn.chains.is_empty());
    }

    #[test]
    fn test_connection_metadata_default() {
        let metadata = ConnectionMetadata::default();

        assert_eq!(metadata.network, "");
        assert_eq!(metadata.connection_type, "");
        assert_eq!(metadata.source_ip, "");
        assert_eq!(metadata.destination_ip, "");
        assert_eq!(metadata.source_port, "");
        assert_eq!(metadata.destination_port, "");
        assert_eq!(metadata.host, "");
        assert_eq!(metadata.dns_mode, "");
        assert_eq!(metadata.process_path, "");
        assert_eq!(metadata.special_proxy, "");
    }

    #[test]
    fn test_connections_response_with_data() {
        let json = r#"{
            "downloadTotal": 1048576,
            "uploadTotal": 524288,
            "connections": [
                {
                    "id": "conn-1",
                    "upload": 1024,
                    "download": 2048,
                    "start": "2024-01-01T00:00:00Z",
                    "chains": ["DIRECT"],
                    "rule": "DIRECT"
                }
            ]
        }"#;

        let response: ConnectionsResponse = serde_json::from_str(json).unwrap();

        assert_eq!(response.download_total, 1048576);
        assert_eq!(response.upload_total, 524288);
        assert_eq!(response.connections.len(), 1);
        assert_eq!(response.connections[0].id, "conn-1");
        assert_eq!(response.connections[0].upload, 1024);
        assert_eq!(response.connections[0].download, 2048);
    }

    #[test]
    fn test_connection_snapshot_with_null_connections() {
        let json = r#"{
            "downloadTotal": 100,
            "uploadTotal": 200,
            "connections": null
        }"#;

        let snapshot: ConnectionSnapshot = serde_json::from_str(json).unwrap();

        assert_eq!(snapshot.download_total, 100);
        assert_eq!(snapshot.upload_total, 200);
        assert_eq!(snapshot.connections.len(), 0);
    }
}
