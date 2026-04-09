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
    use super::{is_group_type, test_all_delays, test_delay};
    use crate::core::MihomoClient;
    use mockito::Server;

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

    #[tokio::test]
    async fn test_delay_for_single_proxy() {
        let mut server = Server::new_async().await;
        let delay_mock = server
            .mock("GET", "/proxies/HK-01/delay")
            .match_query(mockito::Matcher::AllOf(vec![
                mockito::Matcher::UrlEncoded("timeout".into(), "3000".into()),
                mockito::Matcher::UrlEncoded("url".into(), "https://example.com".into()),
            ]))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"delay":42}"#)
            .create_async()
            .await;

        let client = MihomoClient::new(&server.url(), None).expect("create client");
        let delay = test_delay(&client, "HK-01", "https://example.com", 3000)
            .await
            .expect("test delay");
        delay_mock.assert_async().await;
        assert_eq!(delay, 42);
    }

    #[tokio::test]
    async fn test_all_delays_ignores_groups_and_failed_nodes() {
        let mut server = Server::new_async().await;
        let proxies = server
            .mock("GET", "/proxies")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                    "proxies": {
                        "GLOBAL": {"type":"Selector","now":"HK 01","all":["HK 01","JP-01"]},
                        "HK 01": {"type":"Shadowsocks","history":[]},
                        "JP-01": {"type":"Shadowsocks","history":[]}
                    }
                }"#,
            )
            .create_async()
            .await;

        let delay_ok = server
            .mock("GET", "/proxies/HK%2001/delay")
            .match_query(mockito::Matcher::AllOf(vec![
                mockito::Matcher::UrlEncoded("timeout".into(), "5000".into()),
                mockito::Matcher::UrlEncoded("url".into(), "https://example.com".into()),
            ]))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"delay":88}"#)
            .create_async()
            .await;

        let delay_fail = server
            .mock("GET", "/proxies/JP-01/delay")
            .match_query(mockito::Matcher::AllOf(vec![
                mockito::Matcher::UrlEncoded("timeout".into(), "5000".into()),
                mockito::Matcher::UrlEncoded("url".into(), "https://example.com".into()),
            ]))
            .with_status(500)
            .create_async()
            .await;

        let client = MihomoClient::new(&server.url(), None).expect("create client");
        let result = test_all_delays(&client, "https://example.com", 5000)
            .await
            .expect("test all delays");

        proxies.assert_async().await;
        delay_ok.assert_async().await;
        delay_fail.assert_async().await;

        assert_eq!(result.len(), 1);
        assert_eq!(result.get("HK 01"), Some(&88));
        assert!(!result.contains_key("GLOBAL"));
        assert!(!result.contains_key("JP-01"));
    }
}
