mod common;

use common::{setup_temp_home, temp_home_path};
use mihomo_rs::ConfigManager;
use tokio::fs;

fn external_controller_of(content: &str) -> Option<String> {
    let value: serde_yaml::Value = serde_yaml::from_str(content).ok()?;
    value
        .get("external-controller")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

#[tokio::test]
async fn ensure_external_controller_keeps_remote_host_port() {
    let temp = setup_temp_home();
    let home = temp_home_path(&temp);
    let cm = ConfigManager::with_home(home).expect("create manager");

    cm.save(
        "remote",
        "port: 7890\nexternal-controller: example.com:19090\n",
    )
    .await
    .expect("save remote profile");
    cm.set_current("remote").await.expect("set remote profile");

    let controller = cm
        .ensure_external_controller()
        .await
        .expect("ensure remote controller");
    assert_eq!(controller, "http://example.com:19090");

    let content = cm.load("remote").await.expect("load profile");
    assert_eq!(
        external_controller_of(&content).as_deref(),
        Some("example.com:19090")
    );
}

#[tokio::test]
async fn ensure_external_controller_updates_invalid_format() {
    let temp = setup_temp_home();
    let home = temp_home_path(&temp);
    let cm = ConfigManager::with_home(home).expect("create manager");

    cm.save(
        "invalid",
        "port: 7890\nexternal-controller: not-a-valid-address\n",
    )
    .await
    .expect("save invalid profile");
    cm.set_current("invalid").await.expect("set invalid profile");

    let controller = cm
        .ensure_external_controller()
        .await
        .expect("ensure controller");

    assert!(controller.starts_with("http://127.0.0.1:"));
    let content = cm.load("invalid").await.expect("load profile");
    let controller_in_file = external_controller_of(&content).expect("controller exists");
    assert!(controller_in_file.starts_with("127.0.0.1:"));
}

#[tokio::test]
async fn ensure_external_controller_reassigns_occupied_local_port() {
    let listener = std::net::TcpListener::bind("127.0.0.1:9090")
        .expect("bind local port to simulate occupation");
    listener
        .set_nonblocking(true)
        .expect("set listener nonblocking");

    let temp = setup_temp_home();
    let home = temp_home_path(&temp);
    let cm = ConfigManager::with_home(home.clone()).expect("create manager");

    cm.save(
        "local",
        "port: 7890\nexternal-controller: 127.0.0.1:9090\n",
    )
    .await
    .expect("save local profile");
    cm.set_current("local").await.expect("set local profile");

    let controller = cm
        .ensure_external_controller()
        .await
        .expect("ensure occupied local port");
    assert!(controller.starts_with("http://127.0.0.1:"));
    assert_ne!(controller, "http://127.0.0.1:9090");

    let content = fs::read_to_string(home.join("configs/local.yaml"))
        .await
        .expect("read updated yaml");
    let controller_in_file = external_controller_of(&content).expect("controller exists");
    assert!(controller_in_file.starts_with("127.0.0.1:"));
    assert_ne!(controller_in_file, "127.0.0.1:9090");
}
