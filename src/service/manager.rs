use super::process;
use crate::core::{get_home_dir, MihomoError, Result};
use std::path::PathBuf;
#[cfg(test)]
use std::sync::atomic::{AtomicBool, Ordering};

#[cfg(test)]
static FORCE_START_FAILURE_AFTER_SPAWN: AtomicBool = AtomicBool::new(false);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServiceStatus {
    Running(u32),
    Stopped,
}

pub struct ServiceManager {
    binary_path: PathBuf,
    config_path: PathBuf,
    pid_file: PathBuf,
}

impl ServiceManager {
    pub fn new(binary_path: PathBuf, config_path: PathBuf) -> Self {
        let home = get_home_dir().unwrap_or_else(|_| PathBuf::from("."));
        let pid_file = home.join("mihomo.pid");

        Self {
            binary_path,
            config_path,
            pid_file,
        }
    }

    pub fn with_home(binary_path: PathBuf, config_path: PathBuf, home: PathBuf) -> Self {
        let pid_file = home.join("mihomo.pid");

        Self {
            binary_path,
            config_path,
            pid_file,
        }
    }

    pub fn with_pid_file(binary_path: PathBuf, config_path: PathBuf, pid_file: PathBuf) -> Self {
        Self {
            binary_path,
            config_path,
            pid_file,
        }
    }

    pub async fn start(&self) -> Result<()> {
        if self.is_running().await {
            return Err(MihomoError::Service(
                "Service is already running".to_string(),
            ));
        }

        let pid = process::spawn_daemon(&self.binary_path, &self.config_path).await?;
        process::write_pid_file(&self.pid_file, pid).await?;

        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        #[cfg(test)]
        if FORCE_START_FAILURE_AFTER_SPAWN.load(Ordering::SeqCst) {
            process::remove_pid_file(&self.pid_file).await?;
            return Err(MihomoError::Service("Service failed to start".to_string()));
        }

        if !process::is_process_alive(pid) {
            process::remove_pid_file(&self.pid_file).await?;
            return Err(MihomoError::Service("Service failed to start".to_string()));
        }

        Ok(())
    }

    pub async fn stop(&self) -> Result<()> {
        let pid = process::read_pid_file(&self.pid_file).await?;

        if !process::is_process_alive(pid) {
            process::remove_pid_file(&self.pid_file).await?;
            return Err(MihomoError::Service("Service is not running".to_string()));
        }

        process::kill_process(pid)?;
        process::remove_pid_file(&self.pid_file).await?;

        Ok(())
    }

    pub async fn restart(&self) -> Result<()> {
        if self.is_running().await {
            self.stop().await?;
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        }
        self.start().await
    }

    pub async fn status(&self) -> Result<ServiceStatus> {
        match process::read_pid_file(&self.pid_file).await {
            Ok(pid) => {
                if process::is_process_alive(pid) {
                    Ok(ServiceStatus::Running(pid))
                } else {
                    process::remove_pid_file(&self.pid_file).await?;
                    Ok(ServiceStatus::Stopped)
                }
            }
            Err(_) => Ok(ServiceStatus::Stopped),
        }
    }

