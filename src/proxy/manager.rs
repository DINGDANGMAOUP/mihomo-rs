use crate::core::{MihomoClient, ProxyGroup, ProxyInfo, ProxyNode, Result};
use std::collections::HashMap;

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
            if info.proxy_type != "Selector" && info.proxy_type != "URLTest" {
                let delay = info.history.first().map(|h| h.delay);
                nodes.push(ProxyNode {
                    name,
                    proxy_type: info.proxy_type,
                    delay,
                    alive: delay.is_some(),
                });
            }
        }

        nodes.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(nodes)
    }

    pub async fn list_groups(&self) -> Result<Vec<ProxyGroup>> {
        let proxies = self.client.get_proxies().await?;
        let mut groups = vec![];

        for (name, info) in proxies {
            if info.proxy_type == "Selector" || info.proxy_type == "URLTest" {
                groups.push(ProxyGroup {
                    name,
                    group_type: info.proxy_type,
                    now: info.now.unwrap_or_default(),
                    all: info.all.unwrap_or_default(),
                });
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
