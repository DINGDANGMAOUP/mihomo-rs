use futures_util::StreamExt;
use mihomo_rs::{MihomoClient, MihomoError};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

#[cfg(unix)]
fn unique_socket_path(prefix: &str) -> std::path::PathBuf {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    std::path::PathBuf::from(format!(
        "/tmp/mihomo-rs-{}-{}-{}.sock",
        prefix,
        std::process::id(),
        nanos
    ))
}

async fn spawn_hanging_tcp_server() -> std::net::SocketAddr {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind tcp listener");
    let addr = listener.local_addr().expect("listener addr");
    tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.expect("accept");
        let mut buf = [0u8; 1024];
        let _ = stream.read(&mut buf).await;
        tokio::time::sleep(Duration::from_millis(200)).await;
    });
    addr
}

#[tokio::test]
async fn tcp_websocket_timeouts_are_reported() {
    let logs_addr = spawn_hanging_tcp_server().await;
    let logs_client = MihomoClient::new(&format!("http://{}", logs_addr), Some("token".into()))
        .expect("create client")
        .with_ws_connect_timeout(Duration::from_millis(20));
    let logs_err = logs_client
        .stream_logs(Some("info"))
        .await
        .expect_err("logs should timeout");
    assert!(matches!(logs_err, MihomoError::Service(_)));

    let traffic_addr = spawn_hanging_tcp_server().await;
    let traffic_client =
        MihomoClient::new(&format!("http://{}", traffic_addr), Some("token".into()))
            .expect("create client")
            .with_ws_connect_timeout(Duration::from_millis(20));
    let traffic_err = traffic_client
        .stream_traffic()
        .await
        .expect_err("traffic should timeout");
    assert!(matches!(traffic_err, MihomoError::Service(_)));

    let conn_addr = spawn_hanging_tcp_server().await;
    let conn_client = MihomoClient::new(&format!("http://{}", conn_addr), Some("token".into()))
        .expect("create client")
        .with_ws_connect_timeout(Duration::from_millis(20));
    let conn_err = conn_client
        .stream_connections()
        .await
        .expect_err("connections should timeout");
    assert!(matches!(conn_err, MihomoError::Service(_)));
}

#[tokio::test]
async fn https_scheme_uses_wss_branch_and_fails_fast() {
    let client = MihomoClient::new("https://127.0.0.1:1", Some("token".into()))
        .expect("create https client")
        .with_ws_connect_timeout(Duration::from_millis(20));

    assert!(matches!(
        client
            .stream_logs(Some("debug"))
            .await
            .expect_err("logs should fail"),
        MihomoError::WebSocket(_) | MihomoError::Service(_)
    ));
    assert!(matches!(
        client.stream_traffic().await.expect_err("traffic should fail"),
        MihomoError::WebSocket(_) | MihomoError::Service(_)
    ));
    assert!(matches!(
        client
            .stream_connections()
            .await
            .expect_err("connections should fail"),
        MihomoError::WebSocket(_) | MihomoError::Service(_)
    ));
}

#[tokio::test]
#[cfg(unix)]
async fn unix_http_invalid_response_and_non_numeric_status_are_handled() {
    use tokio::net::UnixListener;

    let invalid_socket = unique_socket_path("invalid-http");
    let _ = std::fs::remove_file(&invalid_socket);
    let listener = UnixListener::bind(&invalid_socket).expect("bind invalid response socket");
    tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.expect("accept");
        let mut req_buf = [0u8; 1024];
        let _ = stream.read(&mut req_buf).await;
        stream
            .write_all(b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n")
            .await
            .expect("write malformed response");
    });
    let client = MihomoClient::new(invalid_socket.to_str().expect("socket path"), None)
        .expect("create client");
    let err = client
        .get_version()
        .await
        .expect_err("malformed unix response should fail");
    match err {
        MihomoError::Config(msg) => assert_eq!(msg, "Invalid HTTP response"),
        other => panic!("expected config error, got: {}", other),
    }
    let _ = std::fs::remove_file(&invalid_socket);

    let non_numeric_socket = unique_socket_path("nonnumeric-status");
    let _ = std::fs::remove_file(&non_numeric_socket);
    let listener = UnixListener::bind(&non_numeric_socket).expect("bind nonnumeric status socket");
    tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.expect("accept");
        let mut req_buf = [0u8; 1024];
        let _ = stream.read(&mut req_buf).await;
        let body = br#"{"version":"v1.2.3","premium":false,"meta":false}"#;
        let response = format!(
            "HTTP/1.1 XYZ Weird\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n",
            body.len()
        );
        stream
            .write_all(response.as_bytes())
            .await
            .expect("write status line");
        stream.write_all(body).await.expect("write body");
    });
    let client = MihomoClient::new(non_numeric_socket.to_str().expect("socket path"), None)
        .expect("create client");
    let version = client.get_version().await.expect("version should parse");
    assert_eq!(version.version, "v1.2.3");
    let _ = std::fs::remove_file(&non_numeric_socket);
}

