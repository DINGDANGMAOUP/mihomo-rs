#![allow(clippy::result_large_err)]

mod common;

use mihomo_rs::cli::{
    run_cli_command, Commands, ConfigAction, ConnectionAction, ProxyAction, ServiceAction,
    VersionAction,
};
use mihomo_rs::{ConfigManager, VersionManager};
use mockito::{Matcher, Server};
use std::env;
use std::path::Path;
use std::sync::OnceLock;
use tempfile::tempdir;
use tokio::sync::Mutex;
use tokio_tungstenite::accept_hdr_async;
use tokio_tungstenite::tungstenite::handshake::server::{Request, Response};
use tokio_tungstenite::tungstenite::Message;

fn env_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

async fn run_local_ws_server_for_logs_and_traffic() -> String {
    use futures_util::SinkExt;
    use tokio::net::TcpListener;

    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind ws listener");
    let addr = listener.local_addr().expect("listener addr");

    tokio::spawn(async move {
        let (stream_logs, _) = listener.accept().await.expect("accept logs ws");
        let mut ws_logs = accept_hdr_async(stream_logs, |_req: &Request, resp: Response| Ok(resp))
            .await
            .expect("accept logs handshake");
        ws_logs
            .send(Message::Text("log line".to_string().into()))
            .await
            .expect("send logs message");
        ws_logs
            .send(Message::Close(None))
            .await
            .expect("close logs ws");

        let (stream_traffic, _) = listener.accept().await.expect("accept traffic ws");
        let mut ws_traffic =
            accept_hdr_async(stream_traffic, |_req: &Request, resp: Response| Ok(resp))
                .await
                .expect("accept traffic handshake");
        ws_traffic
            .send(Message::Text(
                "{\"up\":2048,\"down\":1024}".to_string().into(),
            ))
            .await
            .expect("send traffic message");
        ws_traffic
            .send(Message::Close(None))
            .await
            .expect("close traffic ws");
    });

    format!("http://{}", addr)
}

async fn write_executable_script(path: &Path, body: &str) {
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;
    use tokio::fs;

    fs::write(path, body).await.expect("write script");
    #[cfg(unix)]
    {
        let mut perms = fs::metadata(path)
            .await
            .expect("script metadata")
            .permissions();
        perms.set_mode(0o755);
        fs::set_permissions(path, perms)
            .await
            .expect("set executable permissions");
    }
}

#[cfg(unix)]
async fn with_mocked_stdin<T, Fut>(input: &str, fut: Fut) -> T
where
    Fut: std::future::Future<Output = T>,
{
    use std::ffi::c_void;
    use std::os::fd::RawFd;

    unsafe extern "C" {
        fn pipe(fds: *mut i32) -> i32;
        fn dup(fd: i32) -> i32;
        fn dup2(oldfd: i32, newfd: i32) -> i32;
        fn close(fd: i32) -> i32;
        fn write(fd: i32, buf: *const c_void, count: usize) -> isize;
    }

    const STDIN_FILENO: RawFd = 0;
    let mut fds = [0_i32; 2];
    // SAFETY: libc calls with valid pointers and checked return codes.
    let saved_stdin = unsafe {
        assert_eq!(pipe(fds.as_mut_ptr()), 0, "pipe failed");
        let duped = dup(STDIN_FILENO);
        assert!(duped >= 0, "dup stdin failed");
        let bytes = input.as_bytes();
        assert_eq!(
            write(fds[1], bytes.as_ptr().cast::<c_void>(), bytes.len()),
            bytes.len() as isize,
            "write mocked stdin failed"
        );
        assert_eq!(close(fds[1]), 0, "close write end failed");
        assert_eq!(
            dup2(fds[0], STDIN_FILENO),
            STDIN_FILENO,
            "dup2 stdin failed"
        );
        assert_eq!(close(fds[0]), 0, "close read end failed");
        duped
    };

    let output = fut.await;

    // SAFETY: restore previous stdin file descriptor.
    unsafe {
        assert_eq!(
            dup2(saved_stdin, STDIN_FILENO),
            STDIN_FILENO,
            "restore stdin failed"
        );
        assert_eq!(close(saved_stdin), 0, "close saved stdin failed");
    }

    output
}

