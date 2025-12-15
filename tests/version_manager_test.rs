mod common;

use mihomo_rs::{VersionManager, Result};
use common::{setup_temp_home, get_temp_home_path};

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

// Note: We don't test actual installation in unit tests as it requires:
// - Network connection
// - Downloading large binaries
// - Platform-specific behavior
// These are better suited for integration tests or manual testing
