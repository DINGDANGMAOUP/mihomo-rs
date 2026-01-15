mod common;

use common::{create_test_config, get_temp_home_path, setup_temp_home};
use mihomo_rs::{ConfigManager, Result};

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
