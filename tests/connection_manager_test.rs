// Integration tests for ConnectionManager

use mihomo_rs::connection::ConnectionManager;
use mihomo_rs::core::MihomoClient;
use mockito::Server;

#[tokio::test]
async fn test_connection_manager_list() {
    let mut server = Server::new_async().await;
    let mock = server
        .mock("GET", "/connections")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{
            "connections": [
                {
                    "id": "test-id-1",
                    "metadata": {
                        "network": "tcp",
                        "type": "HTTP",
                        "sourceIP": "192.168.1.100",
                        "destinationIP": "1.1.1.1",
                        "sourcePort": "54321",
                        "destinationPort": "443",
                        "host": "example.com",
                        "dnsMode": "normal",
                        "processPath": "/usr/bin/curl",
                        "specialProxy": ""
                    },
                    "upload": 1024,
                    "download": 2048,
                    "start": "2024-01-01T00:00:00Z",
                    "chains": ["DIRECT"],
                    "rule": "DIRECT",
                    "rulePayload": ""
                }
            ],
            "downloadTotal": 2048,
            "uploadTotal": 1024
        }"#,
        )
        .create_async()
        .await;

    let client = MihomoClient::new(&server.url(), None).unwrap();
    let manager = ConnectionManager::new(client);
    let result = manager.list().await;

    mock.assert_async().await;
    assert!(result.is_ok());
    let connections = result.unwrap();
    assert_eq!(connections.len(), 1);
    assert_eq!(connections[0].id, "test-id-1");
    assert_eq!(connections[0].metadata.host, "example.com");
}

#[tokio::test]
async fn test_connection_manager_get_all() {
    let mut server = Server::new_async().await;
    let mock = server
        .mock("GET", "/connections")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{
            "connections": [],
            "downloadTotal": 5000,
            "uploadTotal": 3000
        }"#,
        )
        .create_async()
        .await;

    let client = MihomoClient::new(&server.url(), None).unwrap();
    let manager = ConnectionManager::new(client);
    let result = manager.get_all().await;

    mock.assert_async().await;
    assert!(result.is_ok());
    let response = result.unwrap();
    assert_eq!(response.connections.len(), 0);
    assert_eq!(response.download_total, 5000);
    assert_eq!(response.upload_total, 3000);
}

