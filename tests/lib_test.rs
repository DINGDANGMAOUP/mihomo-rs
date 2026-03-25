// Integration tests for top-level convenience functions

use mihomo_rs::Result;
use mockito::{Matcher, Server};
use std::sync::{Mutex, OnceLock};
use tempfile::tempdir;
use tokio::fs;
use {flate2::write::GzEncoder, flate2::Compression, std::io::Write};

fn env_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

fn expected_platform() -> &'static str {
    match std::env::consts::ARCH {
        "x86_64" => "amd64",
        "aarch64" => "arm64",
        "arm" => "armv7",
        _ => "amd64",
    }
}

fn expected_os_name() -> &'static str {
    match std::env::consts::OS {
        "linux" => "linux",
        "macos" => "darwin",
        "windows" => "windows",
        _ => "linux",
    }
}

// Note: These tests verify that the public API is exported correctly
// Actual functionality testing requires a running mihomo instance and network access

#[test]
fn test_result_type_alias() {
    // Test that Result type alias works correctly
    fn returns_result() -> Result<i32> {
        Ok(42)
    }

    assert_eq!(returns_result().unwrap(), 42);
}

#[test]
fn test_public_exports() {
    // Verify all main types are exported
    use mihomo_rs::{
        Channel, ConfigManager, MihomoClient, MihomoError, Profile, ProxyManager, Result,
        ServiceManager, ServiceStatus, VersionManager,
    };

    // Type existence checks
    let _: Option<Channel> = None;
    let _: Option<ConfigManager> = None;
    let _: Option<MihomoClient> = None;
    let _: Option<MihomoError> = None;
    let _: Option<ProxyManager> = None;
    let _: Option<ServiceManager> = None;
    let _: Option<ServiceStatus> = None;
    let _: Option<VersionManager> = None;
    let _: Option<Profile> = None;
    let _: Result<()> = Ok(());
}

#[test]
fn test_channel_enum() {
    use mihomo_rs::Channel;

    // Test Channel enum variants exist
    let _stable = Channel::Stable;
    let _beta = Channel::Beta;
    let _nightly = Channel::Nightly;
}

#[test]
fn test_service_status_enum() {
    use mihomo_rs::ServiceStatus;

    // Test ServiceStatus enum variants
    let running = ServiceStatus::Running(12345);
    let stopped = ServiceStatus::Stopped;

    match running {
        ServiceStatus::Running(pid) => assert_eq!(pid, 12345),
        ServiceStatus::Stopped => panic!("Should be running"),
    }

    match stopped {
        ServiceStatus::Stopped => {} // OK
        ServiceStatus::Running(_) => panic!("Should be stopped"),
    }
}

#[test]
fn test_error_type() {
    use mihomo_rs::MihomoError;

    // Test various error types can be created
    let config_err = MihomoError::Config("test".to_string());
    let service_err = MihomoError::Service("test".to_string());
    let version_err = MihomoError::Version("test".to_string());
    let proxy_err = MihomoError::Proxy("test".to_string());
    let not_found_err = MihomoError::NotFound("test".to_string());

    assert!(matches!(config_err, MihomoError::Config(_)));
    assert!(matches!(service_err, MihomoError::Service(_)));
    assert!(matches!(version_err, MihomoError::Version(_)));
    assert!(matches!(proxy_err, MihomoError::Proxy(_)));
    assert!(matches!(not_found_err, MihomoError::NotFound(_)));
}

#[tokio::test]
#[allow(clippy::await_holding_lock)]
async fn test_switch_proxy_convenience_function() {
    let _guard = env_lock().lock().expect("env lock");
    let temp = tempdir().expect("tempdir");
    let old = std::env::var("MIHOMO_HOME").ok();
    // SAFETY: env updates are serialized by a process-wide mutex in this test module.
    unsafe { std::env::set_var("MIHOMO_HOME", temp.path()) };

    let configs_dir = temp.path().join("configs");
    fs::create_dir_all(&configs_dir).await.unwrap();

    let mut server = Server::new_async().await;
    let switch_mock = server
        .mock("PUT", "/proxies/GLOBAL")
        .match_body(Matcher::Json(serde_json::json!({"name":"DIRECT"})))
        .with_status(204)
        .create_async()
        .await;

    let config = format!(
        "port: 7890\nsocks-port: 7891\nexternal-controller: {}\n",
        server.url()
    );
    fs::write(configs_dir.join("default.yaml"), config)
        .await
        .unwrap();

    let result = mihomo_rs::switch_proxy("GLOBAL", "DIRECT").await;
    switch_mock.assert_async().await;
    assert!(result.is_ok());

    if let Some(prev) = old {
        // SAFETY: env updates are serialized by a process-wide mutex in this test module.
        unsafe { std::env::set_var("MIHOMO_HOME", prev) };
    } else {
        // SAFETY: env updates are serialized by a process-wide mutex in this test module.
        unsafe { std::env::remove_var("MIHOMO_HOME") };
    }
}

