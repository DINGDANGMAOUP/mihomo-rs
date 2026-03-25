mod common;

use common::{create_test_config, get_temp_home_path, setup_temp_home};
use mihomo_rs::{ConfigManager, Result};
use std::sync::{Mutex, OnceLock};
use tokio::fs;

fn env_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

#[tokio::test]
async fn test_config_manager_new() -> Result<()> {
    let temp_dir = setup_temp_home();
    let home = get_temp_home_path(&temp_dir);

    let cm = ConfigManager::with_home(home)?;

    // Verify the manager was created successfully
    assert!(cm.list_profiles().await.is_ok());

    Ok(())
}

#[tokio::test]
async fn test_ensure_default_config() -> Result<()> {
    let temp_dir = setup_temp_home();
    let home = get_temp_home_path(&temp_dir);

    let cm = ConfigManager::with_home(home)?;

    // Ensure default config
    cm.ensure_default_config().await?;

    // Verify default config was created
    let profiles = cm.list_profiles().await?;
    assert!(profiles.iter().any(|p| p.name == "default"));

    Ok(())
}

#[tokio::test]
async fn test_save_and_load_profile() -> Result<()> {
    let temp_dir = setup_temp_home();
    let home = get_temp_home_path(&temp_dir);

    let cm = ConfigManager::with_home(home)?;

    let profile_name = "test-profile";
    let config_content = create_test_config();

    // Save profile
    cm.save(profile_name, &config_content).await?;

    // Load profile
    let loaded = cm.load(profile_name).await?;
    assert_eq!(loaded.trim(), config_content.trim());

    Ok(())
}

#[tokio::test]
async fn test_list_profiles() -> Result<()> {
    let temp_dir = setup_temp_home();
    let home = get_temp_home_path(&temp_dir);

    let cm = ConfigManager::with_home(home)?;

    // Create multiple profiles
    cm.save("profile1", &create_test_config()).await?;
    cm.save("profile2", &create_test_config()).await?;

    let profiles = cm.list_profiles().await?;
    assert!(profiles.len() >= 2);
    assert!(profiles.iter().any(|p| p.name == "profile1"));
    assert!(profiles.iter().any(|p| p.name == "profile2"));

    Ok(())
}

#[tokio::test]
async fn test_set_current_profile() -> Result<()> {
    let temp_dir = setup_temp_home();
    let home = get_temp_home_path(&temp_dir);

    let cm = ConfigManager::with_home(home)?;

    // Create and set profile
    let profile_name = "my-profile";
    cm.save(profile_name, &create_test_config()).await?;
    cm.set_current(profile_name).await?;

    // Verify current profile
    let current = cm.get_current().await?;
    assert_eq!(current, profile_name);

    Ok(())
}

#[tokio::test]
async fn test_delete_profile() -> Result<()> {
    let temp_dir = setup_temp_home();
    let home = get_temp_home_path(&temp_dir);

    let cm = ConfigManager::with_home(home)?;

    let profile_name = "temp-profile";
    cm.save(profile_name, &create_test_config()).await?;

    // Delete profile
    cm.delete_profile(profile_name).await?;

    // Verify profile is deleted
    let profiles = cm.list_profiles().await?;
    assert!(!profiles.iter().any(|p| p.name == profile_name));

    Ok(())
}