#[tokio::test]
async fn test_connection_manager_close() {
    let mut server = Server::new_async().await;
    let mock = server
        .mock("DELETE", "/connections/test-id-123")
        .with_status(204)
        .create_async()
        .await;

    let client = MihomoClient::new(&server.url(), None).unwrap();
    let manager = ConnectionManager::new(client);
    let result = manager.close("test-id-123").await;

    mock.assert_async().await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_connection_manager_close_all() {
    let mut server = Server::new_async().await;
    let mock = server
        .mock("DELETE", "/connections")
        .with_status(204)
        .create_async()
        .await;

    let client = MihomoClient::new(&server.url(), None).unwrap();
    let manager = ConnectionManager::new(client);
    let result = manager.close_all().await;

    mock.assert_async().await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_connection_manager_filter_by_host() {
    let mut server = Server::new_async().await;
    let mock = server
        .mock("GET", "/connections")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{
            "connections": [
                {
                    "id": "conn-1",
                    "metadata": {
                        "network": "tcp",
                        "type": "HTTP",
                        "sourceIP": "192.168.1.100",
                        "destinationIP": "1.1.1.1",
                        "sourcePort": "54321",
                        "destinationPort": "443",
                        "host": "example.com",
                        "dnsMode": "normal",
                        "processPath": "/usr/bin/curl",
                        "specialProxy": ""
                    },
                    "upload": 1024,
                    "download": 2048,
                    "start": "2024-01-01T00:00:00Z",
                    "chains": ["DIRECT"],
                    "rule": "DIRECT",
                    "rulePayload": ""
                },
                {
                    "id": "conn-2",
                    "metadata": {
                        "network": "tcp",
                        "type": "HTTP",
                        "sourceIP": "192.168.1.100",
                        "destinationIP": "2.2.2.2",
                        "sourcePort": "54322",
                        "destinationPort": "443",
                        "host": "another.com",
                        "dnsMode": "normal",
                        "processPath": "/usr/bin/wget",
                        "specialProxy": ""
                    },
                    "upload": 512,
                    "download": 1024,
                    "start": "2024-01-01T00:01:00Z",
                    "chains": ["DIRECT"],
                    "rule": "DIRECT",
                    "rulePayload": ""
                },
                {
                    "id": "conn-3",
                    "metadata": {
                        "network": "tcp",
                        "type": "HTTPS",
                        "sourceIP": "192.168.1.100",
                        "destinationIP": "1.1.1.1",
                        "sourcePort": "54323",
                        "destinationPort": "443",
                        "host": "example.org",
                        "dnsMode": "normal",
                        "processPath": "/usr/bin/curl",
                        "specialProxy": ""
                    },
                    "upload": 2048,
                    "download": 4096,
                    "start": "2024-01-01T00:02:00Z",
                    "chains": ["PROXY"],
                    "rule": "MATCH",
                    "rulePayload": ""
                }
            ],
            "downloadTotal": 7168,
            "uploadTotal": 3584
        }"#,
        )
        .create_async()
        .await;

    let client = MihomoClient::new(&server.url(), None).unwrap();
    let manager = ConnectionManager::new(client);
    let result = manager.filter_by_host("example").await;

    mock.assert_async().await;
    assert!(result.is_ok());
    let filtered = result.unwrap();
    assert_eq!(filtered.len(), 2); // example.com and example.org
    assert!(filtered.iter().all(|c| c.metadata.host.contains("example")));
}

#[tokio::test]
async fn test_connection_manager_filter_by_process() {
    let mut server = Server::new_async().await;
    let mock = server
        .mock("GET", "/connections")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{
            "connections": [
                {
                    "id": "conn-1",
                    "metadata": {
                        "network": "tcp",
                        "type": "HTTP",
                        "sourceIP": "192.168.1.100",
                        "destinationIP": "1.1.1.1",
                        "sourcePort": "54321",
                        "destinationPort": "443",
                        "host": "example.com",
                        "dnsMode": "normal",
                        "processPath": "/usr/bin/curl",
                        "specialProxy": ""
                    },
                    "upload": 1024,
                    "download": 2048,
                    "start": "2024-01-01T00:00:00Z",
                    "chains": ["DIRECT"],
                    "rule": "DIRECT",
                    "rulePayload": ""
                },
                {
                    "id": "conn-2",
                    "metadata": {
                        "network": "tcp",
                        "type": "HTTP",
                        "sourceIP": "192.168.1.100",
                        "destinationIP": "2.2.2.2",
                        "sourcePort": "54322",
                        "destinationPort": "443",
                        "host": "another.com",
                        "dnsMode": "normal",
                        "processPath": "/usr/bin/wget",
                        "specialProxy": ""
                    },
                    "upload": 512,
                    "download": 1024,
                    "start": "2024-01-01T00:01:00Z",
                    "chains": ["DIRECT"],
                    "rule": "DIRECT",
                    "rulePayload": ""
                }
            ],
            "downloadTotal": 3072,
            "uploadTotal": 1536
        }"#,
        )
        .create_async()
        .await;

    let client = MihomoClient::new(&server.url(), None).unwrap();
    let manager = ConnectionManager::new(client);
    let result = manager.filter_by_process("curl").await;

    mock.assert_async().await;
    assert!(result.is_ok());
    let filtered = result.unwrap();
    assert_eq!(filtered.len(), 1);
    assert!(filtered[0].metadata.process_path.contains("curl"));
}

