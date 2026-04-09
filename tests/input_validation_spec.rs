use mihomo_rs::{ConfigManager, MihomoError, VersionManager};
use tempfile::tempdir;
use tokio::fs;

#[tokio::test]
async fn config_manager_rejects_invalid_profile_names() {
    let temp = tempdir().expect("create temp dir");
    let manager = ConfigManager::with_home(temp.path().to_path_buf()).expect("create manager");

    let invalid = "../evil";
    assert!(matches!(
        manager.load(invalid).await.expect_err("load should fail"),
        MihomoError::Config(_)
    ));
    assert!(matches!(
        manager
            .save(invalid, "port: 7890")
            .await
            .expect_err("save should fail"),
        MihomoError::Config(_)
    ));
    assert!(matches!(
        manager
            .set_current(invalid)
            .await
            .expect_err("set_current should fail"),
        MihomoError::Config(_)
    ));
    assert!(matches!(
        manager
            .delete_profile(invalid)
            .await
            .expect_err("delete should fail"),
        MihomoError::Config(_)
    ));
}

#[tokio::test]
async fn version_manager_rejects_invalid_version_names() {
    let temp = tempdir().expect("create temp dir");
    let manager = VersionManager::with_home(temp.path().to_path_buf()).expect("create manager");
    let invalid = "../v1";

    assert!(matches!(
        manager
            .install(invalid)
            .await
            .expect_err("install should fail"),
        MihomoError::Version(_)
    ));
    assert!(matches!(
        manager
            .set_default(invalid)
            .await
            .expect_err("set_default should fail"),
        MihomoError::Version(_)
    ));
    assert!(matches!(
        manager
            .get_binary_path(Some(invalid))
            .await
            .expect_err("get_binary_path should fail"),
        MihomoError::Version(_)
    ));
    assert!(matches!(
        manager
            .uninstall(invalid)
            .await
            .expect_err("uninstall should fail"),
        MihomoError::Version(_)
    ));
}

#[tokio::test]
async fn version_manager_rejects_invalid_default_version_from_config() {
    let temp = tempdir().expect("create temp dir");
    let home = temp.path();
    let manager = VersionManager::with_home(home.to_path_buf()).expect("create manager");

    fs::write(
        home.join("config.toml"),
        "[default]\nversion = \"../invalid\"\n",
    )
    .await
    .expect("write config");

    assert!(matches!(
        manager
            .get_binary_path(None)
            .await
            .expect_err("get_binary_path should reject invalid default"),
        MihomoError::Version(_)
    ));
}
