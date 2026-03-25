use crate::core::{MihomoClient, ProxyGroup, ProxyInfo, ProxyNode, Result};
use std::collections::HashMap;

fn is_proxy_group_type(proxy_type: &str) -> bool {
    matches!(
        proxy_type,
        "Selector" | "URLTest" | "Fallback" | "LoadBalance" | "Relay"
    )
}

fn to_proxy_node(name: String, info: ProxyInfo) -> Option<ProxyNode> {
    if is_proxy_group_type(&info.proxy_type) {
        return None;
    }

    let delay = info.history.first().map(|h| h.delay);
    Some(ProxyNode {
        name,
        proxy_type: info.proxy_type,
        delay,
        alive: delay.is_some(),
    })
}

fn to_proxy_group(name: String, info: ProxyInfo) -> Option<ProxyGroup> {
    if !is_proxy_group_type(&info.proxy_type) {
        return None;
    }

    Some(ProxyGroup {
        name,
        group_type: info.proxy_type,
        now: info.now.unwrap_or_default(),
        all: info.all.unwrap_or_default(),
    })
}

pub struct ProxyManager {
    client: MihomoClient,
}

impl ProxyManager {
    pub fn new(client: MihomoClient) -> Self {
        Self { client }
    }

    pub async fn list_proxies(&self) -> Result<Vec<ProxyNode>> {
        let proxies = self.client.get_proxies().await?;
        let mut nodes = vec![];

        for (name, info) in proxies {
            if let Some(node) = to_proxy_node(name, info) {
                nodes.push(node);
            }
        }

        log::debug!("Filtered {} proxy nodes from all proxies", nodes.len());
        nodes.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(nodes)
    }

    pub async fn list_groups(&self) -> Result<Vec<ProxyGroup>> {
        let proxies = self.client.get_proxies().await?;
        let mut groups = vec![];

        for (name, info) in proxies {
            if let Some(group) = to_proxy_group(name, info) {
                groups.push(group);
            }
        }

        groups.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(groups)
    }

    pub async fn switch(&self, group: &str, proxy: &str) -> Result<()> {
        self.client.switch_proxy(group, proxy).await
    }

    pub async fn get_current(&self, group: &str) -> Result<String> {
        let info = self.client.get_proxy(group).await?;
        Ok(info.now.unwrap_or_default())
    }

    pub async fn get_all_proxies(&self) -> Result<HashMap<String, ProxyInfo>> {
        self.client.get_proxies().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::DelayHistory;

    fn make_proxy_info(proxy_type: &str, with_delay: bool) -> ProxyInfo {
        ProxyInfo {
            proxy_type: proxy_type.to_string(),
            now: Some("auto".to_string()),
            all: Some(vec!["a".to_string(), "b".to_string()]),
            history: if with_delay {
                vec![DelayHistory {
                    time: "2026-01-01T00:00:00Z".to_string(),
                    delay: 123,
                }]
            } else {
                vec![]
            },
        }
    }

    #[test]
    fn group_type_detection_is_consistent() {
        assert!(is_proxy_group_type("Selector"));
        assert!(is_proxy_group_type("URLTest"));
        assert!(is_proxy_group_type("Fallback"));
        assert!(is_proxy_group_type("LoadBalance"));
        assert!(is_proxy_group_type("Relay"));

        assert!(!is_proxy_group_type("Direct"));
        assert!(!is_proxy_group_type("Reject"));
        assert!(!is_proxy_group_type("ss"));
    }

    #[test]
    fn to_proxy_node_filters_group_entries() {
        let group = make_proxy_info("Selector", true);
        let node = make_proxy_info("Direct", true);

        assert!(to_proxy_node("g".to_string(), group).is_none());

        let parsed = to_proxy_node("n".to_string(), node).expect("node should be kept");
        assert_eq!(parsed.name, "n");
        assert_eq!(parsed.proxy_type, "Direct");
        assert_eq!(parsed.delay, Some(123));
        assert!(parsed.alive);
    }

    #[test]
    fn to_proxy_group_only_accepts_group_entries() {
        let group = make_proxy_info("URLTest", false);
        let node = make_proxy_info("ss", false);

        let parsed = to_proxy_group("g".to_string(), group).expect("group should be kept");
        assert_eq!(parsed.name, "g");
        assert_eq!(parsed.group_type, "URLTest");
        assert_eq!(parsed.now, "auto");
        assert_eq!(parsed.all.len(), 2);

        assert!(to_proxy_group("n".to_string(), node).is_none());
    }
}
