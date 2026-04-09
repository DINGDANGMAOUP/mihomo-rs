use super::process;
use crate::core::{get_home_dir, MihomoError, Result};
use std::path::PathBuf;
use std::time::Duration;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServiceStatus {
    Running(u32),
    Stopped,
}

pub struct ServiceManager {
    binary_path: PathBuf,
    config_path: PathBuf,
    pid_file: PathBuf,
    stop_retries: u32,
    stop_interval: Duration,
}

const DEFAULT_STOP_RETRIES: u32 = 20;
const DEFAULT_STOP_INTERVAL_MS: u64 = 100;

impl ServiceManager {
    pub fn new(binary_path: PathBuf, config_path: PathBuf) -> Self {
        let home = get_home_dir().unwrap_or_else(|_| PathBuf::from("."));
        let pid_file = home.join("mihomo.pid");

        Self {
            binary_path,
            config_path,
            pid_file,
            stop_retries: DEFAULT_STOP_RETRIES,
            stop_interval: Duration::from_millis(DEFAULT_STOP_INTERVAL_MS),
        }
    }

    pub fn with_home(binary_path: PathBuf, config_path: PathBuf, home: PathBuf) -> Self {
        let pid_file = home.join("mihomo.pid");

        Self {
            binary_path,
            config_path,
            pid_file,
            stop_retries: DEFAULT_STOP_RETRIES,
            stop_interval: Duration::from_millis(DEFAULT_STOP_INTERVAL_MS),
        }
    }

    pub fn with_pid_file(binary_path: PathBuf, config_path: PathBuf, pid_file: PathBuf) -> Self {
        Self {
            binary_path,
            config_path,
            pid_file,
            stop_retries: DEFAULT_STOP_RETRIES,
            stop_interval: Duration::from_millis(DEFAULT_STOP_INTERVAL_MS),
        }
    }

    pub fn with_stop_wait(mut self, retries: u32, interval: Duration) -> Self {
        self.stop_retries = retries.max(1);
        self.stop_interval = interval.max(Duration::from_millis(1));
        self
    }

    pub async fn start(&self) -> Result<()> {
        if self.is_running().await {
            return Err(MihomoError::Service(
                "Service is already running".to_string(),
            ));
        }

        let pid = process::spawn_daemon(&self.binary_path, &self.config_path).await?;

        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        if !process::is_process_alive(pid) {
            process::remove_pid_file(&self.pid_file).await?;
            return Err(MihomoError::Service("Service failed to start".to_string()));
        }

        let start_time = process::get_process_start_time(pid);
        process::write_pid_record(&self.pid_file, pid, start_time).await?;

        Ok(())
    }

    pub async fn stop(&self) -> Result<()> {
        let record = process::read_pid_record(&self.pid_file).await?;

        if !process::is_process_alive_checked(record.pid, record.start_time) {
            process::remove_pid_file(&self.pid_file).await?;
            return Err(MihomoError::Service("Service is not running".to_string()));
        }

        process::kill_process_checked(record.pid, record.start_time)?;

        let stopped = Self::wait_for_stop(
            || !process::is_process_alive_checked(record.pid, record.start_time),
            self.stop_retries,
            self.stop_interval,
        )
        .await;

        if !stopped {
            return Err(MihomoError::Service(
                "Service did not stop within timeout".to_string(),
            ));
        }

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
        match process::read_pid_record(&self.pid_file).await {
            Ok(record) => {
                if process::is_process_alive_checked(record.pid, record.start_time) {
                    Ok(ServiceStatus::Running(record.pid))
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

    async fn wait_for_stop<F>(mut is_stopped: F, retries: u32, interval: Duration) -> bool
    where
        F: FnMut() -> bool,
    {
        for _ in 0..retries {
            if is_stopped() {
                return true;
            }
            tokio::time::sleep(interval).await;
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_status_cleans_stale_pid_record() {
        let dir = tempdir().expect("create temp dir");
        let pid_file = dir.path().join("mihomo.pid");

        process::write_pid_record(&pid_file, u32::MAX, Some(1))
            .await
            .expect("write stale pid");

        let manager = ServiceManager::with_pid_file(
            PathBuf::from("/bin/echo"),
            PathBuf::from("/tmp/config.yaml"),
            pid_file.clone(),
        );

        let status = manager.status().await.expect("status check");
        assert_eq!(status, ServiceStatus::Stopped);
        assert!(!pid_file.exists());
    }

    #[tokio::test]
    async fn test_wait_for_stop_succeeds_after_retries() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;

        let count = Arc::new(AtomicUsize::new(0));
        let count_clone = count.clone();
        let stopped = ServiceManager::wait_for_stop(
            move || count_clone.fetch_add(1, Ordering::Relaxed) >= 2,
            5,
            Duration::from_millis(1),
        )
        .await;

        assert!(stopped);
        assert!(count.load(Ordering::Relaxed) >= 3);
    }

    #[tokio::test]
    async fn test_wait_for_stop_returns_false_when_condition_never_met() {
        let stopped = ServiceManager::wait_for_stop(|| false, 2, Duration::from_millis(1)).await;
        assert!(!stopped);
    }

    #[test]
    fn test_with_stop_wait_overrides_defaults() {
        let manager = ServiceManager::with_pid_file(
            PathBuf::from("/bin/echo"),
            PathBuf::from("/tmp/config.yaml"),
            PathBuf::from("/tmp/mihomo.pid"),
        )
        .with_stop_wait(3, Duration::from_millis(5));

        assert_eq!(manager.stop_retries, 3);
        assert_eq!(manager.stop_interval, Duration::from_millis(5));
    }

    #[test]
    fn test_with_stop_wait_clamps_to_minimum_values() {
        let manager = ServiceManager::with_pid_file(
            PathBuf::from("/bin/echo"),
            PathBuf::from("/tmp/config.yaml"),
            PathBuf::from("/tmp/mihomo.pid"),
        )
        .with_stop_wait(0, Duration::from_millis(0));

        assert_eq!(manager.stop_retries, 1);
        assert_eq!(manager.stop_interval, Duration::from_millis(1));
    }
}
