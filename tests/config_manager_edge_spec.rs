mod common;

use common::{setup_temp_home, temp_home_path};
use mihomo_rs::{ConfigManager, MihomoError};

#[tokio::test]
async fn not_found_paths_for_load_set_current_delete() {
    let temp = setup_temp_home();
    let home = temp_home_path(&temp);
    let cm = ConfigManager::with_home(home).expect("create manager");

    assert!(matches!(
        cm.load("missing").await.expect_err("load missing"),
        MihomoError::NotFound(_)
    ));
    assert!(matches!(
        cm.set_current("missing")
            .await
            .expect_err("set current missing"),
        MihomoError::NotFound(_)
    ));
    assert!(matches!(
        cm.delete_profile("missing")
            .await
            .expect_err("delete missing"),
        MihomoError::NotFound(_)
    ));
}

#[tokio::test]
async fn get_external_controller_normalizes_colon_and_http_https() {
    let temp = setup_temp_home();
    let home = temp_home_path(&temp);
    let cm = ConfigManager::with_home(home).expect("create manager");

    cm.save("colon", "port: 7890\nexternal-controller: :10090\n")
        .await
        .expect("save colon config");
    cm.set_current("colon").await.expect("set colon config");
    let colon = cm
        .get_external_controller()
        .await
        .expect("get colon controller");
    assert_eq!(colon, "http://127.0.0.1:10090");

    cm.save(
        "http",
        "port: 7890\nexternal-controller: http://example.com:18080\n",
    )
    .await
    .expect("save http config");
    cm.set_current("http").await.expect("set http config");
    let http = cm
        .get_external_controller()
        .await
        .expect("get http controller");
    assert_eq!(http, "http://example.com:18080");

    cm.save(
        "https",
        "port: 7890\nexternal-controller: https://example.com:18443\n",
    )
    .await
    .expect("save https config");
    cm.set_current("https").await.expect("set https config");
    let https = cm
        .get_external_controller()
        .await
        .expect("get https controller");
    assert_eq!(https, "https://example.com:18443");
}

#[tokio::test]
async fn ensure_external_controller_updates_local_http_without_port_and_occupied_https_port() {
    let temp = setup_temp_home();
    let home = temp_home_path(&temp);
    let cm = ConfigManager::with_home(home).expect("create manager");

    cm.save(
        "no-port",
        "port: 7890\nexternal-controller: http://localhost\n",
    )
    .await
    .expect("save no-port config");
    cm.set_current("no-port").await.expect("set no-port profile");

    let updated = cm
        .ensure_external_controller()
        .await
        .expect("ensure no-port controller");
    assert!(updated.starts_with("http://127.0.0.1:"));

    let listener = std::net::TcpListener::bind("127.0.0.1:12090")
        .expect("occupy local port for https controller");
    listener
        .set_nonblocking(true)
        .expect("set nonblocking listener");

    cm.save(
        "https-local",
        "port: 7890\nexternal-controller: https://127.0.0.1:12090\n",
    )
    .await
    .expect("save https local config");
    cm.set_current("https-local")
        .await
        .expect("set https local profile");

    let reassigned = cm
        .ensure_external_controller()
        .await
        .expect("ensure occupied https local controller");
    assert!(reassigned.starts_with("http://127.0.0.1:"));
    assert_ne!(reassigned, "https://127.0.0.1:12090");
}

#[tokio::test]
async fn ensure_external_controller_keeps_local_http_https_when_port_is_free() {
    let temp = setup_temp_home();
    let home = temp_home_path(&temp);
    let cm = ConfigManager::with_home(home).expect("create manager");

    cm.save(
        "http-free",
        "port: 7890\nexternal-controller: http://127.0.0.1:19191\n",
    )
    .await
    .expect("save http-free config");
    cm.set_current("http-free")
        .await
        .expect("set http-free profile");
    let http = cm
        .ensure_external_controller()
        .await
        .expect("ensure free http local");
    assert_eq!(http, "http://127.0.0.1:19191");

    cm.save(
        "https-free",
        "port: 7890\nexternal-controller: https://localhost:19443\n",
    )
    .await
    .expect("save https-free config");
    cm.set_current("https-free")
        .await
        .expect("set https-free profile");
    let https = cm
        .ensure_external_controller()
        .await
        .expect("ensure free https local");
    assert_eq!(https, "https://localhost:19443");
}

#[tokio::test]
async fn list_profiles_ignores_non_yaml_and_get_current_falls_back_to_default() {
    let temp = setup_temp_home();
    let home = temp_home_path(&temp);
    let cm = ConfigManager::with_home(home.clone()).expect("create manager");

    tokio::fs::create_dir_all(home.join("configs"))
        .await
        .expect("create config dir");
    tokio::fs::write(home.join("configs/readme.txt"), "not a profile")
        .await
        .expect("write txt file");
    cm.save("alpha", "port: 7890\nexternal-controller: 127.0.0.1:9090\n")
        .await
        .expect("save yaml profile");

    let profiles = cm.list_profiles().await.expect("list profiles");
    assert_eq!(profiles.len(), 1);
    assert_eq!(profiles[0].name, "alpha");

    tokio::fs::write(home.join("config.toml"), "[other]\nvalue = 1\n")
        .await
        .expect("write config without default profile");
    let current = cm.get_current().await.expect("get current fallback");
    assert_eq!(current, "default");
}

#[tokio::test]
async fn ensure_external_controller_updates_invalid_http_url() {
    let temp = setup_temp_home();
    let home = temp_home_path(&temp);
    let cm = ConfigManager::with_home(home).expect("create manager");

    cm.save(
        "invalid-http",
        "port: 7890\nexternal-controller: \"http://:\"\n",
    )
    .await
    .expect("save invalid http url");
    cm.set_current("invalid-http")
        .await
        .expect("set invalid-http profile");

    let updated = cm
        .ensure_external_controller()
        .await
        .expect("ensure invalid http url");
    assert!(updated.starts_with("http://127.0.0.1:"));
}

#[tokio::test]
async fn ensure_external_controller_keeps_remote_http_https_urls() {
    let temp = setup_temp_home();
    let home = temp_home_path(&temp);
    let cm = ConfigManager::with_home(home).expect("create manager");

    cm.save(
        "remote-http",
        "port: 7890\nexternal-controller: http://example.com\n",
    )
    .await
    .expect("save remote http profile");
    cm.set_current("remote-http")
        .await
        .expect("set remote http profile");
    let http = cm
        .ensure_external_controller()
        .await
        .expect("ensure remote http");
    assert_eq!(http, "http://example.com");

    cm.save(
        "remote-https",
        "port: 7890\nexternal-controller: https://example.com\n",
    )
    .await
    .expect("save remote https profile");
    cm.set_current("remote-https")
        .await
        .expect("set remote https profile");
    let https = cm
        .ensure_external_controller()
        .await
        .expect("ensure remote https");
    assert_eq!(https, "https://example.com");
}

#[tokio::test]
async fn ensure_external_controller_handles_plain_colon_address() {
    let temp = setup_temp_home();
    let home = temp_home_path(&temp);
    let cm = ConfigManager::with_home(home).expect("create manager");

    cm.save("colon", "port: 7890\nexternal-controller: :10090\n")
        .await
        .expect("save colon profile");
    cm.set_current("colon").await.expect("set colon profile");

    let controller = cm
        .ensure_external_controller()
        .await
        .expect("ensure colon controller");
    assert_eq!(controller, "http://127.0.0.1:10090");
}
