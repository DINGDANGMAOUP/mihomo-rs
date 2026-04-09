mod common;

use futures_util::StreamExt;
use mihomo_rs::{MihomoClient, MihomoError};
use mockito::{Matcher, Server};
use tokio::net::TcpListener;
use tokio_tungstenite::{accept_async, tungstenite::Message as WsMessage};

#[tokio::test]
async fn get_version_reads_json_payload() {
    let mut server = Server::new_async().await;
    let version_mock = server
        .mock("GET", "/version")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"version":"v1.20.0","premium":true,"meta":false}"#)
        .create_async()
        .await;

    let client = MihomoClient::new(&server.url(), None).expect("create client");
    let version = client.get_version().await.expect("get version");

    version_mock.assert_async().await;
    assert_eq!(version.version, "v1.20.0");
    assert!(version.premium);
    assert!(!version.meta);
}

#[tokio::test]
async fn get_proxy_and_switch_proxy_use_encoded_path() {
    let mut server = Server::new_async().await;
    let group = "GLOBAL SELECT";

    let get_mock = server
        .mock("GET", "/proxies/GLOBAL%20SELECT")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"type":"Selector","now":"HK-01","all":["HK-01","JP-01"]}"#)
        .create_async()
        .await;

    let switch_mock = server
        .mock("PUT", "/proxies/GLOBAL%20SELECT")
        .match_header("content-type", Matcher::Regex("application/json".to_string()))
        .match_body(Matcher::JsonString(r#"{"name":"JP-01"}"#.to_string()))
        .with_status(204)
        .create_async()
        .await;

    let client = MihomoClient::new(&server.url(), None).expect("create client");
    let info = client.get_proxy(group).await.expect("get proxy");
    client
        .switch_proxy(group, "JP-01")
        .await
        .expect("switch proxy");

    get_mock.assert_async().await;
    switch_mock.assert_async().await;
    assert_eq!(info.now.as_deref(), Some("HK-01"));
}

#[tokio::test]
async fn delay_and_reload_config_without_path_send_expected_query() {
    let mut server = Server::new_async().await;

    let delay_mock = server
        .mock("GET", "/proxies/HK-01/delay")
        .match_query(Matcher::AllOf(vec![
            Matcher::UrlEncoded("timeout".into(), "5000".into()),
            Matcher::UrlEncoded("url".into(), "https://example.com".into()),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"delay":35}"#)
        .create_async()
        .await;

    let reload_without_path = server
        .mock("PUT", "/configs")
        .match_query(Matcher::UrlEncoded("force".into(), "true".into()))
        .with_status(204)
        .expect(1)
        .create_async()
        .await;

    let client = MihomoClient::new(&server.url(), None).expect("create client");
    let delay = client
        .test_delay("HK-01", "https://example.com", 5000)
        .await
        .expect("test delay");
    client
        .reload_config(None)
        .await
        .expect("reload without path");

    delay_mock.assert_async().await;
    reload_without_path.assert_async().await;
    assert_eq!(delay, 35);
}

#[tokio::test]
async fn reload_config_with_path_sends_json_body() {
    let mut server = Server::new_async().await;

    let reload_with_path = server
        .mock("PUT", "/configs")
        .match_query(Matcher::UrlEncoded("force".into(), "true".into()))
        .match_body(Matcher::JsonString(
            r#"{"path":"/tmp/mihomo.yaml"}"#.to_string(),
        ))
        .with_status(204)
        .create_async()
        .await;

    let client = MihomoClient::new(&server.url(), None).expect("create client");
    client
        .reload_config(Some("/tmp/mihomo.yaml"))
        .await
        .expect("reload with path");

    reload_with_path.assert_async().await;
}

#[tokio::test]
async fn memory_and_connection_endpoints_work() {
    let mut server = Server::new_async().await;

    let memory_mock = server
        .mock("GET", "/memory")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"inuse":1024,"oslimit":8192}"#)
        .create_async()
        .await;

    let list_connections = server
        .mock("GET", "/connections")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(common::mock_connections_payload())
        .create_async()
        .await;

    let close_one = server
        .mock("DELETE", "/connections/c1")
        .with_status(204)
        .create_async()
        .await;

    let close_all = server
        .mock("DELETE", "/connections")
        .with_status(204)
        .create_async()
        .await;

    let client = MihomoClient::new(&server.url(), None).expect("create client");
    let memory = client.get_memory().await.expect("get memory");
    let snapshot = client.get_connections().await.expect("get connections");
    client.close_connection("c1").await.expect("close one");
    client
        .close_all_connections()
        .await
        .expect("close all connections");

    memory_mock.assert_async().await;
    list_connections.assert_async().await;
    close_one.assert_async().await;
    close_all.assert_async().await;

    assert_eq!(memory.in_use, 1024);
    assert_eq!(snapshot.connections.len(), 2);
    assert_eq!(snapshot.download_total, 4096);
}

#[tokio::test]
async fn endpoints_return_error_on_invalid_json_payload() {
    let mut server = Server::new_async().await;

    let version_mock = server
        .mock("GET", "/version")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body("not-json")
        .expect(1)
        .create_async()
        .await;
    let proxies_mock = server
        .mock("GET", "/proxies")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body("not-json")
        .expect(1)
        .create_async()
        .await;
    let proxy_mock = server
        .mock("GET", "/proxies/SG-01")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body("not-json")
        .expect(1)
        .create_async()
        .await;
    let delay_mock = server
        .mock("GET", "/proxies/SG-01/delay")
        .match_query(Matcher::AllOf(vec![
            Matcher::UrlEncoded("timeout".into(), "3000".into()),
            Matcher::UrlEncoded("url".into(), "https://example.com".into()),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body("not-json")
        .expect(1)
        .create_async()
        .await;
    let memory_mock = server
        .mock("GET", "/memory")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body("not-json")
        .expect(1)
        .create_async()
        .await;
    let connections_mock = server
        .mock("GET", "/connections")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body("not-json")
        .expect(1)
        .create_async()
        .await;

    let client = MihomoClient::new(&server.url(), None).expect("create client");
    assert!(matches!(
        client.get_version().await.expect_err("version invalid json"),
        MihomoError::Json(_)
    ));
    assert!(matches!(
        client.get_proxies().await.expect_err("proxies invalid json"),
        MihomoError::Json(_)
    ));
    assert!(matches!(
        client
            .get_proxy("SG-01")
            .await
            .expect_err("proxy invalid json"),
        MihomoError::Json(_)
    ));
    assert!(matches!(
        client
            .test_delay("SG-01", "https://example.com", 3000)
            .await
            .expect_err("delay invalid json"),
        MihomoError::Json(_)
    ));
    assert!(matches!(
        client.get_memory().await.expect_err("memory invalid json"),
        MihomoError::Json(_)
    ));
    assert!(matches!(
        client
            .get_connections()
            .await
            .expect_err("connections invalid json"),
        MihomoError::Json(_)
    ));

    version_mock.assert_async().await;
    proxies_mock.assert_async().await;
    proxy_mock.assert_async().await;
    delay_mock.assert_async().await;
    memory_mock.assert_async().await;
    connections_mock.assert_async().await;
}

#[tokio::test]
async fn websocket_invalid_messages_are_ignored_then_close() {
    use futures_util::SinkExt;

    let logs_listener = TcpListener::bind("127.0.0.1:0").await.expect("bind logs");
    let logs_addr = logs_listener.local_addr().expect("logs addr");
    tokio::spawn(async move {
        let (stream, _) = logs_listener.accept().await.expect("accept logs");
        let ws = accept_async(stream).await.expect("accept ws logs");
        let (mut write, _) = ws.split();
        write
            .send(WsMessage::Binary(vec![1, 2, 3].into()))
            .await
            .expect("send binary logs");
        write
            .send(WsMessage::Close(None))
            .await
            .expect("send close logs");
    });
    let logs_client = MihomoClient::new(&format!("http://{}", logs_addr), None).expect("client");
    let mut logs_rx = logs_client.stream_logs(None).await.expect("stream logs");
    let logs_result = tokio::time::timeout(std::time::Duration::from_secs(1), logs_rx.recv())
        .await
        .expect("logs recv timeout");
    assert!(logs_result.is_none());

    let traffic_listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind traffic");
    let traffic_addr = traffic_listener.local_addr().expect("traffic addr");
    tokio::spawn(async move {
        let (stream, _) = traffic_listener.accept().await.expect("accept traffic");
        let ws = accept_async(stream).await.expect("accept ws traffic");
        let (mut write, _) = ws.split();
        write
            .send(WsMessage::Text("not-json".into()))
            .await
            .expect("send invalid traffic payload");
        write
            .send(WsMessage::Close(None))
            .await
            .expect("send close traffic");
    });
    let traffic_client =
        MihomoClient::new(&format!("http://{}", traffic_addr), None).expect("client");
    let mut traffic_rx = traffic_client
        .stream_traffic()
        .await
        .expect("stream traffic");
    let traffic_result =
        tokio::time::timeout(std::time::Duration::from_secs(1), traffic_rx.recv())
            .await
            .expect("traffic recv timeout");
    assert!(traffic_result.is_none());

    let conn_listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind connections");
    let conn_addr = conn_listener.local_addr().expect("connections addr");
    tokio::spawn(async move {
        let (stream, _) = conn_listener.accept().await.expect("accept connections");
        let ws = accept_async(stream).await.expect("accept ws connections");
        let (mut write, _) = ws.split();
        write
            .send(WsMessage::Text("not-json".into()))
            .await
            .expect("send invalid connections payload");
        write
            .send(WsMessage::Close(None))
            .await
            .expect("send close connections");
    });
    let conn_client = MihomoClient::new(&format!("http://{}", conn_addr), None).expect("client");
    let mut conn_rx = conn_client
        .stream_connections()
        .await
        .expect("stream connections");
    let conn_result = tokio::time::timeout(std::time::Duration::from_secs(1), conn_rx.recv())
        .await
        .expect("connections recv timeout");
    assert!(conn_result.is_none());
}

#[tokio::test]
async fn websocket_sender_breaks_when_receiver_is_dropped() {
    use futures_util::SinkExt;
    use tokio::sync::oneshot;

    let logs_listener = TcpListener::bind("127.0.0.1:0").await.expect("bind logs");
    let logs_addr = logs_listener.local_addr().expect("logs addr");
    let (logs_signal_tx, logs_signal_rx) = oneshot::channel::<()>();
    tokio::spawn(async move {
        let (stream, _) = logs_listener.accept().await.expect("accept logs");
        let ws = accept_async(stream).await.expect("accept ws logs");
        let (mut write, _) = ws.split();
        let _ = logs_signal_rx.await;
        write
            .send(WsMessage::Text("late log".into()))
            .await
            .expect("send log message");
    });
    let logs_client = MihomoClient::new(&format!("http://{}", logs_addr), None).expect("client");
    let logs_rx = logs_client.stream_logs(None).await.expect("stream logs");
    drop(logs_rx);
    let _ = logs_signal_tx.send(());

    let traffic_listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind traffic");
    let traffic_addr = traffic_listener.local_addr().expect("traffic addr");
    let (traffic_signal_tx, traffic_signal_rx) = oneshot::channel::<()>();
    tokio::spawn(async move {
        let (stream, _) = traffic_listener.accept().await.expect("accept traffic");
        let ws = accept_async(stream).await.expect("accept ws traffic");
        let (mut write, _) = ws.split();
        let _ = traffic_signal_rx.await;
        write
            .send(WsMessage::Text(r#"{"up":1,"down":2}"#.into()))
            .await
            .expect("send traffic message");
    });
    let traffic_client =
        MihomoClient::new(&format!("http://{}", traffic_addr), None).expect("client");
    let traffic_rx = traffic_client
        .stream_traffic()
        .await
        .expect("stream traffic");
    drop(traffic_rx);
    let _ = traffic_signal_tx.send(());

    let conn_listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind connections");
    let conn_addr = conn_listener.local_addr().expect("connections addr");
    let (conn_signal_tx, conn_signal_rx) = oneshot::channel::<()>();
    tokio::spawn(async move {
        let (stream, _) = conn_listener.accept().await.expect("accept connections");
        let ws = accept_async(stream).await.expect("accept ws connections");
        let (mut write, _) = ws.split();
        let _ = conn_signal_rx.await;
        write
            .send(WsMessage::Text(
                r#"{"connections":[],"downloadTotal":0,"uploadTotal":0}"#.into(),
            ))
            .await
            .expect("send connections message");
    });
    let conn_client = MihomoClient::new(&format!("http://{}", conn_addr), None).expect("client");
    let conn_rx = conn_client
        .stream_connections()
        .await
        .expect("stream connections");
    drop(conn_rx);
    let _ = conn_signal_tx.send(());

    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
}

#[tokio::test]
async fn websocket_binary_messages_are_ignored_for_traffic_and_connections() {
    use futures_util::SinkExt;

    let traffic_listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind traffic");
    let traffic_addr = traffic_listener.local_addr().expect("traffic addr");
    tokio::spawn(async move {
        let (stream, _) = traffic_listener.accept().await.expect("accept traffic");
        let ws = accept_async(stream).await.expect("accept ws traffic");
        let (mut write, _) = ws.split();
        write
            .send(WsMessage::Binary(vec![0x1, 0x2].into()))
            .await
            .expect("send binary traffic");
        write
            .send(WsMessage::Close(None))
            .await
            .expect("close traffic");
    });
    let traffic_client =
        MihomoClient::new(&format!("http://{}", traffic_addr), None).expect("client");
    let mut traffic_rx = traffic_client
        .stream_traffic()
        .await
        .expect("stream traffic");
    assert!(
        tokio::time::timeout(std::time::Duration::from_secs(1), traffic_rx.recv())
            .await
            .expect("traffic recv timeout")
            .is_none()
    );

    let conn_listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind connections");
    let conn_addr = conn_listener.local_addr().expect("connections addr");
    tokio::spawn(async move {
        let (stream, _) = conn_listener.accept().await.expect("accept connections");
        let ws = accept_async(stream).await.expect("accept ws connections");
        let (mut write, _) = ws.split();
        write
            .send(WsMessage::Binary(vec![0xA, 0xB].into()))
            .await
            .expect("send binary connections");
        write
            .send(WsMessage::Close(None))
            .await
            .expect("close connections");
    });
    let conn_client = MihomoClient::new(&format!("http://{}", conn_addr), None).expect("client");
    let mut conn_rx = conn_client
        .stream_connections()
        .await
        .expect("stream connections");
    assert!(
        tokio::time::timeout(std::time::Duration::from_secs(1), conn_rx.recv())
            .await
            .expect("connections recv timeout")
            .is_none()
    );
}
