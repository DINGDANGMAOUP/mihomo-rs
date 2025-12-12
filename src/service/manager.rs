use super::process;
use crate::core::{get_home_dir, MihomoError, Result};
use std::path::PathBuf;

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

        if !process::is_process_alive(pid) {
            process::remove_pid_file(&self.pid_file).await?;
            return Err(MihomoError::Service(
                "Service failed to start".to_string(),
            ));
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
