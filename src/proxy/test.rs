use crate::core::{MihomoClient, Result};
use std::collections::HashMap;

fn is_group_type(proxy_type: &str) -> bool {
    matches!(
        proxy_type,
        "Selector" | "URLTest" | "Fallback" | "LoadBalance" | "Relay"
    )
}

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
        if !is_group_type(&info.proxy_type) {
            if let Ok(delay) = client.test_delay(&name, test_url, timeout).await {
                results.insert(name, delay);
            }
        }
    }

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::is_group_type;

    #[test]
    fn test_is_group_type() {
        assert!(is_group_type("Selector"));
        assert!(is_group_type("URLTest"));
        assert!(is_group_type("Fallback"));
        assert!(is_group_type("LoadBalance"));
        assert!(is_group_type("Relay"));
        assert!(!is_group_type("Direct"));
        assert!(!is_group_type("Reject"));
    }
}
