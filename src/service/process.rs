use crate::core::{MihomoError, Result};
use std::path::Path;
use std::process::{Command, Stdio};
use sysinfo::{Pid, System};
use tokio::fs;

pub async fn spawn_daemon(binary: &Path, config: &Path) -> Result<u32> {
    if !binary.exists() {
        return Err(MihomoError::NotFound(format!(
            "Binary not found: {}",
            binary.display()
        )));
    }

    if !config.exists() {
        return Err(MihomoError::NotFound(format!(
            "Config not found: {}",
            config.display()
        )));
    }

    let child = Command::new(binary)
        .arg("-d")
        .arg(config.parent().unwrap())
        .arg("-f")
        .arg(config)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| MihomoError::Service(format!("Failed to spawn process: {}", e)))?;

    let pid = child.id();
    Ok(pid)
}

pub fn kill_process(pid: u32) -> Result<()> {
    let mut system = System::new();
    system.refresh_processes();

    let pid = Pid::from_u32(pid);
    if let Some(process) = system.process(pid) {
        if !process.kill() {
            return Err(MihomoError::Service(format!(
                "Failed to kill process {}",
                pid
            )));
        }
    }

    Ok(())
}

pub fn is_process_alive(pid: u32) -> bool {
    let mut system = System::new();
    system.refresh_processes();
    system.process(Pid::from_u32(pid)).is_some()
}

pub async fn read_pid_file(path: &Path) -> Result<u32> {
    if !path.exists() {
        return Err(MihomoError::NotFound("PID file not found".to_string()));
    }

    let content = fs::read_to_string(path).await?;
    let pid = content
        .trim()
        .parse::<u32>()
        .map_err(|e| MihomoError::Service(format!("Invalid PID in file: {}", e)))?;

    Ok(pid)
}

pub async fn write_pid_file(path: &Path, pid: u32) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).await?;
    }
    fs::write(path, pid.to_string()).await?;
    Ok(())
}

pub async fn remove_pid_file(path: &Path) -> Result<()> {
    if path.exists() {
        fs::remove_file(path).await?;
    }
    Ok(())
}