#[tokio::test]
async fn test_connection_manager_filter_by_rule() {
    let mut server = Server::new_async().await;
    let mock = server
        .mock("GET", "/connections")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{
            "connections": [
                {
                    "id": "conn-1",
                    "metadata": {
                        "network": "tcp",
                        "type": "HTTP",
                        "sourceIP": "192.168.1.100",
                        "destinationIP": "1.1.1.1",
                        "sourcePort": "54321",
                        "destinationPort": "443",
                        "host": "example.com",
                        "dnsMode": "normal",
                        "processPath": "/usr/bin/curl",
                        "specialProxy": ""
                    },
                    "upload": 1024,
                    "download": 2048,
                    "start": "2024-01-01T00:00:00Z",
                    "chains": ["DIRECT"],
                    "rule": "DIRECT",
                    "rulePayload": ""
                },
                {
                    "id": "conn-2",
                    "metadata": {
                        "network": "tcp",
                        "type": "HTTP",
                        "sourceIP": "192.168.1.100",
                        "destinationIP": "2.2.2.2",
                        "sourcePort": "54322",
                        "destinationPort": "443",
                        "host": "another.com",
                        "dnsMode": "normal",
                        "processPath": "/usr/bin/wget",
                        "specialProxy": ""
                    },
                    "upload": 512,
                    "download": 1024,
                    "start": "2024-01-01T00:01:00Z",
                    "chains": ["PROXY"],
                    "rule": "MATCH",
                    "rulePayload": ""
                }
            ],
            "downloadTotal": 3072,
            "uploadTotal": 1536
        }"#,
        )
        .create_async()
        .await;

    let client = MihomoClient::new(&server.url(), None).unwrap();
    let manager = ConnectionManager::new(client);
    let result = manager.filter_by_rule("DIRECT").await;

    mock.assert_async().await;
    assert!(result.is_ok());
    let filtered = result.unwrap();
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].rule, "DIRECT");
}

#[tokio::test]
async fn test_connection_manager_get_statistics() {
    let mut server = Server::new_async().await;
    let mock = server
        .mock("GET", "/connections")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{
            "connections": [
                {
                    "id": "conn-1",
                    "metadata": {
                        "network": "tcp",
                        "type": "HTTP",
                        "sourceIP": "192.168.1.100",
                        "destinationIP": "1.1.1.1",
                        "sourcePort": "54321",
                        "destinationPort": "443",
                        "host": "example.com",
                        "dnsMode": "normal",
                        "processPath": "/usr/bin/curl",
                        "specialProxy": ""
                    },
                    "upload": 1024,
                    "download": 2048,
                    "start": "2024-01-01T00:00:00Z",
                    "chains": ["DIRECT"],
                    "rule": "DIRECT",
                    "rulePayload": ""
                },
                {
                    "id": "conn-2",
                    "metadata": {
                        "network": "tcp",
                        "type": "HTTP",
                        "sourceIP": "192.168.1.100",
                        "destinationIP": "2.2.2.2",
                        "sourcePort": "54322",
                        "destinationPort": "443",
                        "host": "another.com",
                        "dnsMode": "normal",
                        "processPath": "/usr/bin/wget",
                        "specialProxy": ""
                    },
                    "upload": 512,
                    "download": 1024,
                    "start": "2024-01-01T00:01:00Z",
                    "chains": ["DIRECT"],
                    "rule": "DIRECT",
                    "rulePayload": ""
                }
            ],
            "downloadTotal": 100000,
            "uploadTotal": 50000
        }"#,
        )
        .create_async()
        .await;

    let client = MihomoClient::new(&server.url(), None).unwrap();
    let manager = ConnectionManager::new(client);
    let result = manager.get_statistics().await;

    mock.assert_async().await;
    assert!(result.is_ok());
    let (download, upload, count) = result.unwrap();
    assert_eq!(download, 100000);
    assert_eq!(upload, 50000);
    assert_eq!(count, 2);
}

#[tokio::test]
async fn test_connection_manager_close_by_host() {
    let mut server = Server::new_async().await;

    // Mock for listing connections
    let list_mock = server
        .mock("GET", "/connections")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{
            "connections": [
                {
                    "id": "conn-1",
                    "metadata": {
                        "network": "tcp",
                        "type": "HTTP",
                        "sourceIP": "192.168.1.100",
                        "destinationIP": "1.1.1.1",
                        "sourcePort": "54321",
                        "destinationPort": "443",
                        "host": "example.com",
                        "dnsMode": "normal",
                        "processPath": "/usr/bin/curl",
                        "specialProxy": ""
                    },
                    "upload": 1024,
                    "download": 2048,
                    "start": "2024-01-01T00:00:00Z",
                    "chains": ["DIRECT"],
                    "rule": "DIRECT",
                    "rulePayload": ""
                },
                {
                    "id": "conn-2",
                    "metadata": {
                        "network": "tcp",
                        "type": "HTTP",
                        "sourceIP": "192.168.1.100",
                        "destinationIP": "1.1.1.1",
                        "sourcePort": "54322",
                        "destinationPort": "443",
                        "host": "example.org",
                        "dnsMode": "normal",
                        "processPath": "/usr/bin/curl",
                        "specialProxy": ""
                    },
                    "upload": 1024,
                    "download": 2048,
                    "start": "2024-01-01T00:00:00Z",
                    "chains": ["DIRECT"],
                    "rule": "DIRECT",
                    "rulePayload": ""
                }
            ],
            "downloadTotal": 4096,
            "uploadTotal": 2048
        }"#,
        )
        .create_async()
        .await;

    // Mock for closing first connection
    let close_mock_1 = server
        .mock("DELETE", "/connections/conn-1")
        .with_status(204)
        .create_async()
        .await;

    let client = MihomoClient::new(&server.url(), None).unwrap();
    let manager = ConnectionManager::new(client);
    let result = manager.close_by_host("example.com").await;

    list_mock.assert_async().await;
    close_mock_1.assert_async().await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 1);
}

