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
