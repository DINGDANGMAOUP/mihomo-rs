use crate::core::{Connection, ConnectionSnapshot, ConnectionsResponse, MihomoClient, Result};

pub struct ConnectionManager {
    client: MihomoClient,
}

impl ConnectionManager {
    pub fn new(client: MihomoClient) -> Self {
        Self { client }
    }

    pub async fn list(&self) -> Result<Vec<Connection>> {
        let response = self.client.get_connections().await?;
        log::debug!("Listed {} active connections", response.connections.len());
        Ok(response.connections)
    }

    pub async fn get_all(&self) -> Result<ConnectionsResponse> {
        self.client.get_connections().await
    }

    pub async fn close(&self, id: &str) -> Result<()> {
        self.client.close_connection(id).await
    }

    pub async fn close_all(&self) -> Result<()> {
        self.client.close_all_connections().await
    }

    pub async fn filter_by_host(&self, host: &str) -> Result<Vec<Connection>> {
        let connections = self.list().await?;
        let filtered: Vec<Connection> = connections
            .into_iter()
            .filter(|c| matches_host_filter(c, host))
            .collect();
        log::debug!(
            "Filtered {} connections matching host '{}'",
            filtered.len(),
            host
        );
        Ok(filtered)
    }

    pub async fn filter_by_process(&self, process: &str) -> Result<Vec<Connection>> {
        let connections = self.list().await?;
        let filtered: Vec<Connection> = connections
            .into_iter()
            .filter(|c| c.metadata.process_path.contains(process))
            .collect();
        log::debug!(
            "Filtered {} connections matching process '{}'",
            filtered.len(),
            process
        );
        Ok(filtered)
    }

    pub async fn filter_by_rule(&self, rule: &str) -> Result<Vec<Connection>> {
        let connections = self.list().await?;
        let filtered: Vec<Connection> = connections
            .into_iter()
            .filter(|c| c.rule.contains(rule))
            .collect();
        log::debug!(
            "Filtered {} connections matching rule '{}'",
            filtered.len(),
            rule
        );
        Ok(filtered)
    }

    pub async fn get_statistics(&self) -> Result<(u64, u64, usize)> {
        let response = self.client.get_connections().await?;
        Ok((
            response.download_total,
            response.upload_total,
            response.connections.len(),
        ))
    }

    pub async fn stream(&self) -> Result<tokio::sync::mpsc::UnboundedReceiver<ConnectionSnapshot>> {
        self.client.stream_connections().await
    }

    pub async fn close_by_host(&self, host: &str) -> Result<usize> {
        let connections = self.filter_by_host(host).await?;
        let count = connections.len();
        for conn in connections {
            self.close(&conn.id).await?;
        }
        log::debug!("Closed {} connections for host '{}'", count, host);
        Ok(count)
    }

    pub async fn close_by_process(&self, process: &str) -> Result<usize> {
        let connections = self.filter_by_process(process).await?;
        let count = connections.len();
        for conn in connections {
            self.close(&conn.id).await?;
        }
        log::debug!("Closed {} connections for process '{}'", count, process);
        Ok(count)
    }
}

fn matches_host_filter(connection: &Connection, host_filter: &str) -> bool {
    connection.metadata.host.contains(host_filter)
        || connection.metadata.destination_ip.contains(host_filter)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{Connection, ConnectionMetadata};
    use mockito::Server;

    // Helper function to create test connection
    fn create_test_connection(id: &str, host: &str, process: &str, rule: &str) -> Connection {
        Connection {
            id: id.to_string(),
            metadata: ConnectionMetadata {
                network: "tcp".to_string(),
                connection_type: "HTTP".to_string(),
                source_ip: "192.168.1.1".to_string(),
                destination_ip: "1.1.1.1".to_string(),
                source_port: "12345".to_string(),
                destination_port: "443".to_string(),
                host: host.to_string(),
                dns_mode: "normal".to_string(),
                process_path: process.to_string(),
                special_proxy: String::new(),
            },
            upload: 1024,
            download: 2048,
            start: "2024-01-01T00:00:00Z".to_string(),
            chains: vec!["DIRECT".to_string()],
            rule: rule.to_string(),
            rule_payload: String::new(),
        }
    }

    #[test]
    fn test_connection_manager_new() {
        let client = MihomoClient::new("http://127.0.0.1:9090", None).unwrap();
        let manager = ConnectionManager::new(client);
        // Just verify it can be created
        assert!(std::mem::size_of_val(&manager) > 0);
    }

    #[test]
    fn test_create_test_connection() {
        let conn = create_test_connection("test-id", "example.com", "/usr/bin/app", "DIRECT");

        assert_eq!(conn.id, "test-id");
        assert_eq!(conn.metadata.host, "example.com");
        assert_eq!(conn.metadata.process_path, "/usr/bin/app");
        assert_eq!(conn.rule, "DIRECT");
        assert_eq!(conn.upload, 1024);
        assert_eq!(conn.download, 2048);
    }

    #[test]
    fn host_filter_matches_host_or_destination_ip() {
        let by_host = create_test_connection("by-host", "example.com", "/usr/bin/app", "DIRECT");
        let mut by_ip = create_test_connection("by-ip", "", "/usr/bin/app", "DIRECT");
        by_ip.metadata.destination_ip = "4.4.4.4".to_string();

        assert!(matches_host_filter(&by_host, "example"));
        assert!(matches_host_filter(&by_ip, "4.4.4"));
        assert!(!matches_host_filter(&by_ip, "example"));
    }

    #[tokio::test]
    async fn test_manager_methods_with_mock_server() {
        let mut server = Server::new_async().await;
        let payload = r#"{"connections":[{"id":"c1","metadata":{"network":"tcp","type":"HTTP","sourceIP":"10.0.0.2","destinationIP":"1.1.1.1","sourcePort":"52345","destinationPort":"443","host":"example.com","dnsMode":"normal","processPath":"/Applications/Safari.app"},"upload":12,"download":34,"start":"2024-01-01T00:00:00Z","chains":["DIRECT"],"rule":"MATCH","rulePayload":""}],"downloadTotal":34,"uploadTotal":12}"#;

        let list_mock = server
            .mock("GET", "/connections")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(payload)
            .expect(7)
            .create_async()
            .await;
        let close_mock = server
            .mock("DELETE", "/connections/c1")
            .with_status(204)
            .expect(2)
            .create_async()
            .await;

        let client = MihomoClient::new(&server.url(), None).expect("create client");
        let manager = ConnectionManager::new(client);

        let listed = manager.list().await.expect("list");
        assert_eq!(listed.len(), 1);
        assert_eq!(
            manager.get_all().await.expect("get_all").connections.len(),
            1
        );
        assert_eq!(
            manager.filter_by_host("example").await.expect("host").len(),
            1
        );
        assert_eq!(manager.filter_by_host("1.1.1").await.expect("ip").len(), 1);
        assert_eq!(
            manager
                .filter_by_process("Safari")
                .await
                .expect("process")
                .len(),
            1
        );
        assert_eq!(
            manager.filter_by_rule("MATCH").await.expect("rule").len(),
            1
        );
        assert_eq!(
            manager.get_statistics().await.expect("statistics"),
            (34, 12, 1)
        );

        manager.close("c1").await.expect("close one");
        assert_eq!(
            manager.close_by_host("example").await.expect("close host"),
            1
        );

        list_mock.assert_async().await;
        close_mock.assert_async().await;
    }
}