#[tokio::test]
async fn run_cli_command_covers_config_version_and_service_paths() {
    let _guard = env_lock().lock().await;

    let temp = tempdir().expect("create temp dir");
    let old_home = env::var("MIHOMO_HOME").ok();
    env::set_var("MIHOMO_HOME", temp.path());

    let cm = ConfigManager::new().expect("config manager");
    cm.save(
        "default",
        "port: 7890\nexternal-controller: 127.0.0.1:9090\n",
    )
    .await
    .expect("write default profile");
    cm.save("alt", "port: 7891\nexternal-controller: 127.0.0.1:9090\n")
        .await
        .expect("write alt profile");
    cm.set_current("default")
        .await
        .expect("set default profile current");

    run_cli_command(Commands::Config {
        action: ConfigAction::List,
    })
    .await
    .expect("config list");
    run_cli_command(Commands::Config {
        action: ConfigAction::Current,
    })
    .await
    .expect("config current");
    run_cli_command(Commands::Config {
        action: ConfigAction::Path,
    })
    .await
    .expect("config path");
    run_cli_command(Commands::Config {
        action: ConfigAction::Set {
            key: mihomo_rs::cli::ConfigKey::ConfigsDir,
            value: "icloud/configs".to_string(),
        },
    })
    .await
    .expect("config set configs-dir");
    run_cli_command(Commands::Config {
        action: ConfigAction::Unset {
            key: mihomo_rs::cli::ConfigKey::ConfigsDir,
        },
    })
    .await
    .expect("config unset configs-dir");
    run_cli_command(Commands::Config {
        action: ConfigAction::Show {
            profile: Some("default".to_string()),
        },
    })
    .await
    .expect("config show");
    run_cli_command(Commands::Config {
        action: ConfigAction::Use {
            profile: "alt".to_string(),
        },
    })
    .await
    .expect("config use");
    run_cli_command(Commands::Config {
        action: ConfigAction::Delete {
            profile: "default".to_string(),
        },
    })
    .await
    .expect("config delete");

    common::install_fake_version(temp.path(), "v1.2.3").await;
    common::install_fake_version(temp.path(), "v1.2.4").await;

    run_cli_command(Commands::Default {
        version: "v1.2.3".to_string(),
    })
    .await
    .expect("set default version");
    run_cli_command(Commands::List)
        .await
        .expect("list installed versions");
    run_cli_command(Commands::Uninstall {
        version: "v1.2.4".to_string(),
    })
    .await
    .expect("uninstall non-default version");

    let install_existing = run_cli_command(Commands::Install {
        version: Some("v1.2.3".to_string()),
    })
    .await;
    assert!(install_existing.is_err());

    run_cli_command(Commands::Status)
        .await
        .expect("service status on fresh pid file");

    assert!(run_cli_command(Commands::Start).await.is_err());
    assert!(run_cli_command(Commands::Restart).await.is_err());
    assert!(run_cli_command(Commands::Stop).await.is_err());

    // Keep one direct manager call to exercise constructor path in this test context.
    let _vm = VersionManager::new().expect("version manager new");

    if let Some(value) = old_home {
        env::set_var("MIHOMO_HOME", value);
    } else {
        env::remove_var("MIHOMO_HOME");
    }
}

