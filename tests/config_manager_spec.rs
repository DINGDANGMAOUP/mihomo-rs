mod common;

use common::{config_without_controller, default_test_config, setup_temp_home, temp_home_path};
use mihomo_rs::{ConfigManager, MihomoError};
use std::sync::OnceLock;
use tokio::fs;
use tokio::sync::Mutex;

fn env_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

fn external_controller_of(content: &str) -> Option<String> {
    let value: serde_yaml::Value = serde_yaml::from_str(content).ok()?;
    value
        .get("external-controller")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

#[tokio::test]
async fn profile_lifecycle_save_load_list_set_current() {
    let _guard = env_lock().lock().await;

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
    let _guard = env_lock().lock().await;

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
        MihomoError::Config(msg) => assert_eq!(msg.as_str(), "Cannot delete the active profile"),
        other => panic!("expected config error, got: {}", other),
    }
}

#[tokio::test]
async fn ensure_default_config_creates_missing_profile_file() {
    let _guard = env_lock().lock().await;

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
    let _guard = env_lock().lock().await;

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
    let _guard = env_lock().lock().await;

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
    let _guard = env_lock().lock().await;

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
    let _guard = env_lock().lock().await;

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

#[tokio::test]
async fn custom_configs_dir_in_settings_is_used_for_profile_io() {
    let _guard = env_lock().lock().await;

    let temp = setup_temp_home();
    let home = temp_home_path(&temp);
    let manager = ConfigManager::with_home(home.clone()).expect("create config manager");

    fs::write(
        home.join("config.toml"),
        "[paths]\nconfigs_dir = \"icloud/configs\"\n",
    )
    .await
    .expect("write custom configs dir");

    manager
        .save("cloud", &default_test_config())
        .await
        .expect("save cloud profile");
    manager
        .set_current("cloud")
        .await
        .expect("set current cloud profile");

    let profiles = manager.list_profiles().await.expect("list profiles");
    assert_eq!(profiles.len(), 1);
    assert_eq!(profiles[0].path, home.join("icloud/configs/cloud.yaml"));

    let current_path = manager.get_current_path().await.expect("get current path");
    assert_eq!(current_path, home.join("icloud/configs/cloud.yaml"));
    assert!(home.join("icloud/configs/cloud.yaml").exists());
}

#[tokio::test]
async fn set_and_unset_configs_dir_updates_settings_and_preserves_default_profile() {
    let _guard = env_lock().lock().await;

    let temp = setup_temp_home();
    let home = temp_home_path(&temp);
    let manager = ConfigManager::with_home(home.clone()).expect("create config manager");

    fs::write(home.join("config.toml"), "[default]\nprofile = \"work\"\n")
        .await
        .expect("write config with default profile");

    let resolved = manager
        .set_configs_dir("~/Library/Mobile Documents/mihomo-rs/configs")
        .await
        .expect("set configs dir");
    assert!(resolved.ends_with("Library/Mobile Documents/mihomo-rs/configs"));

    let content = fs::read_to_string(home.join("config.toml"))
        .await
        .expect("read config");
    let config: toml::Value = toml::from_str(&content).expect("parse config");
    assert_eq!(
        config
            .get("default")
            .and_then(|v| v.get("profile"))
            .and_then(|v| v.as_str()),
        Some("work")
    );
    assert_eq!(
        config
            .get("paths")
            .and_then(|v| v.get("configs_dir"))
            .and_then(|v| v.as_str()),
        Some("~/Library/Mobile Documents/mihomo-rs/configs")
    );

    let unset_resolved = manager
        .unset_configs_dir()
        .await
        .expect("unset configs dir");
    assert_eq!(unset_resolved, home.join("configs"));

    let content = fs::read_to_string(home.join("config.toml"))
        .await
        .expect("read config after unset");
    let config: toml::Value = toml::from_str(&content).expect("parse config after unset");
    assert_eq!(
        config
            .get("default")
            .and_then(|v| v.get("profile"))
            .and_then(|v| v.as_str()),
        Some("work")
    );
    assert!(config
        .get("paths")
        .and_then(|v| v.get("configs_dir"))
        .is_none());
}

#[tokio::test]
async fn set_and_unset_configs_dir_return_stored_paths_even_when_env_override_exists() {
    let _guard = env_lock().lock().await;

    let temp = setup_temp_home();
    let home = temp_home_path(&temp);
    let manager = ConfigManager::with_home(home.clone()).expect("create config manager");

    let old_value = std::env::var("MIHOMO_CONFIGS_DIR").ok();
    std::env::set_var("MIHOMO_CONFIGS_DIR", home.join("env-override"));

    let resolved = manager
        .set_configs_dir("icloud/configs")
        .await
        .expect("set configs dir");
    assert_eq!(resolved, home.join("icloud/configs"));

    let unset_resolved = manager
        .unset_configs_dir()
        .await
        .expect("unset configs dir");
    assert_eq!(unset_resolved, home.join("configs"));

    if let Some(value) = old_value {
        std::env::set_var("MIHOMO_CONFIGS_DIR", value);
    } else {
        std::env::remove_var("MIHOMO_CONFIGS_DIR");
    }
}

#[tokio::test]
async fn set_configs_dir_rejects_empty_path() {
    let _guard = env_lock().lock().await;

    let temp = setup_temp_home();
    let home = temp_home_path(&temp);
    let manager = ConfigManager::with_home(home).expect("create config manager");

    let err = manager
        .set_configs_dir("   ")
        .await
        .expect_err("empty path should fail");
    assert!(matches!(err, MihomoError::Config(_)));
}

#[tokio::test]
async fn special_character_configs_dir_roundtrips_and_updates_current_path() {
    let _guard = env_lock().lock().await;

    let temp = setup_temp_home();
    let home = temp_home_path(&temp);
    let manager = ConfigManager::with_home(home.clone()).expect("create config manager");

    let special_path = "iCloud Drive/代理配置 (测试) [v2] #1 & team";
    let resolved = manager
        .set_configs_dir(special_path)
        .await
        .expect("set special configs dir");
    assert_eq!(resolved, home.join(special_path));

    manager
        .save("alpha", &default_test_config())
        .await
        .expect("save alpha in special dir");
    manager
        .set_current("alpha")
        .await
        .expect("set alpha current");

    let current_path = manager.get_current_path().await.expect("get current path");
    assert_eq!(current_path, home.join(special_path).join("alpha.yaml"));

    let content = fs::read_to_string(home.join("config.toml"))
        .await
        .expect("read config with special path");
    let config: toml::Value = toml::from_str(&content).expect("parse config with special path");
    assert_eq!(
        config
            .get("paths")
            .and_then(|v| v.get("configs_dir"))
            .and_then(|v| v.as_str()),
        Some(special_path)
    );
}