#[tokio::test]
#[allow(clippy::await_holding_lock)]
async fn test_stop_service_convenience_function_error_path() {
    let _guard = env_lock().lock().expect("env lock");
    let temp = tempdir().expect("tempdir");
    let old = std::env::var("MIHOMO_HOME").ok();
    // SAFETY: env updates are serialized by a process-wide mutex in this test module.
    unsafe { std::env::set_var("MIHOMO_HOME", temp.path()) };

    let versions_dir = temp.path().join("versions").join("v1.0.0");
    fs::create_dir_all(&versions_dir).await.unwrap();
    let binary = if cfg!(windows) {
        versions_dir.join("mihomo.exe")
    } else {
        versions_dir.join("mihomo")
    };
    fs::write(&binary, b"fake-binary").await.unwrap();
    fs::write(
        temp.path().join("config.toml"),
        "[default]\nversion = \"v1.0.0\"\n",
    )
    .await
    .unwrap();

    let config_path = temp.path().join("configs").join("default.yaml");
    fs::create_dir_all(config_path.parent().unwrap())
        .await
        .unwrap();
    fs::write(&config_path, "port: 7890\n").await.unwrap();

    let result = mihomo_rs::stop_service(&config_path).await;
    assert!(result.is_err());

    if let Some(prev) = old {
        // SAFETY: env updates are serialized by a process-wide mutex in this test module.
        unsafe { std::env::set_var("MIHOMO_HOME", prev) };
    } else {
        // SAFETY: env updates are serialized by a process-wide mutex in this test module.
        unsafe { std::env::remove_var("MIHOMO_HOME") };
    }
}

#[tokio::test]
#[allow(clippy::await_holding_lock)]
async fn test_install_mihomo_some_version_already_installed() {
    let _guard = env_lock().lock().expect("env lock");
    let temp = tempdir().expect("tempdir");
    let old = std::env::var("MIHOMO_HOME").ok();
    // SAFETY: env updates are serialized by a process-wide mutex in this test module.
    unsafe { std::env::set_var("MIHOMO_HOME", temp.path()) };

    let version = "v1.2.3";
    let version_dir = temp.path().join("versions").join(version);
    fs::create_dir_all(&version_dir).await.unwrap();

    let result = mihomo_rs::install_mihomo(Some(version)).await;
    assert!(result.is_err());

    if let Some(prev) = old {
        // SAFETY: env updates are serialized by a process-wide mutex in this test module.
        unsafe { std::env::set_var("MIHOMO_HOME", prev) };
    } else {
        // SAFETY: env updates are serialized by a process-wide mutex in this test module.
        unsafe { std::env::remove_var("MIHOMO_HOME") };
    }
}

#[tokio::test]
#[cfg(not(target_os = "windows"))]
#[allow(clippy::await_holding_lock)]
async fn test_install_mihomo_some_version_success() {
    let _guard = env_lock().lock().expect("env lock");
    let temp = tempdir().expect("tempdir");
    let old_home = std::env::var("MIHOMO_HOME").ok();
    let old_dl = std::env::var("MIHOMO_DOWNLOAD_BASE_URL").ok();

    let version = "v2.3.4";
    let platform = expected_platform();
    let os_name = expected_os_name();
    let filename = format!("mihomo-{}-{}-{}.gz", os_name, platform, version);
    let path = format!(
        "/MetaCubeX/mihomo/releases/download/{}/{}",
        version, filename
    );

    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(b"mihomo-bin").unwrap();
    let gz = encoder.finish().unwrap();

    let mut server = Server::new_async().await;
    let dl_mock = server
        .mock("GET", path.as_str())
        .with_status(200)
        .with_body(gz)
        .create_async()
        .await;

    // SAFETY: env updates are serialized by a process-wide mutex in this test module.
    unsafe {
        std::env::set_var("MIHOMO_HOME", temp.path());
        std::env::set_var("MIHOMO_DOWNLOAD_BASE_URL", server.url());
    }

    let result = mihomo_rs::install_mihomo(Some(version)).await.unwrap();
    dl_mock.assert_async().await;
    assert_eq!(result, version.to_string());

    if let Some(prev) = old_home {
        // SAFETY: env updates are serialized by a process-wide mutex in this test module.
        unsafe { std::env::set_var("MIHOMO_HOME", prev) };
    } else {
        // SAFETY: env updates are serialized by a process-wide mutex in this test module.
        unsafe { std::env::remove_var("MIHOMO_HOME") };
    }
    if let Some(prev) = old_dl {
        // SAFETY: env updates are serialized by a process-wide mutex in this test module.
        unsafe { std::env::set_var("MIHOMO_DOWNLOAD_BASE_URL", prev) };
    } else {
        // SAFETY: env updates are serialized by a process-wide mutex in this test module.
        unsafe { std::env::remove_var("MIHOMO_DOWNLOAD_BASE_URL") };
    }
}

