mod common;

use common::{get_temp_home_path, setup_temp_home};
use mihomo_rs::{Result, VersionManager};
use std::sync::{Mutex, OnceLock};
use tokio::fs;

fn env_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

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
async fn test_set_default_and_get_default() -> Result<()> {
    let temp_dir = setup_temp_home();
    let home = get_temp_home_path(&temp_dir);
    let vm = VersionManager::with_home(home.clone())?;

    let version = "v1.0.0";
    let binary_name = if cfg!(windows) {
        "mihomo.exe"
    } else {
        "mihomo"
    };
    let version_dir = home.join("versions").join(version);
    fs::create_dir_all(&version_dir).await?;
    fs::write(version_dir.join(binary_name), b"bin").await?;

    vm.set_default(version).await?;
    let current = vm.get_default().await?;
    assert_eq!(current, version);

    Ok(())
}

#[tokio::test]
async fn test_uninstall_default_version_should_fail() -> Result<()> {
    let temp_dir = setup_temp_home();
    let home = get_temp_home_path(&temp_dir);
    let vm = VersionManager::with_home(home.clone())?;

    let version = "v1.0.0";
    let binary_name = if cfg!(windows) {
        "mihomo.exe"
    } else {
        "mihomo"
    };
    let version_dir = home.join("versions").join(version);
    fs::create_dir_all(&version_dir).await?;
    fs::write(version_dir.join(binary_name), b"bin").await?;
    vm.set_default(version).await?;

    let result = vm.uninstall(version).await;
    assert!(result.is_err());

    Ok(())
}

#[tokio::test]
async fn test_uninstall_non_default_version() -> Result<()> {
    let temp_dir = setup_temp_home();
    let home = get_temp_home_path(&temp_dir);
    let vm = VersionManager::with_home(home.clone())?;

    let binary_name = if cfg!(windows) {
        "mihomo.exe"
    } else {
        "mihomo"
    };
    let keep_version = "v1.0.0";
    let remove_version = "v1.0.1";

    for version in [keep_version, remove_version] {
        let version_dir = home.join("versions").join(version);
        fs::create_dir_all(&version_dir).await?;
        fs::write(version_dir.join(binary_name), b"bin").await?;
    }

    vm.set_default(keep_version).await?;
    vm.uninstall(remove_version).await?;

    let removed_dir = home.join("versions").join(remove_version);
    assert!(!removed_dir.exists());

    Ok(())
}

#[tokio::test]
async fn test_get_default_with_invalid_config_should_fail() -> Result<()> {
    let temp_dir = setup_temp_home();
    let home = get_temp_home_path(&temp_dir);
    let vm = VersionManager::with_home(home.clone())?;

    fs::write(home.join("config.toml"), "invalid = [").await?;

    let result = vm.get_default().await;
    assert!(result.is_err());

    Ok(())
}

#[tokio::test]
#[allow(clippy::await_holding_lock)]
async fn test_version_manager_new_uses_mihomo_home_env() -> Result<()> {
    let _guard = env_lock().lock().expect("env lock");
    let temp_dir = setup_temp_home();
    let home = get_temp_home_path(&temp_dir);
    let old = std::env::var("MIHOMO_HOME").ok();
    // SAFETY: env updates are serialized by a process-wide mutex in this test module.
    unsafe { std::env::set_var("MIHOMO_HOME", &home) };

    let vm = VersionManager::new()?;
    assert!(vm.list_installed().await?.is_empty());

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
async fn test_install_returns_error_when_version_already_exists() -> Result<()> {
    let temp_dir = setup_temp_home();
    let home = get_temp_home_path(&temp_dir);
    let vm = VersionManager::with_home(home.clone())?;

    let version = "v-dup";
    fs::create_dir_all(home.join("versions").join(version)).await?;

    let result = vm.install(version).await;
    assert!(result.is_err());

    Ok(())
}

#[tokio::test]
async fn test_get_binary_path_with_specific_version_success() -> Result<()> {
    let temp_dir = setup_temp_home();
    let home = get_temp_home_path(&temp_dir);
    let vm = VersionManager::with_home(home.clone())?;

    let version = "v1.2.3";
    let binary_name = if cfg!(windows) {
        "mihomo.exe"
    } else {
        "mihomo"
    };
    let version_dir = home.join("versions").join(version);
    let expected = version_dir.join(binary_name);
    fs::create_dir_all(&version_dir).await?;
    fs::write(&expected, b"bin").await?;

    let path = vm.get_binary_path(Some(version)).await?;
    assert_eq!(path, expected);

    Ok(())
}

#[tokio::test]
async fn test_get_binary_path_with_specific_version_missing_binary_should_fail() -> Result<()> {
    let temp_dir = setup_temp_home();
    let home = get_temp_home_path(&temp_dir);
    let vm = VersionManager::with_home(home.clone())?;

    let version = "v1.2.4";
    fs::create_dir_all(home.join("versions").join(version)).await?;

    let result = vm.get_binary_path(Some(version)).await;
    assert!(result.is_err());

    Ok(())
}

#[tokio::test]
async fn test_get_default_with_missing_version_field_should_fail() -> Result<()> {
    let temp_dir = setup_temp_home();
    let home = get_temp_home_path(&temp_dir);
    let vm = VersionManager::with_home(home.clone())?;

    fs::write(home.join("config.toml"), "[default]\nname = \"x\"\n").await?;
    let result = vm.get_default().await;
    assert!(result.is_err());

    Ok(())
}

#[tokio::test]
async fn test_list_installed_sorts_and_marks_default() -> Result<()> {
    let temp_dir = setup_temp_home();
    let home = get_temp_home_path(&temp_dir);
    let vm = VersionManager::with_home(home.clone())?;

    let binary_name = if cfg!(windows) {
        "mihomo.exe"
    } else {
        "mihomo"
    };
    for version in ["v1.0.0", "v2.0.0"] {
        let version_dir = home.join("versions").join(version);
        fs::create_dir_all(&version_dir).await?;
        fs::write(version_dir.join(binary_name), b"bin").await?;
    }
    vm.set_default("v1.0.0").await?;

    let versions = vm.list_installed().await?;
    assert_eq!(versions.len(), 2);
    assert_eq!(versions[0].version, "v2.0.0");
    assert_eq!(versions[1].version, "v1.0.0");
    assert!(!versions[0].is_default);
    assert!(versions[1].is_default);

    Ok(())
}

#[tokio::test]
async fn test_set_default_nonexistent_version_should_fail() -> Result<()> {
    let temp_dir = setup_temp_home();
    let home = get_temp_home_path(&temp_dir);
    let vm = VersionManager::with_home(home)?;

    let result = vm.set_default("v-missing").await;
    assert!(result.is_err());

    Ok(())
}

#[tokio::test]
async fn test_uninstall_nonexistent_version_should_fail() -> Result<()> {
    let temp_dir = setup_temp_home();
    let home = get_temp_home_path(&temp_dir);
    let vm = VersionManager::with_home(home)?;

    let result = vm.uninstall("v-missing").await;
    assert!(result.is_err());

    Ok(())
}
