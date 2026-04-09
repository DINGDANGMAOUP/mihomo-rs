mod common;

use common::{install_fake_version, setup_temp_home, temp_home_path};
use mihomo_rs::{MihomoError, VersionManager};
use tokio::fs;

#[tokio::test]
async fn list_installed_is_sorted_and_marks_default() {
    let temp = setup_temp_home();
    let home = temp_home_path(&temp);
    let manager = VersionManager::with_home(home.clone()).expect("create version manager");

    install_fake_version(&home, "v1.2.0").await;
    install_fake_version(&home, "v1.10.0").await;
    install_fake_version(&home, "v1.9.0").await;

    manager
        .set_default("v1.9.0")
        .await
        .expect("set default version");

    let versions = manager
        .list_installed()
        .await
        .expect("list installed versions");
    let names: Vec<_> = versions.iter().map(|v| v.version.clone()).collect();

    assert_eq!(names, vec!["v1.10.0", "v1.9.0", "v1.2.0"]);
    assert!(versions
        .iter()
        .any(|v| v.version == "v1.9.0" && v.is_default));
}

#[tokio::test]
async fn get_default_and_binary_path_work_after_setting_default() {
    let temp = setup_temp_home();
    let home = temp_home_path(&temp);
    let manager = VersionManager::with_home(home.clone()).expect("create version manager");

    let binary = install_fake_version(&home, "v2.0.0").await;
    manager.set_default("v2.0.0").await.expect("set default");

    let default = manager.get_default().await.expect("get default");
    let binary_path = manager
        .get_binary_path(None)
        .await
        .expect("get binary path");

    assert_eq!(default, "v2.0.0");
    assert_eq!(binary_path, binary);
}

#[tokio::test]
async fn set_default_preserves_existing_profile_field() {
    let temp = setup_temp_home();
    let home = temp_home_path(&temp);
    let manager = VersionManager::with_home(home.clone()).expect("create version manager");

    install_fake_version(&home, "v1.0.0").await;
    fs::write(home.join("config.toml"), "[default]\nprofile = \"work\"\n")
        .await
        .expect("write config with profile");

    manager.set_default("v1.0.0").await.expect("set default");

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
            .get("default")
            .and_then(|v| v.get("version"))
            .and_then(|v| v.as_str()),
        Some("v1.0.0")
    );
}

#[tokio::test]
async fn uninstall_rejects_default_but_removes_non_default() {
    let temp = setup_temp_home();
    let home = temp_home_path(&temp);
    let manager = VersionManager::with_home(home.clone()).expect("create version manager");

    install_fake_version(&home, "v1.0.0").await;
    install_fake_version(&home, "v1.1.0").await;

    manager.set_default("v1.0.0").await.expect("set default");

    let err = manager
        .uninstall("v1.0.0")
        .await
        .expect_err("cannot uninstall default version");
    match err {
        MihomoError::Version(msg) => {
            assert_eq!(msg.as_str(), "Cannot uninstall the default version")
        }
        other => panic!("expected version error, got: {}", other),
    }

    manager
        .uninstall("v1.1.0")
        .await
        .expect("uninstall non-default version");
    assert!(!home.join("versions").join("v1.1.0").exists());
}

#[tokio::test]
async fn invalid_config_file_surfaces_as_error() {
    let temp = setup_temp_home();
    let home = temp_home_path(&temp);
    let manager = VersionManager::with_home(home.clone()).expect("create version manager");

    install_fake_version(&home, "v3.0.0").await;
    fs::write(home.join("config.toml"), "[default")
        .await
        .expect("write invalid config");

    let err = manager
        .set_default("v3.0.0")
        .await
        .expect_err("invalid toml should fail");
    assert!(matches!(err, MihomoError::Config(_)));
}

#[tokio::test]
async fn set_default_fails_when_version_not_installed() {
    let temp = setup_temp_home();
    let home = temp_home_path(&temp);
    let manager = VersionManager::with_home(home).expect("create version manager");

    let err = manager
        .set_default("v9.9.9")
        .await
        .expect_err("missing version should fail");
    assert!(matches!(err, MihomoError::NotFound(_)));
}