#[tokio::test]
#[cfg(not(target_os = "windows"))]
#[allow(clippy::await_holding_lock)]
async fn test_install_mihomo_none_uses_stable_channel() {
    let _guard = env_lock().lock().expect("env lock");
    let temp = tempdir().expect("tempdir");
    let old_home = std::env::var("MIHOMO_HOME").ok();
    let old_api = std::env::var("MIHOMO_API_BASE_URL").ok();
    let old_dl = std::env::var("MIHOMO_DOWNLOAD_BASE_URL").ok();

    let version = "v3.4.5";
    let platform = expected_platform();
    let os_name = expected_os_name();
    let filename = format!("mihomo-{}-{}-{}.gz", os_name, platform, version);
    let path = format!(
        "/MetaCubeX/mihomo/releases/download/{}/{}",
        version, filename
    );

    let mut api = Server::new_async().await;
    let latest_mock = api
        .mock("GET", "/repos/MetaCubeX/mihomo/releases/latest")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(format!(
            r#"{{"tag_name":"{}","published_at":"2026-03-25T12:00:00Z"}}"#,
            version
        ))
        .create_async()
        .await;

    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(b"mihomo-channel").unwrap();
    let gz = encoder.finish().unwrap();

    let mut dl = Server::new_async().await;
    let dl_mock = dl
        .mock("GET", path.as_str())
        .with_status(200)
        .with_body(gz)
        .create_async()
        .await;

    // SAFETY: env updates are serialized by a process-wide mutex in this test module.
    unsafe {
        std::env::set_var("MIHOMO_HOME", temp.path());
        std::env::set_var("MIHOMO_API_BASE_URL", api.url());
        std::env::set_var("MIHOMO_DOWNLOAD_BASE_URL", dl.url());
    }

    let result = mihomo_rs::install_mihomo(None).await.unwrap();
    latest_mock.assert_async().await;
    dl_mock.assert_async().await;
    assert_eq!(result, version.to_string());

    if let Some(prev) = old_home {
        // SAFETY: env updates are serialized by a process-wide mutex in this test module.
        unsafe { std::env::set_var("MIHOMO_HOME", prev) };
    } else {
        // SAFETY: env updates are serialized by a process-wide mutex in this test module.
        unsafe { std::env::remove_var("MIHOMO_HOME") };
    }
    if let Some(prev) = old_api {
        // SAFETY: env updates are serialized by a process-wide mutex in this test module.
        unsafe { std::env::set_var("MIHOMO_API_BASE_URL", prev) };
    } else {
        // SAFETY: env updates are serialized by a process-wide mutex in this test module.
        unsafe { std::env::remove_var("MIHOMO_API_BASE_URL") };
    }
    if let Some(prev) = old_dl {
        // SAFETY: env updates are serialized by a process-wide mutex in this test module.
        unsafe { std::env::set_var("MIHOMO_DOWNLOAD_BASE_URL", prev) };
    } else {
        // SAFETY: env updates are serialized by a process-wide mutex in this test module.
        unsafe { std::env::remove_var("MIHOMO_DOWNLOAD_BASE_URL") };
    }
}

#[tokio::test]
#[allow(clippy::await_holding_lock)]
async fn test_start_service_convenience_function_error_path() {
    let _guard = env_lock().lock().expect("env lock");
    let temp = tempdir().expect("tempdir");
    let old = std::env::var("MIHOMO_HOME").ok();
    // SAFETY: env updates are serialized by a process-wide mutex in this test module.
    unsafe { std::env::set_var("MIHOMO_HOME", temp.path()) };

    let version = "v9.9.9";
    let version_dir = temp.path().join("versions").join(version);
    fs::create_dir_all(&version_dir).await.unwrap();
    let binary = if cfg!(windows) {
        version_dir.join("mihomo.exe")
    } else {
        version_dir.join("mihomo")
    };
    fs::write(&binary, b"not-an-executable").await.unwrap();
    fs::write(
        temp.path().join("config.toml"),
        format!("[default]\nversion = \"{}\"\n", version),
    )
    .await
    .unwrap();

    let config_dir = temp.path().join("configs");
    fs::create_dir_all(&config_dir).await.unwrap();
    let config_path = config_dir.join("default.yaml");
    fs::write(
        &config_path,
        "port: 7890\nsocks-port: 7891\nexternal-controller: 127.0.0.1:9090\n",
    )
    .await
    .unwrap();

    let start = mihomo_rs::start_service(&config_path).await;
    assert!(start.is_err());

    if let Some(prev) = old {
        // SAFETY: env updates are serialized by a process-wide mutex in this test module.
        unsafe { std::env::set_var("MIHOMO_HOME", prev) };
    } else {
        // SAFETY: env updates are serialized by a process-wide mutex in this test module.
        unsafe { std::env::remove_var("MIHOMO_HOME") };
    }
}
