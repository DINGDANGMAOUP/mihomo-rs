#![allow(dead_code)]

use std::path::{Path, PathBuf};
use tempfile::{tempdir, TempDir};
use tokio::fs;

pub fn setup_temp_home() -> TempDir {
    tempdir().expect("create temp home")
}

pub fn temp_home_path(temp: &TempDir) -> PathBuf {
    temp.path().to_path_buf()
}

pub fn default_test_config() -> String {
    r#"port: 7890
socks-port: 7891
allow-lan: false
mode: rule
log-level: info
external-controller: 127.0.0.1:9090
"#
    .to_string()
}

pub fn config_without_controller() -> String {
    r#"port: 7890
socks-port: 7891
allow-lan: false
mode: rule
log-level: info
"#
    .to_string()
}

pub fn mock_proxies_payload() -> &'static str {
    r#"{
  "proxies": {
    "GLOBAL": {
      "type": "Selector",
      "now": "HK-01",
      "all": ["HK-01", "JP-01"]
    },
    "HK-01": {
      "type": "Shadowsocks",
      "history": [{"time": "2024-01-01T00:00:00Z", "delay": 35}]
    },
    "JP-01": {
      "type": "Shadowsocks",
      "history": []
    }
  }
}"#
}

pub fn mock_connections_payload() -> &'static str {
    r#"{
  "downloadTotal": 4096,
  "uploadTotal": 2048,
  "connections": [
    {
      "id": "c1",
      "metadata": {
        "network": "tcp",
        "type": "HTTP",
        "sourceIP": "192.168.1.10",
        "destinationIP": "1.1.1.1",
        "sourcePort": "52345",
        "destinationPort": "443",
        "host": "example.com",
        "dnsMode": "normal",
        "processPath": "/usr/bin/curl",
        "specialProxy": ""
      },
      "upload": 100,
      "download": 200,
      "start": "2024-01-01T00:00:00Z",
      "chains": ["DIRECT"],
      "rule": "DIRECT",
      "rulePayload": ""
    },
    {
      "id": "c2",
      "metadata": {
        "network": "tcp",
        "type": "HTTPS",
        "sourceIP": "192.168.1.11",
        "destinationIP": "8.8.8.8",
        "sourcePort": "52346",
        "destinationPort": "443",
        "host": "rust-lang.org",
        "dnsMode": "normal",
        "processPath": "/Applications/Firefox",
        "specialProxy": ""
      },
      "upload": 300,
      "download": 400,
      "start": "2024-01-01T00:00:01Z",
      "chains": ["HK-01"],
      "rule": "MATCH",
      "rulePayload": ""
    }
  ]
}"#
}

pub async fn install_fake_version(home: &Path, version: &str) -> PathBuf {
    let binary_name = if cfg!(windows) {
        "mihomo.exe"
    } else {
        "mihomo"
    };
    let binary_path = home.join("versions").join(version).join(binary_name);
    if let Some(parent) = binary_path.parent() {
        fs::create_dir_all(parent)
            .await
            .expect("create version dir");
    }
    fs::write(&binary_path, b"fake-binary")
        .await
        .expect("write fake binary");
    binary_path
}
