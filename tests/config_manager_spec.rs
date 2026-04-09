mod common;

use common::{config_without_controller, default_test_config, setup_temp_home, temp_home_path};
use mihomo_rs::{ConfigManager, MihomoError};
use tokio::fs;

fn external_controller_of(content: &str) -> Option<String> {
    let value: serde_yaml::Value = serde_yaml::from_str(content).ok()?;
    value
        .get("external-controller")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

#[tokio::test]
async fn profile_lifecycle_save_load_list_set_current() {
    let temp = setup_temp_home();
    let home = temp_home_path(&temp);
    let manager = ConfigManager::with_home(home).expect("create config manager");

    manager
        .save("alpha", &default_test_config())
        .await
        .expect("save alpha");
    manager
        .save("beta", &default_test_config())
        .await
        .expect("save beta");
    manager.set_current("beta").await.expect("set current");

    let loaded = manager.load("beta").await.expect("load beta");
    let current = manager.get_current().await.expect("get current");
    let profiles = manager.list_profiles().await.expect("list profiles");

    assert!(external_controller_of(&loaded).is_some());
    assert_eq!(current, "beta");
    assert_eq!(profiles.len(), 2);
    assert_eq!(profiles[0].name, "alpha");
    assert_eq!(profiles[1].name, "beta");
    assert!(profiles.iter().any(|p| p.name == "beta" && p.active));
}

#[tokio::test]
async fn delete_profile_rejects_active_profile() {
    let temp = setup_temp_home();
    let home = temp_home_path(&temp);
    let manager = ConfigManager::with_home(home).expect("create config manager");

    manager
        .save("active", &default_test_config())
        .await
        .expect("save active");
    manager
        .set_current("active")
        .await
        .expect("set current profile");

    let err = manager
        .delete_profile("active")
        .await
        .expect_err("active profile should not be deletable");
    match err {
        MihomoError::Config(msg) => assert_eq!(msg, "Cannot delete the active profile"),
        other => panic!("expected config error, got: {}", other),
    }
}

#[tokio::test]
async fn ensure_default_config_creates_missing_profile_file() {
    let temp = setup_temp_home();
    let home = temp_home_path(&temp);
    let manager = ConfigManager::with_home(home).expect("create config manager");

    manager
        .ensure_default_config()
        .await
        .expect("ensure default config");

    let profiles = manager.list_profiles().await.expect("list profiles");
    assert_eq!(profiles.len(), 1);
    assert_eq!(profiles[0].name, "default");

    let content = manager.load("default").await.expect("load default config");
    assert!(external_controller_of(&content).is_some());
}

#[tokio::test]
async fn external_controller_normalization_and_preserve_unix_socket() {
    let temp = setup_temp_home();
    let home = temp_home_path(&temp);
    let manager = ConfigManager::with_home(home).expect("create config manager");

    manager
        .save(
            "tcp",
            r#"port: 7890
external-controller: 127.0.0.1:9090
"#,
        )
        .await
        .expect("save tcp profile");
    manager.set_current("tcp").await.expect("set current tcp");
    let tcp_controller = manager
        .get_external_controller()
        .await
        .expect("read tcp controller");
    assert_eq!(tcp_controller, "http://127.0.0.1:9090");

    manager
        .save(
            "unix",
            r#"port: 7890
external-controller: /var/run/mihomo.sock
"#,
        )
        .await
        .expect("save unix profile");
    manager.set_current("unix").await.expect("set current unix");

    let unix_controller = manager
        .ensure_external_controller()
        .await
        .expect("ensure unix controller");
    assert_eq!(unix_controller, "/var/run/mihomo.sock");

    let unix_config = manager.load("unix").await.expect("load unix config");
    assert_eq!(
        external_controller_of(&unix_config).as_deref(),
        Some("/var/run/mihomo.sock")
    );
}

#[tokio::test]
async fn ensure_external_controller_adds_missing_value() {
    let temp = setup_temp_home();
    let home = temp_home_path(&temp);
    let manager = ConfigManager::with_home(home).expect("create config manager");

    manager
        .save("no-controller", &config_without_controller())
        .await
        .expect("save profile without controller");
    manager
        .set_current("no-controller")
        .await
        .expect("set current");

    let url = manager
        .ensure_external_controller()
        .await
        .expect("ensure controller");
    assert!(url.starts_with("http://127.0.0.1:"));

    let updated = manager
        .load("no-controller")
        .await
        .expect("load updated profile");
    assert!(external_controller_of(&updated).is_some());
}

#[tokio::test]
async fn invalid_yaml_and_invalid_settings_file_return_errors() {
    let temp = setup_temp_home();
    let home = temp_home_path(&temp);
    let manager = ConfigManager::with_home(home.clone()).expect("create config manager");

    let save_err = manager
        .save("broken", "invalid: yaml: [")
        .await
        .expect_err("invalid yaml should fail");
    assert!(matches!(save_err, MihomoError::Yaml(_)));

    manager
        .save("good", &default_test_config())
        .await
        .expect("save valid profile");
    fs::write(home.join("config.toml"), "[default")
        .await
        .expect("write invalid settings file");

    let set_err = manager
        .set_current("good")
        .await
        .expect_err("invalid toml should fail");
    assert!(matches!(set_err, MihomoError::Config(_)));
}

#[tokio::test]
async fn get_current_path_tracks_selected_profile_file() {
    let temp = setup_temp_home();
    let home = temp_home_path(&temp);
    let manager = ConfigManager::with_home(home.clone()).expect("create config manager");

    manager
        .save("alpha", &default_test_config())
        .await
        .expect("save alpha");
    manager
        .set_current("alpha")
        .await
        .expect("set current profile");

    let current_path = manager.get_current_path().await.expect("get current path");
    assert_eq!(current_path, home.join("configs/alpha.yaml"));
}