#[tokio::test]
async fn test_connection_manager_close_by_process() {
    let mut server = Server::new_async().await;

    // Mock for listing connections
    let list_mock = server
        .mock("GET", "/connections")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{
            "connections": [
                {
                    "id": "conn-1",
                    "metadata": {
                        "network": "tcp",
                        "type": "HTTP",
                        "sourceIP": "192.168.1.100",
                        "destinationIP": "1.1.1.1",
                        "sourcePort": "54321",
                        "destinationPort": "443",
                        "host": "example.com",
                        "dnsMode": "normal",
                        "processPath": "/usr/bin/curl",
                        "specialProxy": ""
                    },
                    "upload": 1024,
                    "download": 2048,
                    "start": "2024-01-01T00:00:00Z",
                    "chains": ["DIRECT"],
                    "rule": "DIRECT",
                    "rulePayload": ""
                },
                {
                    "id": "conn-2",
                    "metadata": {
                        "network": "tcp",
                        "type": "HTTP",
                        "sourceIP": "192.168.1.100",
                        "destinationIP": "2.2.2.2",
                        "sourcePort": "54322",
                        "destinationPort": "443",
                        "host": "another.com",
                        "dnsMode": "normal",
                        "processPath": "/usr/bin/wget",
                        "specialProxy": ""
                    },
                    "upload": 512,
                    "download": 1024,
                    "start": "2024-01-01T00:01:00Z",
                    "chains": ["DIRECT"],
                    "rule": "DIRECT",
                    "rulePayload": ""
                }
            ],
            "downloadTotal": 3072,
            "uploadTotal": 1536
        }"#,
        )
        .create_async()
        .await;

    // Mock for closing the curl connection
    let close_mock = server
        .mock("DELETE", "/connections/conn-1")
        .with_status(204)
        .create_async()
        .await;

    let client = MihomoClient::new(&server.url(), None).unwrap();
    let manager = ConnectionManager::new(client);
    let result = manager.close_by_process("curl").await;

    list_mock.assert_async().await;
    close_mock.assert_async().await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 1);
}

#[tokio::test]
async fn test_connection_manager_filter_empty_results() {
    let mut server = Server::new_async().await;
    let mock = server
        .mock("GET", "/connections")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{
            "connections": [
                {
                    "id": "conn-1",
                    "metadata": {
                        "network": "tcp",
                        "type": "HTTP",
                        "sourceIP": "192.168.1.100",
                        "destinationIP": "1.1.1.1",
                        "sourcePort": "54321",
                        "destinationPort": "443",
                        "host": "example.com",
                        "dnsMode": "normal",
                        "processPath": "/usr/bin/curl",
                        "specialProxy": ""
                    },
                    "upload": 1024,
                    "download": 2048,
                    "start": "2024-01-01T00:00:00Z",
                    "chains": ["DIRECT"],
                    "rule": "DIRECT",
                    "rulePayload": ""
                }
            ],
            "downloadTotal": 2048,
            "uploadTotal": 1024
        }"#,
        )
        .create_async()
        .await;

    let client = MihomoClient::new(&server.url(), None).unwrap();
    let manager = ConnectionManager::new(client);

    // Test filter by non-existent host
    let result = manager.filter_by_host("nonexistent.com").await;
    mock.assert_async().await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap().len(), 0);
}
