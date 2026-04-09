mod common;

use mihomo_rs::{MihomoClient, ProxyManager};
use mockito::Server;

#[tokio::test]
async fn list_proxies_filters_groups_and_sorts_nodes() {
    let mut server = Server::new_async().await;
    let proxies_mock = server
        .mock("GET", "/proxies")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(common::mock_proxies_payload())
        .create_async()
        .await;

    let client = MihomoClient::new(&server.url(), None).expect("create client");
    let manager = ProxyManager::new(client);

    let nodes = manager.list_proxies().await.expect("list proxies");

    proxies_mock.assert_async().await;
    assert_eq!(nodes.len(), 2);
    assert_eq!(nodes[0].name, "HK-01");
    assert_eq!(nodes[0].delay, Some(35));
    assert!(nodes[0].alive);
    assert_eq!(nodes[1].name, "JP-01");
    assert_eq!(nodes[1].delay, None);
    assert!(!nodes[1].alive);
}

#[tokio::test]
async fn list_groups_switch_and_get_current() {
    let mut server = Server::new_async().await;

    let groups_mock = server
        .mock("GET", "/proxies")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(common::mock_proxies_payload())
        .expect(2)
        .create_async()
        .await;

    let switch_mock = server
        .mock("PUT", "/proxies/GLOBAL")
        .with_status(204)
        .create_async()
        .await;

    let current_mock = server
        .mock("GET", "/proxies/GLOBAL")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"type":"Selector","now":"JP-01","all":["HK-01","JP-01"]}"#)
        .create_async()
        .await;

    let client = MihomoClient::new(&server.url(), None).expect("create client");
    let manager = ProxyManager::new(client);

    let groups = manager.list_groups().await.expect("list groups");
    manager
        .switch("GLOBAL", "JP-01")
        .await
        .expect("switch group");
    let current = manager
        .get_current("GLOBAL")
        .await
        .expect("get current proxy");
    let all = manager
        .get_all_proxies()
        .await
        .expect("get all proxies map");

    groups_mock.assert_async().await;
    switch_mock.assert_async().await;
    current_mock.assert_async().await;

    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].name, "GLOBAL");
    assert_eq!(current, "JP-01");
    assert_eq!(all.len(), 3);
}
