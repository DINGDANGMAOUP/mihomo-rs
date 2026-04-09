mod common;

use common::{get_temp_home_path, setup_temp_home};
use mihomo_rs::{Result, VersionManager};
use tokio::fs;

#[tokio::test]
async fn test_version_manager_new() -> Result<()> {
    let temp_dir = setup_temp_home();
    let home = get_temp_home_path(&temp_dir);

    let vm = VersionManager::with_home(home)?;

    // Verify manager was created
    let versions = vm.list_installed().await?;
    assert_eq!(versions.len(), 0); // New home should have no versions

    Ok(())
}

#[tokio::test]
async fn test_list_installed_empty() -> Result<()> {
    let temp_dir = setup_temp_home();
    let home = get_temp_home_path(&temp_dir);

    let vm = VersionManager::with_home(home)?;

    let versions = vm.list_installed().await?;
    assert!(versions.is_empty());

    Ok(())
}

#[tokio::test]
async fn test_get_default_when_none_set() {
    let temp_dir = setup_temp_home();
    let home = get_temp_home_path(&temp_dir);

    let vm = VersionManager::with_home(home).unwrap();

    // Should error when no default is set
    let result = vm.get_default().await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_get_binary_path_when_none_installed() {
    let temp_dir = setup_temp_home();
    let home = get_temp_home_path(&temp_dir);

    let vm = VersionManager::with_home(home).unwrap();

    // Should error when no version is installed
    let result = vm.get_binary_path(None).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_set_default_preserves_existing_profile_setting() -> Result<()> {
    let temp_dir = setup_temp_home();
    let home = get_temp_home_path(&temp_dir);
    let vm = VersionManager::with_home(home.clone())?;

    let version_dir = home.join("versions").join("v1.0.0");
    fs::create_dir_all(&version_dir).await?;
    fs::write(version_dir.join("mihomo"), "dummy binary").await?;
    fs::write(home.join("config.toml"), "[default]\nprofile = \"work\"\n").await?;

    vm.set_default("v1.0.0").await?;

    let content = fs::read_to_string(home.join("config.toml")).await?;
    let config: toml::Value = toml::from_str(&content).expect("config.toml should be valid TOML");
    assert_eq!(
        config
            .get("default")
            .and_then(|d| d.get("profile"))
            .and_then(|v| v.as_str()),
        Some("work")
    );
    assert_eq!(
        config
            .get("default")
            .and_then(|d| d.get("version"))
            .and_then(|v| v.as_str()),
        Some("v1.0.0")
    );

    Ok(())
}

#[tokio::test]
async fn test_set_default_fails_on_invalid_config_file() -> Result<()> {
    let temp_dir = setup_temp_home();
    let home = get_temp_home_path(&temp_dir);
    let vm = VersionManager::with_home(home.clone())?;

    let version_dir = home.join("versions").join("v1.0.0");
    fs::create_dir_all(&version_dir).await?;
    fs::write(version_dir.join("mihomo"), "dummy binary").await?;
    fs::write(home.join("config.toml"), "[default").await?;

    let result = vm.set_default("v1.0.0").await;
    assert!(result.is_err());

    Ok(())
}

#[tokio::test]
async fn test_uninstall_fails_when_config_is_invalid() -> Result<()> {
    let temp_dir = setup_temp_home();
    let home = get_temp_home_path(&temp_dir);
    let vm = VersionManager::with_home(home.clone())?;

    let version_dir = home.join("versions").join("v1.0.0");
    fs::create_dir_all(&version_dir).await?;
    fs::write(version_dir.join("mihomo"), "dummy binary").await?;
    fs::write(home.join("config.toml"), "[default").await?;

    let result = vm.uninstall("v1.0.0").await;
    assert!(result.is_err());

    Ok(())
}