#[tokio::test]
#[cfg(unix)]
async fn unix_websocket_timeouts_are_reported() {
    use tokio::net::UnixListener;

    let logs_socket = unique_socket_path("logs-timeout");
    let _ = std::fs::remove_file(&logs_socket);
    let listener = UnixListener::bind(&logs_socket).expect("bind logs socket");
    tokio::spawn(async move {
        let (_stream, _) = listener.accept().await.expect("accept");
        tokio::time::sleep(Duration::from_millis(200)).await;
    });
    let client = MihomoClient::new(logs_socket.to_str().expect("socket path"), None)
        .expect("create client")
        .with_ws_connect_timeout(Duration::from_millis(20));
    let err = client
        .stream_logs(Some("info"))
        .await
        .expect_err("unix logs should timeout");
    assert!(matches!(err, MihomoError::Service(_)));
    let _ = std::fs::remove_file(&logs_socket);

    let traffic_socket = unique_socket_path("traffic-timeout");
    let _ = std::fs::remove_file(&traffic_socket);
    let listener = UnixListener::bind(&traffic_socket).expect("bind traffic socket");
    tokio::spawn(async move {
        let (_stream, _) = listener.accept().await.expect("accept");
        tokio::time::sleep(Duration::from_millis(200)).await;
    });
    let client = MihomoClient::new(traffic_socket.to_str().expect("socket path"), None)
        .expect("create client")
        .with_ws_connect_timeout(Duration::from_millis(20));
    let err = client
        .stream_traffic()
        .await
        .expect_err("unix traffic should timeout");
    assert!(matches!(err, MihomoError::Service(_)));
    let _ = std::fs::remove_file(&traffic_socket);

    let conn_socket = unique_socket_path("conn-timeout");
    let _ = std::fs::remove_file(&conn_socket);
    let listener = UnixListener::bind(&conn_socket).expect("bind conn socket");
    tokio::spawn(async move {
        let (_stream, _) = listener.accept().await.expect("accept");
        tokio::time::sleep(Duration::from_millis(200)).await;
    });
    let client = MihomoClient::new(conn_socket.to_str().expect("socket path"), None)
        .expect("create client")
        .with_ws_connect_timeout(Duration::from_millis(20));
    let err = client
        .stream_connections()
        .await
        .expect_err("unix connections should timeout");
    assert!(matches!(err, MihomoError::Service(_)));
    let _ = std::fs::remove_file(&conn_socket);
}

