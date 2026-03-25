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

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::{Matcher, Server};

    #[tokio::test]
    async fn helper_test_delay_forwards_request() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("GET", "/proxies/proxy1/delay")
            .match_query(Matcher::AllOf(vec![
                Matcher::UrlEncoded("timeout".into(), "5000".into()),
                Matcher::UrlEncoded("url".into(), "http://www.gstatic.com/generate_204".into()),
            ]))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"delay":88}"#)
            .create_async()
            .await;

        let client = MihomoClient::new(&server.url(), None).unwrap();
        let delay = test_delay(
            &client,
            "proxy1",
            "http://www.gstatic.com/generate_204",
            5000,
        )
        .await
        .unwrap();

        mock.assert_async().await;
        assert_eq!(delay, 88);
    }

    #[tokio::test]
    async fn helper_test_all_delays_skips_groups_and_failed_proxy() {
        let mut server = Server::new_async().await;
        let proxies_mock = server
            .mock("GET", "/proxies")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                    "proxies": {
                        "SelectorGroup": {"type":"Selector","history":[]},
                        "UrlGroup": {"type":"URLTest","history":[]},
                        "DIRECT": {"type":"Direct","history":[]},
                        "ProxyFail": {"type":"ss","history":[]}
                    }
                }"#,
            )
            .create_async()
            .await;
        let direct_mock = server
            .mock("GET", "/proxies/DIRECT/delay")
            .match_query(Matcher::AllOf(vec![
                Matcher::UrlEncoded("timeout".into(), "3000".into()),
                Matcher::UrlEncoded("url".into(), "http://www.gstatic.com/generate_204".into()),
            ]))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"delay":42}"#)
            .create_async()
            .await;
        let fail_mock = server
            .mock("GET", "/proxies/ProxyFail/delay")
            .match_query(Matcher::AllOf(vec![
                Matcher::UrlEncoded("timeout".into(), "3000".into()),
                Matcher::UrlEncoded("url".into(), "http://www.gstatic.com/generate_204".into()),
            ]))
            .with_status(500)
            .with_body("failed")
            .create_async()
            .await;

        let client = MihomoClient::new(&server.url(), None).unwrap();
        let delays = test_all_delays(&client, "http://www.gstatic.com/generate_204", 3000)
            .await
            .unwrap();

        proxies_mock.assert_async().await;
        direct_mock.assert_async().await;
        fail_mock.assert_async().await;

        assert_eq!(delays.len(), 1);
        assert_eq!(delays.get("DIRECT"), Some(&42));
        assert!(!delays.contains_key("SelectorGroup"));
        assert!(!delays.contains_key("UrlGroup"));
        assert!(!delays.contains_key("ProxyFail"));
    }
}
