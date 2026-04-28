mod common;

use futures_util::StreamExt;
use mihomo_rs::{ConnectionManager, MihomoClient};
use mockito::Server;
use tokio::net::TcpListener;
use tokio_tungstenite::{accept_async, tungstenite::Message as WsMessage};

#[tokio::test]
async fn list_get_all_and_statistics_are_consistent() {
    let mut server = Server::new_async().await;
    let connections_mock = server
        .mock("GET", "/connections")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(common::mock_connections_payload())
        .expect(3)
        .create_async()
        .await;

    let client = MihomoClient::new(&server.url(), None).expect("create client");
    let manager = ConnectionManager::new(client);

    let listed = manager.list().await.expect("list connections");
    let all = manager.get_all().await.expect("get all connections");
    let stats = manager.get_statistics().await.expect("get statistics");

    connections_mock.assert_async().await;

    assert_eq!(listed.len(), 2);
    assert_eq!(all.connections.len(), 2);
    assert_eq!(stats, (4096, 2048, 2));
}

#[tokio::test]
async fn filter_methods_match_expected_connections() {
    let mut server = Server::new_async().await;
    let connections_mock = server
        .mock("GET", "/connections")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(common::mock_connections_payload())
        .expect(4)
        .create_async()
        .await;

    let client = MihomoClient::new(&server.url(), None).expect("create client");
    let manager = ConnectionManager::new(client);

    let by_host = manager
        .filter_by_host("example")
        .await
        .expect("filter by host");
    let by_ip = manager
        .filter_by_host("8.8.8.8")
        .await
        .expect("filter by ip");
    let by_process = manager
        .filter_by_process("Firefox")
        .await
        .expect("filter by process");
    let by_rule = manager
        .filter_by_rule("MATCH")
        .await
        .expect("filter by rule");

    connections_mock.assert_async().await;

    assert_eq!(by_host.len(), 1);
    assert_eq!(by_host[0].id, "c1");
    assert_eq!(by_ip.len(), 1);
    assert_eq!(by_ip[0].id, "c2");

    assert_eq!(by_process.len(), 1);
    assert_eq!(by_process[0].id, "c2");

    assert_eq!(by_rule.len(), 1);
    assert_eq!(by_rule[0].id, "c2");
}

#[tokio::test]
async fn close_methods_hit_expected_endpoints() {
    let mut server = Server::new_async().await;

    let list_for_close_by_host = server
        .mock("GET", "/connections")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(common::mock_connections_payload())
        .create_async()
        .await;

    let close_c1 = server
        .mock("DELETE", "/connections/c1")
        .with_status(204)
        .expect(2)
        .create_async()
        .await;

    let close_all = server
        .mock("DELETE", "/connections")
        .with_status(204)
        .create_async()
        .await;

    let client = MihomoClient::new(&server.url(), None).expect("create client");
    let manager = ConnectionManager::new(client);

    let closed = manager
        .close_by_host("example")
        .await
        .expect("close by host");
    manager.close("c1").await.expect("close single");
    manager.close_all().await.expect("close all");

    list_for_close_by_host.assert_async().await;
    close_c1.assert_async().await;
    close_all.assert_async().await;

    assert_eq!(closed, 1);
}

#[tokio::test]
async fn close_by_process_only_closes_matched_connections() {
    let mut server = Server::new_async().await;

    let list_for_close_by_process = server
        .mock("GET", "/connections")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(common::mock_connections_payload())
        .create_async()
        .await;

    let close_c2 = server
        .mock("DELETE", "/connections/c2")
        .with_status(204)
        .expect(1)
        .create_async()
        .await;

    let client = MihomoClient::new(&server.url(), None).expect("create client");
    let manager = ConnectionManager::new(client);

    let closed = manager
        .close_by_process("Firefox")
        .await
        .expect("close by process");

    list_for_close_by_process.assert_async().await;
    close_c2.assert_async().await;
    assert_eq!(closed, 1);
}

#[tokio::test]
async fn stream_forwards_connection_snapshots() {
    use futures_util::SinkExt;

    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind listener");
    let addr = listener.local_addr().expect("listener addr");

    tokio::spawn(async move {
        let (stream, _) = listener.accept().await.expect("accept connection");
        let ws = accept_async(stream).await.expect("accept websocket");
        let (mut tx, _) = ws.split();
        tx.send(WsMessage::Text(
            r#"{"connections":[],"downloadTotal":0,"uploadTotal":0}"#.into(),
        ))
        .await
        .expect("send snapshot");
    });

    let client = MihomoClient::new(&format!("http://{}", addr), None).expect("create client");
    let manager = ConnectionManager::new(client);
    let mut rx = manager.stream().await.expect("stream snapshots");
    let snapshot = tokio::time::timeout(std::time::Duration::from_secs(1), rx.recv())
        .await
        .expect("recv timeout")
        .expect("snapshot item");
    assert_eq!(snapshot.connections.len(), 0);
}
