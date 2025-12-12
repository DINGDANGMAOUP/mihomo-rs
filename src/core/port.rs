use std::net::{TcpListener, ToSocketAddrs};

/// Check if a port is available on localhost
pub fn is_port_available(port: u16) -> bool {
    match TcpListener::bind(("127.0.0.1", port)) {
        Ok(_) => true,
        Err(_) => false,
    }
}

/// Find an available port starting from the given port
pub fn find_available_port(start_port: u16) -> Option<u16> {
    for port in start_port..start_port + 100 {
        if is_port_available(port) {
            return Some(port);
        }
    }
    None
}

/// Parse port from address string (e.g., "127.0.0.1:9090" -> 9090)
pub fn parse_port_from_addr(addr: &str) -> Option<u16> {
    addr.to_socket_addrs()
        .ok()?
        .next()
        .map(|socket_addr| socket_addr.port())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_port_available() {
        // Port 0 should always be available (OS assigns a free port)
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();

        // Port should not be available while listener is active
        assert!(!is_port_available(port));

        drop(listener);

        // Port should be available after listener is dropped
        assert!(is_port_available(port));
    }

    #[test]
    fn test_find_available_port() {
        let port = find_available_port(9090);
        assert!(port.is_some());
        assert!(port.unwrap() >= 9090);
    }

    #[test]
    fn test_parse_port_from_addr() {
        assert_eq!(parse_port_from_addr("127.0.0.1:9090"), Some(9090));
        assert_eq!(parse_port_from_addr("localhost:8080"), Some(8080));
        assert_eq!(parse_port_from_addr("invalid"), None);
    }
}
