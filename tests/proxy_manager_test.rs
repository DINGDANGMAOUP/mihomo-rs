mod common;

use mihomo_rs::{core::MihomoClient, ProxyManager, Result};
use mockito::{Matcher, Server};

#[tokio::test]
async fn test_list_proxies_filters_groups() -> Result<()> {
    let mut server = Server::new_async().await;
    let mock = server
        .mock("GET", "/proxies")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{
                "proxies": {
                    "GLOBAL": {"type":"Selector","history":[]},
                    "DIRECT": {"type":"Direct","history":[{"time":"2026-01-01T00:00:00Z","delay":12}]},
                    "ss-node": {"type":"ss","history":[]}
                }
            }"#,
        )
        .create_async()
        .await;

    let client = MihomoClient::new(&server.url(), None)?;
    let pm = ProxyManager::new(client);
    let nodes = pm.list_proxies().await?;

    mock.assert_async().await;
    assert_eq!(nodes.len(), 2);
    assert_eq!(nodes[0].name, "DIRECT");
    assert_eq!(nodes[0].delay, Some(12));
    assert_eq!(nodes[1].name, "ss-node");
    assert_eq!(nodes[1].delay, None);

    Ok(())
}

#[tokio::test]
async fn test_list_groups_returns_only_groups() -> Result<()> {
    let mut server = Server::new_async().await;
    let mock = server
        .mock("GET", "/proxies")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{
                "proxies": {
                    "GLOBAL": {"type":"Selector","now":"DIRECT","all":["DIRECT","ss-node"],"history":[]},
                    "AUTO": {"type":"URLTest","now":"ss-node","all":["DIRECT","ss-node"],"history":[]},
                    "DIRECT": {"type":"Direct","history":[]}
                }
            }"#,
        )
        .create_async()
        .await;

    let client = MihomoClient::new(&server.url(), None)?;
    let pm = ProxyManager::new(client);
    let groups = pm.list_groups().await?;

    mock.assert_async().await;
    assert_eq!(groups.len(), 2);
    assert_eq!(groups[0].name, "AUTO");
    assert_eq!(groups[1].name, "GLOBAL");

    Ok(())
}

#[tokio::test]
async fn test_switch_and_get_current() -> Result<()> {
    let mut server = Server::new_async().await;
    let switch_mock = server
        .mock("PUT", "/proxies/GLOBAL")
        .match_body(Matcher::Json(serde_json::json!({"name":"DIRECT"})))
        .with_status(204)
        .create_async()
        .await;
    let get_mock = server
        .mock("GET", "/proxies/GLOBAL")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"type":"Selector","now":"DIRECT","all":["DIRECT"],"history":[]}"#)
        .create_async()
        .await;

    let client = MihomoClient::new(&server.url(), None)?;
    let pm = ProxyManager::new(client);

    pm.switch("GLOBAL", "DIRECT").await?;
    let current = pm.get_current("GLOBAL").await?;

    switch_mock.assert_async().await;
    get_mock.assert_async().await;
    assert_eq!(current, "DIRECT");

    Ok(())
}

#[tokio::test]
async fn test_get_all_proxies_passthrough() -> Result<()> {
    let mut server = Server::new_async().await;
    let mock = server
        .mock("GET", "/proxies")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"proxies":{"DIRECT":{"type":"Direct","history":[]}}}"#)
        .create_async()
        .await;

    let client = MihomoClient::new(&server.url(), None)?;
    let pm = ProxyManager::new(client);
    let proxies = pm.get_all_proxies().await?;

    mock.assert_async().await;
    assert!(proxies.contains_key("DIRECT"));

    Ok(())
}
