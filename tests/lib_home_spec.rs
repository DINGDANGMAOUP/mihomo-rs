use mihomo_rs::{
    install_mihomo, start_service, stop_service, switch_proxy, ConfigManager, MihomoError,
    VersionManager,
};
use std::env;
use std::sync::OnceLock;
use tempfile::tempdir;
use tokio::fs;
use tokio::sync::Mutex;

fn env_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

#[tokio::test]
async fn top_level_entrypoints_and_home_resolution_are_exercised() {
    let _guard = env_lock().lock().await;

    let temp = tempdir().expect("create temp dir");
    let temp_home = temp.path().to_path_buf();

    let old_home = env::var("MIHOMO_HOME").ok();
    env::set_var("MIHOMO_HOME", &temp_home);

    let resolved = mihomo_rs::core::get_home_dir().expect("resolve home from env");
    assert_eq!(resolved, temp_home);

    env::remove_var("MIHOMO_HOME");
    let default_home = mihomo_rs::core::get_home_dir().expect("resolve default home");
    assert!(default_home.ends_with(".config/mihomo-rs"));

    env::set_var("MIHOMO_HOME", &temp_home);

    let config_path = temp.path().join("missing-config.yaml");

    assert!(matches!(
        install_mihomo(Some("v0.0.0"))
            .await
            .expect_err("install should fail"),
        MihomoError::Version(_) | MihomoError::Http(_)
    ));
    assert!(matches!(
        start_service(&config_path)
            .await
            .expect_err("start service should fail"),
        MihomoError::NotFound(_)
    ));
    assert!(matches!(
        stop_service(&config_path)
            .await
            .expect_err("stop service should fail"),
        MihomoError::NotFound(_)
    ));
    assert!(matches!(
        switch_proxy("GLOBAL", "DIRECT")
            .await
            .expect_err("switch proxy should fail"),
        MihomoError::NotFound(_)
    ));

    if let Some(value) = old_home {
        env::set_var("MIHOMO_HOME", value);
    } else {
        env::remove_var("MIHOMO_HOME");
    }
}

#[tokio::test]
async fn top_level_switch_proxy_reaches_client_creation_error_path() {
    let _guard = env_lock().lock().await;

    let temp = tempdir().expect("create temp dir");
    let home = temp.path().to_path_buf();
    let old_home = env::var("MIHOMO_HOME").ok();
    env::set_var("MIHOMO_HOME", &home);

    fs::create_dir_all(home.join("configs"))
        .await
        .expect("create config dir");
    fs::write(
        home.join("configs/default.yaml"),
        "port: 7890\nexternal-controller: \"://invalid\"\n",
    )
    .await
    .expect("write default profile");

    let err = switch_proxy("GLOBAL", "DIRECT")
        .await
        .expect_err("invalid controller url should fail");
    assert!(matches!(
        err,
        MihomoError::Config(_) | MihomoError::UrlParse(_) | MihomoError::Http(_)
    ));

    if let Some(value) = old_home {
        env::set_var("MIHOMO_HOME", value);
    } else {
        env::remove_var("MIHOMO_HOME");
    }
}

#[tokio::test]
async fn manager_new_entrypoints_use_mihomo_home_env() {
    let _guard = env_lock().lock().await;

    let temp = tempdir().expect("create temp dir");
    let home = temp.path().to_path_buf();
    let old_home = env::var("MIHOMO_HOME").ok();
    env::set_var("MIHOMO_HOME", &home);

    let _cm = ConfigManager::new().expect("config manager new");
    let _vm = VersionManager::new().expect("version manager new");

    if let Some(value) = old_home {
        env::set_var("MIHOMO_HOME", value);
    } else {
        env::remove_var("MIHOMO_HOME");
    }
}