#[tokio::test]
async fn test_invalid_yaml_validation() {
    let temp_dir = setup_temp_home();
    let home = get_temp_home_path(&temp_dir);

    let cm = ConfigManager::with_home(home).unwrap();

    let invalid_yaml = "invalid: yaml: content: [";

    // Should fail to save invalid YAML
    let result = cm.save("invalid", invalid_yaml).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_get_current_path() -> Result<()> {
    let temp_dir = setup_temp_home();
    let home = get_temp_home_path(&temp_dir);

    let cm = ConfigManager::with_home(home)?;

    let profile_name = "test";
    cm.save(profile_name, &create_test_config()).await?;
    cm.set_current(profile_name).await?;

    let path = cm.get_current_path().await?;
    assert!(path.to_string_lossy().contains(profile_name));

    Ok(())
}

#[tokio::test]
async fn test_get_external_controller_unix_socket() -> Result<()> {
    let temp_dir = setup_temp_home();
    let home = get_temp_home_path(&temp_dir);

    let cm = ConfigManager::with_home(home)?;

    // Test Unix socket with absolute path
    let config_unix = r#"
port: 7890
socks-port: 7891
external-controller: /var/run/mihomo.sock
"#;
    cm.save("unix-test", config_unix).await?;
    cm.set_current("unix-test").await?;

    let controller = cm.get_external_controller().await?;
    assert_eq!(controller, "/var/run/mihomo.sock");

    // Test Unix socket with URI scheme
    let config_unix_uri = r#"
port: 7890
socks-port: 7891
external-controller: unix:///var/run/mihomo.sock
"#;
    cm.save("unix-uri-test", config_unix_uri).await?;
    cm.set_current("unix-uri-test").await?;

    let controller_uri = cm.get_external_controller().await?;
    assert_eq!(controller_uri, "unix:///var/run/mihomo.sock");

    // Test TCP (existing behavior)
    let config_tcp = r#"
port: 7890
socks-port: 7891
external-controller: 127.0.0.1:9090
"#;
    cm.save("tcp-test", config_tcp).await?;
    cm.set_current("tcp-test").await?;

    let controller_tcp = cm.get_external_controller().await?;
    assert_eq!(controller_tcp, "http://127.0.0.1:9090");

    Ok(())
}

#[tokio::test]
async fn test_delete_active_profile_should_fail() -> Result<()> {
    let temp_dir = setup_temp_home();
    let home = get_temp_home_path(&temp_dir);
    let cm = ConfigManager::with_home(home)?;

    cm.save("active", &create_test_config()).await?;
    cm.set_current("active").await?;

    let result = cm.delete_profile("active").await;
    assert!(result.is_err());

    Ok(())
}

#[tokio::test]
async fn test_set_current_nonexistent_profile_should_fail() -> Result<()> {
    let temp_dir = setup_temp_home();
    let home = get_temp_home_path(&temp_dir);
    let cm = ConfigManager::with_home(home)?;

    let result = cm.set_current("not-exists").await;
    assert!(result.is_err());

    Ok(())
}

#[tokio::test]
async fn test_get_current_with_invalid_toml_should_fail() -> Result<()> {
    let temp_dir = setup_temp_home();
    let home = get_temp_home_path(&temp_dir);
    let cm = ConfigManager::with_home(home.clone())?;

    fs::write(home.join("config.toml"), "invalid = [").await?;

    let result = cm.get_current().await;
    assert!(result.is_err());

    Ok(())
}

#[tokio::test]
async fn test_ensure_external_controller_adds_missing_field() -> Result<()> {
    let temp_dir = setup_temp_home();
    let home = get_temp_home_path(&temp_dir);
    let cm = ConfigManager::with_home(home)?;

    let config_without_controller = r#"
port: 7890
socks-port: 7891
mode: rule
"#;
    cm.save("no-controller", config_without_controller).await?;
    cm.set_current("no-controller").await?;

    let controller = cm.ensure_external_controller().await?;
    assert!(controller.starts_with("http://127.0.0.1:"));

    let updated = cm.load("no-controller").await?;
    assert!(updated.contains("external-controller: 127.0.0.1:"));

    Ok(())
}

#[tokio::test]
#[allow(clippy::await_holding_lock)]
async fn test_config_manager_new_uses_mihomo_home_env() -> Result<()> {
    let _guard = env_lock().lock().expect("env lock");
    let temp_dir = setup_temp_home();
    let home = get_temp_home_path(&temp_dir);
    let old = std::env::var("MIHOMO_HOME").ok();
    // SAFETY: env updates are serialized by a process-wide mutex in this test module.
    unsafe { std::env::set_var("MIHOMO_HOME", &home) };

    let cm = ConfigManager::new()?;
    cm.save("env-profile", &create_test_config()).await?;
    assert!(cm.load("env-profile").await.is_ok());

    if let Some(prev) = old {
        // SAFETY: env updates are serialized by a process-wide mutex in this test module.
        unsafe { std::env::set_var("MIHOMO_HOME", prev) };
    } else {
        // SAFETY: env updates are serialized by a process-wide mutex in this test module.
        unsafe { std::env::remove_var("MIHOMO_HOME") };
    }

    Ok(())
}

#[tokio::test]
async fn test_set_current_with_invalid_existing_toml_fallbacks_to_new_table() -> Result<()> {
    let temp_dir = setup_temp_home();
    let home = get_temp_home_path(&temp_dir);
    let cm = ConfigManager::with_home(home.clone())?;

    cm.save("fallback", &create_test_config()).await?;
    fs::write(home.join("config.toml"), "invalid = [").await?;

    cm.set_current("fallback").await?;
    assert_eq!(cm.get_current().await?, "fallback");

    Ok(())
}

#[tokio::test]
async fn test_get_external_controller_colon_and_https_formats() -> Result<()> {
    let temp_dir = setup_temp_home();
    let home = get_temp_home_path(&temp_dir);
    let cm = ConfigManager::with_home(home)?;

    let cfg_colon = r#"
port: 7890
socks-port: 7891
external-controller: :19090
"#;
    cm.save("colon", cfg_colon).await?;
    cm.set_current("colon").await?;
    assert_eq!(
        cm.get_external_controller().await?,
        "http://127.0.0.1:19090".to_string()
    );

    let cfg_https = r#"
port: 7890
socks-port: 7891
external-controller: https://127.0.0.1:19443
"#;
    cm.save("https", cfg_https).await?;
    cm.set_current("https").await?;
    assert_eq!(
        cm.get_external_controller().await?,
        "https://127.0.0.1:19443".to_string()
    );

    Ok(())
}

#[tokio::test]
async fn test_ensure_external_controller_keeps_valid_colon_address() -> Result<()> {
    let temp_dir = setup_temp_home();
    let home = get_temp_home_path(&temp_dir);
    let cm = ConfigManager::with_home(home)?;

    let listener = std::net::TcpListener::bind("127.0.0.1:0")?;
    let port = listener.local_addr()?.port();
    drop(listener);

    let config = format!(
        r#"
port: 7890
socks-port: 7891
external-controller: :{}
"#,
        port
    );
    cm.save("valid-colon", &config).await?;
    cm.set_current("valid-colon").await?;

    let controller = cm.ensure_external_controller().await?;
    assert_eq!(controller, format!("http://127.0.0.1:{}", port));

    Ok(())
}

#[tokio::test]
async fn test_ensure_external_controller_updates_occupied_port() -> Result<()> {
    let temp_dir = setup_temp_home();
    let home = get_temp_home_path(&temp_dir);
    let cm = ConfigManager::with_home(home)?;

    let listener = std::net::TcpListener::bind("127.0.0.1:0")?;
    let occupied = listener.local_addr()?.port();

    let config = format!(
        r#"
port: 7890
socks-port: 7891
external-controller: 127.0.0.1:{}
"#,
        occupied
    );
    cm.save("occupied-port", &config).await?;
    cm.set_current("occupied-port").await?;

    let controller = cm.ensure_external_controller().await?;
    assert!(controller.starts_with("http://127.0.0.1:"));
    assert_ne!(controller, format!("http://127.0.0.1:{}", occupied));

    drop(listener);
    Ok(())
}

#[tokio::test]
async fn test_ensure_external_controller_updates_invalid_address() -> Result<()> {
    let temp_dir = setup_temp_home();
    let home = get_temp_home_path(&temp_dir);
    let cm = ConfigManager::with_home(home)?;

    let config = r#"
port: 7890
socks-port: 7891
external-controller: invalid-address
"#;
    cm.save("invalid-controller", config).await?;
    cm.set_current("invalid-controller").await?;

    let controller = cm.ensure_external_controller().await?;
    assert!(controller.starts_with("http://127.0.0.1:"));

    Ok(())
}

#[tokio::test]
async fn test_load_nonexistent_profile_should_fail() -> Result<()> {
    let temp_dir = setup_temp_home();
    let home = get_temp_home_path(&temp_dir);
    let cm = ConfigManager::with_home(home)?;

    let result = cm.load("missing").await;
    assert!(result.is_err());

    Ok(())
}

#[tokio::test]
async fn test_delete_nonexistent_profile_should_fail() -> Result<()> {
    let temp_dir = setup_temp_home();
    let home = get_temp_home_path(&temp_dir);
    let cm = ConfigManager::with_home(home)?;

    let result = cm.delete_profile("missing").await;
    assert!(result.is_err());

    Ok(())
}
