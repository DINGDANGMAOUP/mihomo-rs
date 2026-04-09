mod common;

use mihomo_rs::{MihomoError, ServiceManager, ServiceStatus};
use std::path::PathBuf;
use tokio::fs;

#[tokio::test]
async fn status_is_stopped_when_pid_file_missing() {
    let temp = common::setup_temp_home();
    let home = common::temp_home_path(&temp);
    let manager = ServiceManager::with_home(
        PathBuf::from("/path/does/not/exist/mihomo"),
        home.join("config.yaml"),
        home.clone(),
    );

    let status = manager.status().await.expect("query status");
    assert_eq!(status, ServiceStatus::Stopped);
    assert!(!manager.is_running().await);
}

#[tokio::test]
async fn status_cleans_stale_pid_record() {
    let temp = common::setup_temp_home();
    let home = common::temp_home_path(&temp);
    let pid_file = home.join("mihomo.pid");

    fs::write(&pid_file, "4294967295:1")
        .await
        .expect("write stale pid");

    let manager = ServiceManager::with_pid_file(
        PathBuf::from("/bin/echo"),
        home.join("config.yaml"),
        pid_file.clone(),
    );

    let status = manager.status().await.expect("check stale pid status");
    assert_eq!(status, ServiceStatus::Stopped);
    assert!(!pid_file.exists());
}

#[tokio::test]
async fn start_stop_restart_return_expected_errors_without_real_process() {
    let temp = common::setup_temp_home();
    let home = common::temp_home_path(&temp);
    let config = home.join("config.yaml");
    fs::write(&config, "port: 7890\n")
        .await
        .expect("write config file");

    let manager = ServiceManager::with_pid_file(
        PathBuf::from("/path/does/not/exist/mihomo"),
        config,
        home.join("mihomo.pid"),
    );

    let start_err = manager.start().await.expect_err("start should fail");
    assert!(matches!(start_err, MihomoError::NotFound(_)));

    let stop_err = manager.stop().await.expect_err("stop should fail");
    assert!(matches!(stop_err, MihomoError::NotFound(_)));

    let restart_err = manager.restart().await.expect_err("restart should fail");
    assert!(matches!(restart_err, MihomoError::NotFound(_)));
}

#[tokio::test]
async fn stop_removes_stale_pid_record_and_reports_not_running() {
    let temp = common::setup_temp_home();
    let home = common::temp_home_path(&temp);
    let pid_file = home.join("mihomo.pid");

    fs::write(&pid_file, "4294967295:1")
        .await
        .expect("write stale pid for stop");

    let manager = ServiceManager::with_pid_file(
        PathBuf::from("/bin/echo"),
        home.join("config.yaml"),
        pid_file.clone(),
    );

    let err = manager
        .stop()
        .await
        .expect_err("stop on stale pid should fail as not running");
    match err {
        MihomoError::Service(msg) => assert_eq!(msg, "Service is not running"),
        other => panic!("expected service error, got: {}", other),
    }
    assert!(!pid_file.exists());
}