#[tokio::test]
async fn get_default_and_binary_path_errors_without_setup() {
    let temp = setup_temp_home();
    let home = temp_home_path(&temp);
    let manager = VersionManager::with_home(home.clone()).expect("create version manager");

    let default_err = manager
        .get_default()
        .await
        .expect_err("no default should fail");
    match default_err {
        MihomoError::NotFound(msg) => assert_eq!(msg, "No default version set"),
        other => panic!("expected not found error, got: {}", other),
    }

    let binary_err = manager
        .get_binary_path(Some("v1.0.0"))
        .await
        .expect_err("missing binary should fail");
    assert!(matches!(binary_err, MihomoError::NotFound(_)));

    let version_dir = home.join("versions").join("v1.0.0");
    fs::create_dir_all(&version_dir)
        .await
        .expect("create empty version dir");

    let binary_err_2 = manager
        .get_binary_path(Some("v1.0.0"))
        .await
        .expect_err("missing binary file should still fail");
    assert!(matches!(binary_err_2, MihomoError::NotFound(_)));
}

#[tokio::test]
async fn uninstall_behaves_correctly_without_default_version() {
    let temp = setup_temp_home();
    let home = temp_home_path(&temp);
    let manager = VersionManager::with_home(home.clone()).expect("create version manager");

    install_fake_version(&home, "v4.0.0").await;
    manager
        .uninstall("v4.0.0")
        .await
        .expect("uninstall should work when default is unset");
    assert!(!home.join("versions").join("v4.0.0").exists());

    let err = manager
        .uninstall("v4.0.0")
        .await
        .expect_err("uninstall missing version should fail");
    assert!(matches!(err, MihomoError::NotFound(_)));
}

#[tokio::test]
async fn install_fails_fast_when_version_already_exists() {
    let temp = setup_temp_home();
    let home = temp_home_path(&temp);
    let manager = VersionManager::with_home(home.clone()).expect("create version manager");

    install_fake_version(&home, "v5.0.0").await;
    let err = manager
        .install("v5.0.0")
        .await
        .expect_err("already installed should fail");
    assert!(matches!(err, MihomoError::Version(_)));
}

#[tokio::test]
async fn list_installed_orders_semver_before_non_semver_names() {
    let temp = setup_temp_home();
    let home = temp_home_path(&temp);
    let manager = VersionManager::with_home(home.clone()).expect("create version manager");

    install_fake_version(&home, "v1.0.1").await;
    install_fake_version(&home, "v1.0.0").await;
    install_fake_version(&home, "snapshot-build").await;
    install_fake_version(&home, "dev").await;

    let versions = manager.list_installed().await.expect("list versions");
    let names: Vec<_> = versions.iter().map(|v| v.version.as_str()).collect();

    assert_eq!(names[0], "v1.0.1");
    assert_eq!(names[1], "v1.0.0");
    assert!(names.contains(&"snapshot-build"));
    assert!(names.contains(&"dev"));
}

#[tokio::test]
async fn get_default_errors_when_version_key_missing() {
    let temp = setup_temp_home();
    let home = temp_home_path(&temp);
    let manager = VersionManager::with_home(home.clone()).expect("create version manager");

    fs::write(home.join("config.toml"), "[default]\nprofile = \"work\"\n")
        .await
        .expect("write config without version");

    let err = manager
        .get_default()
        .await
        .expect_err("missing default.version should fail");
    match err {
        MihomoError::Config(msg) => assert_eq!(msg.as_str(), "No default version in config"),
        other => panic!("expected config error, got: {}", other),
    }
}

#[tokio::test]
async fn uninstall_returns_error_when_config_is_invalid() {
    let temp = setup_temp_home();
    let home = temp_home_path(&temp);
    let manager = VersionManager::with_home(home.clone()).expect("create version manager");

    install_fake_version(&home, "v6.0.0").await;
    fs::write(home.join("config.toml"), "[default")
        .await
        .expect("write invalid config");

    let err = manager
        .uninstall("v6.0.0")
        .await
        .expect_err("invalid config should fail uninstall");
    assert!(matches!(err, MihomoError::Config(_)));
}