#[tokio::test]
#[cfg(unix)]
async fn unix_http_memory_connections_and_close_paths_work() {
    use tokio::net::UnixListener;

    let socket = unique_socket_path("unix-http-ops");
    let _ = std::fs::remove_file(&socket);
    let listener = UnixListener::bind(&socket).expect("bind socket");

    tokio::spawn(async move {
        for _ in 0..4 {
            let (mut stream, _) = listener.accept().await.expect("accept");
            let mut buf = [0u8; 4096];
            let n = stream.read(&mut buf).await.expect("read request");
            let req = String::from_utf8_lossy(&buf[..n]).to_string();

            let response = if req.starts_with("GET /memory HTTP/1.1") {
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 28\r\n\r\n{\"inuse\":64,\"oslimit\":1024}".to_string()
            } else if req.starts_with("GET /connections HTTP/1.1") {
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 49\r\n\r\n{\"connections\":[],\"downloadTotal\":0,\"uploadTotal\":0}".to_string()
            } else if req.starts_with("DELETE /connections HTTP/1.1")
                || req.starts_with("DELETE /connections/c1 HTTP/1.1")
            {
                "HTTP/1.1 204 No Content\r\nContent-Length: 0\r\n\r\n".to_string()
            } else {
                "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\r\n".to_string()
            };

            stream
                .write_all(response.as_bytes())
                .await
                .expect("write response");
        }
    });

    let client = MihomoClient::new(socket.to_str().expect("socket path"), None).expect("client");
    let memory = client.get_memory().await.expect("get memory");
    let snapshot = client.get_connections().await.expect("get connections");
    client
        .close_all_connections()
        .await
        .expect("close all connections");
    client.close_connection("c1").await.expect("close one");

    assert_eq!(memory.in_use, 64);
    assert_eq!(snapshot.connections.len(), 0);
    let _ = std::fs::remove_file(&socket);
}

#[tokio::test]
#[cfg(unix)]
async fn unix_websocket_binary_and_close_messages_are_ignored() {
    use futures_util::SinkExt;
    use tokio::net::UnixListener;
    use tokio_tungstenite::{accept_async, tungstenite::Message as WsMessage};

    let logs_socket = unique_socket_path("unix-logs-binary-close");
    let _ = std::fs::remove_file(&logs_socket);
    let listener = UnixListener::bind(&logs_socket).expect("bind logs socket");
    tokio::spawn(async move {
        let (stream, _) = listener.accept().await.expect("accept");
        let ws = accept_async(stream).await.expect("accept ws");
        let (mut tx, _) = ws.split();
        tx.send(WsMessage::Binary(vec![1u8, 2u8].into()))
            .await
            .expect("send binary");
        tx.send(WsMessage::Close(None)).await.expect("send close");
    });
    let client = MihomoClient::new(logs_socket.to_str().expect("socket path"), None).unwrap();
    let mut rx = client.stream_logs(None).await.expect("stream logs");
    assert!(
        tokio::time::timeout(Duration::from_secs(1), rx.recv())
            .await
            .expect("recv timeout")
            .is_none()
    );
    let _ = std::fs::remove_file(&logs_socket);

    let traffic_socket = unique_socket_path("unix-traffic-binary-close");
    let _ = std::fs::remove_file(&traffic_socket);
    let listener = UnixListener::bind(&traffic_socket).expect("bind traffic socket");
    tokio::spawn(async move {
        let (stream, _) = listener.accept().await.expect("accept");
        let ws = accept_async(stream).await.expect("accept ws");
        let (mut tx, _) = ws.split();
        tx.send(WsMessage::Binary(vec![3u8, 4u8].into()))
            .await
            .expect("send binary");
        tx.send(WsMessage::Close(None)).await.expect("send close");
    });
    let client = MihomoClient::new(traffic_socket.to_str().expect("socket path"), None).unwrap();
    let mut rx = client.stream_traffic().await.expect("stream traffic");
    assert!(
        tokio::time::timeout(Duration::from_secs(1), rx.recv())
            .await
            .expect("recv timeout")
            .is_none()
    );
    let _ = std::fs::remove_file(&traffic_socket);

    let conn_socket = unique_socket_path("unix-connections-binary-close");
    let _ = std::fs::remove_file(&conn_socket);
    let listener = UnixListener::bind(&conn_socket).expect("bind conn socket");
    tokio::spawn(async move {
        let (stream, _) = listener.accept().await.expect("accept");
        let ws = accept_async(stream).await.expect("accept ws");
        let (mut tx, _) = ws.split();
        tx.send(WsMessage::Binary(vec![5u8, 6u8].into()))
            .await
            .expect("send binary");
        tx.send(WsMessage::Close(None)).await.expect("send close");
    });
    let client = MihomoClient::new(conn_socket.to_str().expect("socket path"), None).unwrap();
    let mut rx = client.stream_connections().await.expect("stream connections");
    assert!(
        tokio::time::timeout(Duration::from_secs(1), rx.recv())
            .await
            .expect("recv timeout")
            .is_none()
    );
    let _ = std::fs::remove_file(&conn_socket);
}

#[tokio::test]
#[cfg(unix)]
async fn unix_traffic_and_connections_invalid_text_then_close() {
    use futures_util::SinkExt;
    use tokio::net::UnixListener;
    use tokio_tungstenite::{accept_async, tungstenite::Message as WsMessage};

    let traffic_socket = unique_socket_path("unix-traffic-invalid-text");
    let _ = std::fs::remove_file(&traffic_socket);
    let listener = UnixListener::bind(&traffic_socket).expect("bind traffic socket");
    tokio::spawn(async move {
        let (stream, _) = listener.accept().await.expect("accept");
        let ws = accept_async(stream).await.expect("accept ws");
        let (mut tx, _) = ws.split();
        tx.send(WsMessage::Text("not-json".into()))
            .await
            .expect("send invalid text");
        tx.send(WsMessage::Close(None)).await.expect("send close");
    });
    let client = MihomoClient::new(traffic_socket.to_str().expect("socket path"), None).unwrap();
    let mut rx = client.stream_traffic().await.expect("stream traffic");
    assert!(
        tokio::time::timeout(Duration::from_secs(1), rx.recv())
            .await
            .expect("recv timeout")
            .is_none()
    );
    let _ = std::fs::remove_file(&traffic_socket);

    let conn_socket = unique_socket_path("unix-connections-invalid-text");
    let _ = std::fs::remove_file(&conn_socket);
    let listener = UnixListener::bind(&conn_socket).expect("bind conn socket");
    tokio::spawn(async move {
        let (stream, _) = listener.accept().await.expect("accept");
        let ws = accept_async(stream).await.expect("accept ws");
        let (mut tx, _) = ws.split();
        tx.send(WsMessage::Text("not-json".into()))
            .await
            .expect("send invalid text");
        tx.send(WsMessage::Close(None)).await.expect("send close");
    });
    let client = MihomoClient::new(conn_socket.to_str().expect("socket path"), None).unwrap();
    let mut rx = client.stream_connections().await.expect("stream connections");
    assert!(
        tokio::time::timeout(Duration::from_secs(1), rx.recv())
            .await
            .expect("recv timeout")
            .is_none()
    );
    let _ = std::fs::remove_file(&conn_socket);
}
