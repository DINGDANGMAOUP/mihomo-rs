use super::error::Result;
use super::types::*;
use reqwest::Client;
use serde_json::json;
use std::collections::HashMap;
use url::Url;

#[derive(Clone)]
pub struct MihomoClient {
    client: Client,
    base_url: Url,
    secret: Option<String>,
}

impl MihomoClient {
    pub fn new(base_url: &str, secret: Option<String>) -> Result<Self> {
        let base_url = Url::parse(base_url)?;
        let client = Client::new();
        Ok(Self {
            client,
            base_url,
            secret,
        })
    }

    fn build_url(&self, path: &str) -> Result<Url> {
        Ok(self.base_url.join(path)?)
    }

    fn add_auth(&self, mut req: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        if let Some(secret) = &self.secret {
            req = req.bearer_auth(secret);
        }
        req
    }

    pub async fn get_version(&self) -> Result<Version> {
        let url = self.build_url("/version")?;
        let req = self.client.get(url);
        let req = self.add_auth(req);
        let resp = req.send().await?;
        Ok(resp.json().await?)
    }

    pub async fn get_proxies(&self) -> Result<HashMap<String, ProxyInfo>> {
        let url = self.build_url("/proxies")?;
        log::debug!("Fetching proxies from: {}", url);
        let req = self.client.get(url);
        let req = self.add_auth(req);
        let resp = req.send().await?;
        let data: ProxiesResponse = resp.json().await?;
        log::debug!("Received {} proxies", data.proxies.len());
        Ok(data.proxies)
    }

    pub async fn get_proxy(&self, name: &str) -> Result<ProxyInfo> {
        let url = self.build_url(&format!("/proxies/{}", name))?;
        let req = self.client.get(url);
        let req = self.add_auth(req);
        let resp = req.send().await?;
        Ok(resp.json().await?)
    }

    pub async fn switch_proxy(&self, group: &str, proxy: &str) -> Result<()> {
        let url = self.build_url(&format!("/proxies/{}", group))?;
        log::debug!("Switching group '{}' to proxy '{}' at {}", group, proxy, url);
        let req = self.client.put(url).json(&json!({ "name": proxy }));
        let req = self.add_auth(req);
        req.send().await?;
        log::debug!("Successfully switched group '{}' to '{}'", group, proxy);
        Ok(())
    }

    pub async fn test_delay(&self, proxy: &str, test_url: &str, timeout: u32) -> Result<u32> {
        let url = self.build_url(&format!("/proxies/{}/delay", proxy))?;
        let req = self.client.get(url).query(&[
            ("timeout", timeout.to_string()),
            ("url", test_url.to_string()),
        ]);
        let req = self.add_auth(req);
        let resp = req.send().await?;
        let data: DelayTestResponse = resp.json().await?;
        Ok(data.delay)
    }

    pub async fn reload_config(&self, path: Option<&str>) -> Result<()> {
        let url = self.build_url("/configs")?;
        let mut req = self.client.put(url);
        if let Some(path) = path {
            req = req.query(&[("force", "true")]).json(&json!({ "path": path }));
        } else {
            req = req.query(&[("force", "true")]);
        }
        let req = self.add_auth(req);
        req.send().await?;
        Ok(())
    }
}
