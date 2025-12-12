use crate::core::{MihomoClient, Result};
use std::collections::HashMap;

pub async fn test_delay(
    client: &MihomoClient,
    proxy: &str,
    test_url: &str,
    timeout: u32,
) -> Result<u32> {
    client.test_delay(proxy, test_url, timeout).await
}

pub async fn test_all_delays(
    client: &MihomoClient,
    test_url: &str,
    timeout: u32,
) -> Result<HashMap<String, u32>> {
    let proxies = client.get_proxies().await?;
    let mut results = HashMap::new();

    for (name, info) in proxies {
        if info.proxy_type != "Selector" && info.proxy_type != "URLTest" {
            if let Ok(delay) = client.test_delay(&name, test_url, timeout).await {
                results.insert(name, delay);
            }
        }
    }

    Ok(results)
}
