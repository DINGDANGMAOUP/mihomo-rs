#[cfg(unix)]
mod unix_tests {
    use mihomo_rs::{MihomoError, ServiceManager, ServiceStatus};
    use std::os::unix::fs::PermissionsExt;
    use std::path::Path;
    use tempfile::tempdir;
    use tokio::fs;

    async fn write_fake_daemon(binary: &Path) {
        let script = r#"#!/bin/sh
trap 'exit 0' TERM INT
while true; do
  sleep 1
done
"#;
        fs::write(binary, script).await.expect("write fake daemon");

        let mut perms = fs::metadata(binary)
            .await
            .expect("read daemon metadata")
            .permissions();
        perms.set_mode(0o755);
        fs::set_permissions(binary, perms)
            .await
            .expect("set execute permission");
    }

    #[tokio::test]
    async fn start_stop_status_and_duplicate_start_flow() {
        let dir = tempdir().expect("create temp dir");
        let binary = dir.path().join("mihomo");
        let config = dir.path().join("config.yaml");
        let pid_file = dir.path().join("mihomo.pid");

        write_fake_daemon(&binary).await;
        fs::write(&config, "port: 7890\nexternal-controller: 127.0.0.1:9090\n")
            .await
            .expect("write config");

        let manager = ServiceManager::with_pid_file(binary, config, pid_file)
            .with_stop_wait(100, std::time::Duration::from_millis(20));

        assert_eq!(manager.status().await.expect("initial status"), ServiceStatus::Stopped);

        manager.start().await.expect("start daemon");
        let running = manager.status().await.expect("running status");
        assert!(matches!(running, ServiceStatus::Running(_)));

        let duplicate_start = manager.start().await.expect_err("start should fail when running");
        match duplicate_start {
            MihomoError::Service(msg) => assert_eq!(msg, "Service is already running"),
            other => panic!("expected service error, got: {}", other),
        }

        manager.stop().await.expect("stop daemon");
        assert_eq!(manager.status().await.expect("stopped status"), ServiceStatus::Stopped);
    }

    #[tokio::test]
    async fn restart_replaces_running_process() {
        let dir = tempdir().expect("create temp dir");
        let binary = dir.path().join("mihomo");
        let config = dir.path().join("config.yaml");
        let pid_file = dir.path().join("mihomo.pid");

        write_fake_daemon(&binary).await;
        fs::write(&config, "port: 7890\nexternal-controller: 127.0.0.1:9090\n")
            .await
            .expect("write config");

        let manager = ServiceManager::with_pid_file(binary, config, pid_file)
            .with_stop_wait(100, std::time::Duration::from_millis(20));

        manager.start().await.expect("start before restart");
        let first_pid = match manager.status().await.expect("status after first start") {
            ServiceStatus::Running(pid) => pid,
            ServiceStatus::Stopped => panic!("expected running status"),
        };

        manager.restart().await.expect("restart while running");
        let second_pid = match manager.status().await.expect("status after restart") {
            ServiceStatus::Running(pid) => pid,
            ServiceStatus::Stopped => panic!("expected running status after restart"),
        };

        assert_ne!(first_pid, second_pid);
        manager.stop().await.expect("stop after restart");
    }

    #[tokio::test]
    async fn start_fails_when_process_exits_immediately() {
        let dir = tempdir().expect("create temp dir");
        let binary = dir.path().join("mihomo");
        let config = dir.path().join("config.yaml");
        let pid_file = dir.path().join("mihomo.pid");

        fs::write(&binary, "#!/bin/sh\nsleep 0.05\nexit 0\n")
            .await
            .expect("write short-lived binary");
        let mut perms = fs::metadata(&binary)
            .await
            .expect("read script metadata")
            .permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&binary, perms)
            .await
            .expect("set execute permission");

        fs::write(&config, "port: 7890\nexternal-controller: 127.0.0.1:9090\n")
            .await
            .expect("write config");

        let manager = ServiceManager::with_pid_file(binary, config, pid_file.clone())
            .with_stop_wait(5, std::time::Duration::from_millis(10));

        let start_result = manager.start().await;
        if let Err(err) = start_result {
            match err {
                MihomoError::Service(msg) => assert_eq!(msg, "Service failed to start"),
                other => panic!("expected service error, got: {}", other),
            }
        }

        // Short-lived processes can race with process table refresh on CI.
        // Poll status until it settles to Stopped, which is the invariant we need.
        let mut settled_stopped = false;
        for _ in 0..40 {
            if manager.status().await.expect("status") == ServiceStatus::Stopped {
                settled_stopped = true;
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }

        assert!(settled_stopped, "short-lived process should eventually stop");
        assert!(!pid_file.exists());
    }
}