#[tokio::test]
async fn run_cli_command_covers_proxy_connection_and_memory_paths() {
    let _guard = env_lock().lock().await;

    let temp = tempdir().expect("create temp dir");
    let old_home = env::var("MIHOMO_HOME").ok();
    env::set_var("MIHOMO_HOME", temp.path());

    let mut server = Server::new_async().await;
    let controller = server.url();
    let default_profile = format!("port: 7890\nexternal-controller: {}\n", controller);

    let cm = ConfigManager::new().expect("config manager");
    cm.save("default", &default_profile)
        .await
        .expect("write default profile");
    cm.set_current("default")
        .await
        .expect("set default profile current");

    let proxies_payload = common::mock_proxies_payload();
    let connections_payload = common::mock_connections_payload();

    let mock_get_proxies = server
        .mock("GET", "/proxies")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(proxies_payload)
        .expect(4)
        .create_async()
        .await;
    let mock_switch = server
        .mock("PUT", "/proxies/GLOBAL")
        .with_status(204)
        .expect(1)
        .create_async()
        .await;
    let mock_delay_hk = server
        .mock("GET", "/proxies/HK-01/delay")
        .match_query(Matcher::AllOf(vec![
            Matcher::UrlEncoded("timeout".into(), "5000".into()),
            Matcher::UrlEncoded("url".into(), "http://www.gstatic.com/generate_204".into()),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"delay":35}"#)
        .expect(2)
        .create_async()
        .await;
    let mock_delay_jp = server
        .mock("GET", "/proxies/JP-01/delay")
        .match_query(Matcher::AllOf(vec![
            Matcher::UrlEncoded("timeout".into(), "5000".into()),
            Matcher::UrlEncoded("url".into(), "http://www.gstatic.com/generate_204".into()),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"delay":88}"#)
        .expect(1)
        .create_async()
        .await;

    let mock_get_connections = server
        .mock("GET", "/connections")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(connections_payload)
        .expect(8)
        .create_async()
        .await;
    let mock_close_one = server
        .mock("DELETE", "/connections/c1")
        .with_status(204)
        .expect(3)
        .create_async()
        .await;
    let mock_close_all = server
        .mock("DELETE", "/connections")
        .with_status(204)
        .expect(1)
        .create_async()
        .await;
    let mock_memory = server
        .mock("GET", "/memory")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"inuse":1048576,"oslimit":2097152}"#)
        .expect(1)
        .create_async()
        .await;

    run_cli_command(Commands::Proxy {
        action: ProxyAction::List,
    })
    .await
    .expect("proxy list");
    run_cli_command(Commands::Proxy {
        action: ProxyAction::Groups,
    })
    .await
    .expect("proxy groups");
    run_cli_command(Commands::Proxy {
        action: ProxyAction::Current,
    })
    .await
    .expect("proxy current");
    run_cli_command(Commands::Proxy {
        action: ProxyAction::Switch {
            group: "GLOBAL".to_string(),
            proxy: "HK-01".to_string(),
        },
    })
    .await
    .expect("proxy switch");
    run_cli_command(Commands::Proxy {
        action: ProxyAction::Test {
            proxy: Some("HK-01".to_string()),
            url: "http://www.gstatic.com/generate_204".to_string(),
            timeout: 5000,
        },
    })
    .await
    .expect("proxy test single");
    run_cli_command(Commands::Proxy {
        action: ProxyAction::Test {
            proxy: None,
            url: "http://www.gstatic.com/generate_204".to_string(),
            timeout: 5000,
        },
    })
    .await
    .expect("proxy test all");

    run_cli_command(Commands::Connection {
        action: ConnectionAction::List {
            host: None,
            process: None,
        },
    })
    .await
    .expect("connection list");
    run_cli_command(Commands::Connection {
        action: ConnectionAction::Stats,
    })
    .await
    .expect("connection stats");
    run_cli_command(Commands::Connection {
        action: ConnectionAction::List {
            host: Some("example".to_string()),
            process: None,
        },
    })
    .await
    .expect("connection filter host");
    run_cli_command(Commands::Connection {
        action: ConnectionAction::List {
            host: None,
            process: Some("curl".to_string()),
        },
    })
    .await
    .expect("connection filter process");
    run_cli_command(Commands::Connection {
        action: ConnectionAction::Close {
            legacy_id: None,
            id: Some("c1".to_string()),
            all: false,
            host: None,
            process: None,
            force: false,
        },
    })
    .await
    .expect("connection close");
    run_cli_command(Commands::Connection {
        action: ConnectionAction::Close {
            legacy_id: None,
            id: None,
            all: true,
            host: None,
            process: None,
            force: true,
        },
    })
    .await
    .expect("connection close all");
    run_cli_command(Commands::Connection {
        action: ConnectionAction::Close {
            legacy_id: None,
            id: None,
            all: false,
            host: Some("example".to_string()),
            process: None,
            force: true,
        },
    })
    .await
    .expect("connection close by host");
    run_cli_command(Commands::Connection {
        action: ConnectionAction::Close {
            legacy_id: None,
            id: None,
            all: false,
            host: None,
            process: Some("curl".to_string()),
            force: true,
        },
    })
    .await
    .expect("connection close by process");

    run_cli_command(Commands::Memory)
        .await
        .expect("memory command");

    mock_get_proxies.assert_async().await;
    mock_switch.assert_async().await;
    mock_delay_hk.assert_async().await;
    mock_delay_jp.assert_async().await;
    mock_get_connections.assert_async().await;
    mock_close_one.assert_async().await;
    mock_close_all.assert_async().await;
    mock_memory.assert_async().await;

    if let Some(value) = old_home {
        env::set_var("MIHOMO_HOME", value);
    } else {
        env::remove_var("MIHOMO_HOME");
    }
}

#[tokio::test]
async fn run_cli_command_covers_logs_traffic_and_version_network_error_paths() {
    let _guard = env_lock().lock().await;

    let temp = tempdir().expect("create temp dir");
    let old_home = env::var("MIHOMO_HOME").ok();
    let old_http_proxy = env::var("HTTP_PROXY").ok();
    let old_https_proxy = env::var("HTTPS_PROXY").ok();
    let old_all_proxy = env::var("ALL_PROXY").ok();
    let old_no_proxy = env::var("NO_PROXY").ok();

    env::set_var("MIHOMO_HOME", temp.path());
    env::set_var("HTTP_PROXY", "http://127.0.0.1:9");
    env::set_var("HTTPS_PROXY", "http://127.0.0.1:9");
    env::set_var("ALL_PROXY", "http://127.0.0.1:9");
    env::remove_var("NO_PROXY");

    let controller = run_local_ws_server_for_logs_and_traffic().await;
    let profile = format!("port: 7890\nexternal-controller: {}\n", controller);
    let cm = ConfigManager::new().expect("config manager");
    cm.save("default", &profile)
        .await
        .expect("write default profile");
    cm.set_current("default")
        .await
        .expect("set current profile");

    run_cli_command(Commands::Logs {
        level: Some("info".to_string()),
    })
    .await
    .expect("logs stream");
    run_cli_command(Commands::Traffic)
        .await
        .expect("traffic stream");

    assert!(run_cli_command(Commands::Install {
        version: Some("stable".to_string()),
    })
    .await
    .is_err());
    assert!(run_cli_command(Commands::Update).await.is_err());
    assert!(run_cli_command(Commands::ListRemote { limit: 1 })
        .await
        .is_err());

    if let Some(value) = old_home {
        env::set_var("MIHOMO_HOME", value);
    } else {
        env::remove_var("MIHOMO_HOME");
    }
    if let Some(value) = old_http_proxy {
        env::set_var("HTTP_PROXY", value);
    } else {
        env::remove_var("HTTP_PROXY");
    }
    if let Some(value) = old_https_proxy {
        env::set_var("HTTPS_PROXY", value);
    } else {
        env::remove_var("HTTPS_PROXY");
    }
    if let Some(value) = old_all_proxy {
        env::set_var("ALL_PROXY", value);
    } else {
        env::remove_var("ALL_PROXY");
    }
    if let Some(value) = old_no_proxy {
        env::set_var("NO_PROXY", value);
    } else {
        env::remove_var("NO_PROXY");
    }
}

#[cfg(unix)]
#[tokio::test]
async fn run_cli_command_covers_service_success_lifecycle() {
    let _guard = env_lock().lock().await;

    let temp = tempdir().expect("create temp dir");
    let old_home = env::var("MIHOMO_HOME").ok();
    env::set_var("MIHOMO_HOME", temp.path());

    let cm = ConfigManager::new().expect("config manager");
    cm.save(
        "default",
        "port: 7890\nexternal-controller: https://example.com:443\n",
    )
    .await
    .expect("write default profile");
    cm.set_current("default")
        .await
        .expect("set current profile");

    let vm = VersionManager::new().expect("version manager");
    let binary_name = "mihomo";
    let binary_path = temp.path().join("versions/v9.9.9").join(binary_name);
    if let Some(parent) = binary_path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .expect("create version dir");
    }
    write_executable_script(&binary_path, "#!/bin/sh\nsleep 30\n").await;
    vm.set_default("v9.9.9").await.expect("set default version");

    run_cli_command(Commands::Start)
        .await
        .expect("service start");
    run_cli_command(Commands::Status)
        .await
        .expect("service status");
    run_cli_command(Commands::Restart)
        .await
        .expect("service restart");
    run_cli_command(Commands::Stop).await.expect("service stop");
    run_cli_command(Commands::Status)
        .await
        .expect("service status stopped");

    if let Some(value) = old_home {
        env::set_var("MIHOMO_HOME", value);
    } else {
        env::remove_var("MIHOMO_HOME");
    }
}

#[tokio::test]
async fn run_cli_command_covers_connection_stream_and_empty_branches() {
    let _guard = env_lock().lock().await;

    let temp = tempdir().expect("create temp dir");
    let old_home = env::var("MIHOMO_HOME").ok();
    env::set_var("MIHOMO_HOME", temp.path());

    let mut server = Server::new_async().await;
    let controller = server.url();
    let default_profile = format!("port: 7890\nexternal-controller: {}\n", controller);
    let cm = ConfigManager::new().expect("config manager");
    cm.save("default", &default_profile)
        .await
        .expect("write default profile");
    cm.set_current("default")
        .await
        .expect("set default profile current");

    let empty_connections = r#"{"downloadTotal":0,"uploadTotal":0,"connections":[]}"#;
    let edge_connections = r#"{"downloadTotal":1000,"uploadTotal":500,"connections":[{"id":"abc123456789","metadata":{"network":"tcp","type":"HTTP","sourceIP":"10.0.0.2","destinationIP":"4.4.4.4","sourcePort":"52345","destinationPort":"443","host":"","dnsMode":"normal","processPath":"/usr/bin/edge-app","specialProxy":""},"upload":300,"download":700,"start":"2024-01-01T00:00:00Z","chains":[],"rule":"MATCH","rulePayload":""}]}"#;

    let mock_empty = server
        .mock("GET", "/connections")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(empty_connections)
        .expect(5)
        .create_async()
        .await;
    let mock_edge = server
        .mock("GET", "/connections")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(edge_connections)
        .expect(2)
        .create_async()
        .await;

    run_cli_command(Commands::Connection {
        action: ConnectionAction::List {
            host: None,
            process: None,
        },
    })
    .await
    .expect("connection list empty");
    run_cli_command(Commands::Connection {
        action: ConnectionAction::FilterHost {
            host: "no-such-host".to_string(),
        },
    })
    .await
    .expect("connection filter host empty");
    run_cli_command(Commands::Connection {
        action: ConnectionAction::FilterProcess {
            process: "no-such-proc".to_string(),
        },
    })
    .await
    .expect("connection filter process empty");
    run_cli_command(Commands::Connection {
        action: ConnectionAction::CloseByHost {
            host: "no-such-host".to_string(),
            force: true,
        },
    })
    .await
    .expect("connection close by host empty");
    run_cli_command(Commands::Connection {
        action: ConnectionAction::CloseByProcess {
            process: "no-such-proc".to_string(),
            force: true,
        },
    })
    .await
    .expect("connection close by process empty");

    run_cli_command(Commands::Connection {
        action: ConnectionAction::List {
            host: None,
            process: None,
        },
    })
    .await
    .expect("connection list edge payload");
    run_cli_command(Commands::Connection {
        action: ConnectionAction::FilterProcess {
            process: "edge-app".to_string(),
        },
    })
    .await
    .expect("connection filter process edge payload");

    // Stream one snapshot then close to exercise stream rendering branches.
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind stream ws listener");
    let ws_addr = listener.local_addr().expect("stream ws addr");
    let stream_profile = format!("port: 7890\nexternal-controller: http://{}\n", ws_addr);
    cm.save("default", &stream_profile)
        .await
        .expect("update profile for stream");

    tokio::spawn(async move {
        use futures_util::SinkExt;
        let (stream, _) = listener.accept().await.expect("accept stream ws");
        let mut ws = accept_hdr_async(stream, |_req: &Request, resp: Response| Ok(resp))
            .await
            .expect("accept stream handshake");
        ws.send(Message::Text(
            r#"{"downloadTotal":3000,"uploadTotal":2000,"connections":[{"id":"stream-conn-1","metadata":{"network":"tcp","type":"HTTP","sourceIP":"10.0.0.2","destinationIP":"4.4.4.4","sourcePort":"52345","destinationPort":"443","host":"example.com","dnsMode":"normal","processPath":"/usr/bin/app","specialProxy":""},"upload":1024,"download":2048,"start":"2024-01-01T00:00:00Z","chains":[],"rule":"MATCH","rulePayload":""}]}"#
                .to_string()
                .into(),
        ))
        .await
        .expect("send stream snapshot");
        ws.send(Message::Close(None))
            .await
            .expect("close stream ws");
    });

    run_cli_command(Commands::Connection {
        action: ConnectionAction::Stream,
    })
    .await
    .expect("connection stream");

    mock_empty.assert_async().await;
    mock_edge.assert_async().await;

    if let Some(value) = old_home {
        env::set_var("MIHOMO_HOME", value);
    } else {
        env::remove_var("MIHOMO_HOME");
    }
}

#[cfg(unix)]
#[tokio::test]
async fn run_cli_command_covers_connection_confirmation_branches() {
    let _guard = env_lock().lock().await;

    let temp = tempdir().expect("create temp dir");
    let old_home = env::var("MIHOMO_HOME").ok();
    env::set_var("MIHOMO_HOME", temp.path());

    let mut server = Server::new_async().await;
    let controller = server.url();
    let default_profile = format!("port: 7890\nexternal-controller: {}\n", controller);
    let cm = ConfigManager::new().expect("config manager");
    cm.save("default", &default_profile)
        .await
        .expect("write default profile");
    cm.set_current("default")
        .await
        .expect("set default profile current");

    let connections_payload = common::mock_connections_payload();
    let mock_connections = server
        .mock("GET", "/connections")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(connections_payload)
        .expect(2)
        .create_async()
        .await;

    with_mocked_stdin("n\nn\nn\n", async {
        run_cli_command(Commands::Connection {
            action: ConnectionAction::Close {
                legacy_id: None,
                id: None,
                all: true,
                host: None,
                process: None,
                force: false,
            },
        })
        .await
        .expect("close all cancelled");
        run_cli_command(Commands::Connection {
            action: ConnectionAction::Close {
                legacy_id: None,
                id: None,
                all: false,
                host: Some("example".to_string()),
                process: None,
                force: false,
            },
        })
        .await
        .expect("close by host cancelled");
        run_cli_command(Commands::Connection {
            action: ConnectionAction::Close {
                legacy_id: None,
                id: None,
                all: false,
                host: None,
                process: Some("curl".to_string()),
                force: false,
            },
        })
        .await
        .expect("close by process cancelled");
    })
    .await;

    mock_connections.assert_async().await;

    if let Some(value) = old_home {
        env::set_var("MIHOMO_HOME", value);
    } else {
        env::remove_var("MIHOMO_HOME");
    }
}

#[tokio::test]
async fn run_cli_command_covers_config_and_proxy_empty_branches() {
    let _guard = env_lock().lock().await;

    let temp = tempdir().expect("create temp dir");
    let old_home = env::var("MIHOMO_HOME").ok();
    env::set_var("MIHOMO_HOME", temp.path());

    run_cli_command(Commands::Config {
        action: ConfigAction::List,
    })
    .await
    .expect("config list empty");
    run_cli_command(Commands::Config {
        action: ConfigAction::Path,
    })
    .await
    .expect("config path empty");

    // Invalid config.toml makes get_current() fail and triggers show fallback closure.
    tokio::fs::write(temp.path().join("config.toml"), "default = [")
        .await
        .expect("write invalid config.toml");
    assert!(run_cli_command(Commands::Config {
        action: ConfigAction::Show { profile: None },
    })
    .await
    .is_err());
    tokio::fs::remove_file(temp.path().join("config.toml"))
        .await
        .expect("remove invalid config.toml");

    let mut server = Server::new_async().await;
    let controller = server.url();
    let cm = ConfigManager::new().expect("config manager");
    cm.save(
        "default",
        &format!("port: 7890\nexternal-controller: {}\n", controller),
    )
    .await
    .expect("write default profile");
    cm.set_current("default")
        .await
        .expect("set default profile");
    let empty_proxies = r#"{"proxies":{}}"#;
    let mock_proxies = server
        .mock("GET", "/proxies")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(empty_proxies)
        .expect(3)
        .create_async()
        .await;

    run_cli_command(Commands::Proxy {
        action: ProxyAction::List,
    })
    .await
    .expect("proxy list empty");
    run_cli_command(Commands::Proxy {
        action: ProxyAction::Groups,
    })
    .await
    .expect("proxy groups empty");
    run_cli_command(Commands::Proxy {
        action: ProxyAction::Current,
    })
    .await
    .expect("proxy current empty");

    mock_proxies.assert_async().await;

    if let Some(value) = old_home {
        env::set_var("MIHOMO_HOME", value);
    } else {
        env::remove_var("MIHOMO_HOME");
    }
}

#[tokio::test]
async fn run_cli_command_covers_config_path_empty_state() {
    let _guard = env_lock().lock().await;

    let temp = tempdir().expect("create temp dir");
    let old_home = env::var("MIHOMO_HOME").ok();
    env::set_var("MIHOMO_HOME", temp.path());

    run_cli_command(Commands::Config {
        action: ConfigAction::List,
    })
    .await
    .expect("config list empty");
    run_cli_command(Commands::Config {
        action: ConfigAction::Current,
    })
    .await
    .expect("config current empty");
    run_cli_command(Commands::Config {
        action: ConfigAction::Path,
    })
    .await
    .expect("config path empty");

    if let Some(value) = old_home {
        env::set_var("MIHOMO_HOME", value);
    } else {
        env::remove_var("MIHOMO_HOME");
    }
}

#[tokio::test]
async fn run_cli_command_current_uses_special_character_configs_dir() {
    let _guard = env_lock().lock().await;

    let temp = tempdir().expect("create temp dir");
    let old_home = env::var("MIHOMO_HOME").ok();
    env::set_var("MIHOMO_HOME", temp.path());

    let cm = ConfigManager::new().expect("config manager");
    cm.set_configs_dir("iCloud Drive/代理配置 (测试) [v2] #1 & team")
        .await
        .expect("set special configs dir");
    cm.save(
        "default",
        "port: 7890\nexternal-controller: 127.0.0.1:9090\n",
    )
    .await
    .expect("write default profile");
    cm.set_current("default")
        .await
        .expect("set default current");

    run_cli_command(Commands::Config {
        action: ConfigAction::Current,
    })
    .await
    .expect("config current with special dir");
    run_cli_command(Commands::Config {
        action: ConfigAction::Path,
    })
    .await
    .expect("config path with special dir");

    if let Some(value) = old_home {
        env::set_var("MIHOMO_HOME", value);
    } else {
        env::remove_var("MIHOMO_HOME");
    }
}

#[tokio::test]
async fn run_cli_command_sets_and_unsets_configs_dir() {
    let _guard = env_lock().lock().await;

    let temp = tempdir().expect("create temp dir");
    let old_home = env::var("MIHOMO_HOME").ok();
    env::set_var("MIHOMO_HOME", temp.path());

    run_cli_command(Commands::Config {
        action: ConfigAction::Set {
            key: mihomo_rs::cli::ConfigKey::ConfigsDir,
            value: "iCloud Drive/Clash Configs".to_string(),
        },
    })
    .await
    .expect("config set configs-dir");

    let settings = tokio::fs::read_to_string(temp.path().join("config.toml"))
        .await
        .expect("read config.toml");
    assert!(settings.contains("configs_dir = \"iCloud Drive/Clash Configs\""));

    run_cli_command(Commands::Config {
        action: ConfigAction::Unset {
            key: mihomo_rs::cli::ConfigKey::ConfigsDir,
        },
    })
    .await
    .expect("config unset configs-dir");

    let settings = tokio::fs::read_to_string(temp.path().join("config.toml"))
        .await
        .expect("read config.toml after unset");
    assert!(!settings.contains("configs_dir"));

    if let Some(value) = old_home {
        env::set_var("MIHOMO_HOME", value);
    } else {
        env::remove_var("MIHOMO_HOME");
    }
}

#[tokio::test]
async fn run_cli_command_supports_namespaced_version_and_service_commands() {
    let _guard = env_lock().lock().await;

    let temp = tempdir().expect("create temp dir");
    let old_home = env::var("MIHOMO_HOME").ok();
    env::set_var("MIHOMO_HOME", temp.path());

    common::install_fake_version(temp.path(), "v1.2.3").await;

    let cm = ConfigManager::new().expect("config manager");
    cm.save(
        "default",
        "port: 7890\nexternal-controller: 127.0.0.1:9090\n",
    )
    .await
    .expect("write default profile");
    cm.set_current("default")
        .await
        .expect("set default profile current");

    run_cli_command(Commands::Version {
        action: VersionAction::Use {
            version: "v1.2.3".to_string(),
        },
    })
    .await
    .expect("version use");
    run_cli_command(Commands::Version {
        action: VersionAction::List,
    })
    .await
    .expect("version list");
    run_cli_command(Commands::Service {
        action: ServiceAction::Status,
    })
    .await
    .expect("service status");

    if let Some(value) = old_home {
        env::set_var("MIHOMO_HOME", value);
    } else {
        env::remove_var("MIHOMO_HOME");
    }
}