    pub async fn is_running(&self) -> bool {
        matches!(self.status().await, Ok(ServiceStatus::Running(_)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::sync::{Mutex, OnceLock};
    use tempfile::tempdir;
    use tokio::fs;

    fn test_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    fn make_manager(pid_file: PathBuf) -> ServiceManager {
        ServiceManager::with_pid_file(
            PathBuf::from("/tmp/mihomo"),
            PathBuf::from("/tmp/config.yaml"),
            pid_file,
        )
    }

    #[tokio::test]
    async fn status_without_pid_file_is_stopped() {
        let temp = tempdir().expect("temp dir");
        let manager = make_manager(temp.path().join("mihomo.pid"));

        let status = manager.status().await.expect("status");
        assert_eq!(status, ServiceStatus::Stopped);
    }

    #[tokio::test]
    async fn status_with_current_pid_is_running() {
        let temp = tempdir().expect("temp dir");
        let pid_file = temp.path().join("mihomo.pid");
        let current_pid = std::process::id();
        fs::write(&pid_file, current_pid.to_string())
            .await
            .expect("write pid");

        let manager = make_manager(pid_file);
        let status = manager.status().await.expect("status");
        assert_eq!(status, ServiceStatus::Running(current_pid));
        assert!(manager.is_running().await);
    }

    #[tokio::test]
    async fn stale_pid_is_cleaned_up_and_marked_stopped() {
        let temp = tempdir().expect("temp dir");
        let pid_file = temp.path().join("mihomo.pid");
        fs::write(&pid_file, u32::MAX.to_string())
            .await
            .expect("write pid");

        let manager = make_manager(pid_file.clone());
        let status = manager.status().await.expect("status");

        assert_eq!(status, ServiceStatus::Stopped);
        assert!(!pid_file.exists());
    }

    #[tokio::test]
    async fn stop_with_stale_pid_returns_error_and_cleans_pid_file() {
        let temp = tempdir().expect("temp dir");
        let pid_file = temp.path().join("mihomo.pid");
        fs::write(&pid_file, u32::MAX.to_string())
            .await
            .expect("write pid");

        let manager = make_manager(pid_file.clone());
        let err = manager.stop().await.expect_err("expected service error");

        assert!(matches!(err, MihomoError::Service(_)));
        assert!(!pid_file.exists());
    }

    #[tokio::test]
    async fn start_fails_when_service_already_running() {
        let temp = tempdir().expect("temp dir");
        let pid_file = temp.path().join("mihomo.pid");
        let current_pid = std::process::id();
        fs::write(&pid_file, current_pid.to_string())
            .await
            .expect("write pid");

        let manager = make_manager(pid_file);
        let err = manager.start().await.expect_err("already running");

        assert!(matches!(err, MihomoError::Service(_)));
    }

    #[tokio::test]
    async fn restart_when_not_running_propagates_start_error() {
        let temp = tempdir().expect("temp dir");
        let manager = make_manager(temp.path().join("missing.pid"));

        let err = manager.restart().await.expect_err("restart should fail");
        assert!(matches!(err, MihomoError::NotFound(_)));
    }

    #[tokio::test]
    async fn constructor_new_creates_usable_manager() {
        let manager = ServiceManager::new(
            PathBuf::from("/tmp/mihomo"),
            PathBuf::from("/tmp/config.yaml"),
        );
        let status = manager.status().await.expect("status");
        assert_eq!(status, ServiceStatus::Stopped);
    }

    #[tokio::test]
    async fn constructor_with_home_uses_custom_pid_location() {
        let temp = tempdir().expect("temp dir");
        let current_pid = std::process::id();
        let pid_file = temp.path().join("mihomo.pid");
        fs::write(&pid_file, current_pid.to_string())
            .await
            .expect("write pid");

        let manager = ServiceManager::with_home(
            PathBuf::from("/tmp/mihomo"),
            PathBuf::from("/tmp/config.yaml"),
            temp.path().to_path_buf(),
        );
        let status = manager.status().await.expect("status");
        assert_eq!(status, ServiceStatus::Running(current_pid));
    }

    #[cfg(unix)]
    #[tokio::test]
    #[allow(clippy::await_holding_lock)]
    async fn start_and_stop_service_happy_path() {
        let _guard = test_lock().lock().expect("test lock");
        use std::os::unix::fs::PermissionsExt;

        let temp = tempdir().expect("temp dir");
        let binary = temp.path().join("mihomo");
        let config = temp.path().join("config.yaml");
        let pid_file = temp.path().join("mihomo.pid");

        fs::write(&binary, "#!/bin/sh\nsleep 30\n")
            .await
            .expect("write binary");
        fs::set_permissions(&binary, std::fs::Permissions::from_mode(0o755))
            .await
            .expect("chmod");
        fs::write(&config, "port: 7890\n")
            .await
            .expect("write config");

        let manager = ServiceManager::with_pid_file(binary, config, pid_file.clone());
        manager.start().await.expect("start");
        assert!(pid_file.exists());
        assert!(matches!(
            manager.status().await.expect("status"),
            ServiceStatus::Running(_)
        ));

        manager.stop().await.expect("stop");
        assert!(!pid_file.exists());
        assert_eq!(
            manager.status().await.expect("status"),
            ServiceStatus::Stopped
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    #[allow(clippy::await_holding_lock)]
    async fn start_fails_when_spawned_process_exits_immediately() {
        let _guard = test_lock().lock().expect("test lock");
        use std::os::unix::fs::PermissionsExt;

        let temp = tempdir().expect("temp dir");
        let binary = temp.path().join("mihomo");
        let config = temp.path().join("config.yaml");
        let pid_file = temp.path().join("mihomo.pid");

        fs::write(&binary, "#!/bin/sh\nexit 0\n")
            .await
            .expect("write binary");
        fs::set_permissions(&binary, std::fs::Permissions::from_mode(0o755))
            .await
            .expect("chmod");
        fs::write(&config, "port: 7890\n")
            .await
            .expect("write config");

        let manager = ServiceManager::with_pid_file(binary, config, pid_file.clone());
        match manager.start().await {
            Err(err) => {
                assert!(matches!(err, MihomoError::Service(_)));
                assert!(!pid_file.exists());
            }
            Ok(()) => {
                manager.stop().await.expect("stop after successful start");
                assert!(!pid_file.exists());
            }
        }
    }

    #[cfg(unix)]
    #[tokio::test]
    #[allow(clippy::await_holding_lock)]
    async fn start_forced_failure_after_spawn_cleans_pid_file() {
        let _guard = test_lock().lock().expect("test lock");
        use std::os::unix::fs::PermissionsExt;

        let temp = tempdir().expect("temp dir");
        let binary = temp.path().join("mihomo");
        let config = temp.path().join("config.yaml");
        let pid_file = temp.path().join("mihomo.pid");

        fs::write(&binary, "#!/bin/sh\nsleep 30\n")
            .await
            .expect("write binary");
        fs::set_permissions(&binary, std::fs::Permissions::from_mode(0o755))
            .await
            .expect("chmod");
        fs::write(&config, "port: 7890\n")
            .await
            .expect("write config");

        let manager = ServiceManager::with_pid_file(binary, config, pid_file.clone());
        FORCE_START_FAILURE_AFTER_SPAWN.store(true, Ordering::SeqCst);
        let err = manager.start().await.expect_err("forced start failure");
        FORCE_START_FAILURE_AFTER_SPAWN.store(false, Ordering::SeqCst);

        assert!(matches!(err, MihomoError::Service(_)));
        assert!(!pid_file.exists());
    }

    #[cfg(unix)]
    #[tokio::test]
    #[allow(clippy::await_holding_lock)]
    async fn restart_running_service_stops_then_retries_start() {
        let _guard = test_lock().lock().expect("test lock");
        use std::os::unix::fs::PermissionsExt;

        let temp = tempdir().expect("temp dir");
        let runtime_binary = temp.path().join("runtime-mihomo");
        let runtime_config = temp.path().join("runtime-config.yaml");
        fs::write(&runtime_binary, "#!/bin/sh\nsleep 30\n")
            .await
            .expect("write runtime binary");
        fs::set_permissions(&runtime_binary, std::fs::Permissions::from_mode(0o755))
            .await
            .expect("chmod runtime binary");
        fs::write(&runtime_config, "port: 7890\n")
            .await
            .expect("write runtime config");

        let running_pid = process::spawn_daemon(&runtime_binary, &runtime_config)
            .await
            .expect("spawn runtime service");

        let pid_file = temp.path().join("mihomo.pid");
        process::write_pid_file(&pid_file, running_pid)
            .await
            .expect("write pid file");

        // Use an invalid binary for restart-start phase to force start error
        let manager = ServiceManager::with_pid_file(
            temp.path().join("missing-binary"),
            runtime_config,
            pid_file.clone(),
        );

        let err = manager
            .restart()
            .await
            .expect_err("restart should fail at start");
        assert!(matches!(err, MihomoError::NotFound(_)));
        assert!(!process::is_process_alive(running_pid));
        assert!(!pid_file.exists());
    }
}
