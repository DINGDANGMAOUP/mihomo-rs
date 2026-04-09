use crate::core::{MihomoError, Result};
use std::path::Path;
use std::process::{Command, Stdio};
use sysinfo::{Pid, ProcessesToUpdate, System};
use tokio::fs;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PidRecord {
    pub pid: u32,
    pub start_time: Option<u64>,
}

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

    let config_dir = config.parent().unwrap_or_else(|| Path::new("."));

    let child = Command::new(binary)
        .arg("-d")
        .arg(config_dir)
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
    system.refresh_processes(ProcessesToUpdate::All, true);

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
    system.refresh_processes(ProcessesToUpdate::All, true);
    system.process(Pid::from_u32(pid)).is_some()
}

pub fn get_process_start_time(pid: u32) -> Option<u64> {
    let mut system = System::new();
    system.refresh_processes(ProcessesToUpdate::All, true);
    system.process(Pid::from_u32(pid)).map(|p| p.start_time())
}

pub fn is_process_alive_checked(pid: u32, expected_start_time: Option<u64>) -> bool {
    if !is_process_alive(pid) {
        return false;
    }

    match expected_start_time {
        Some(expected) => get_process_start_time(pid) == Some(expected),
        None => true,
    }
}

pub fn kill_process_checked(pid: u32, expected_start_time: Option<u64>) -> Result<()> {
    if !is_process_alive_checked(pid, expected_start_time) {
        return Err(MihomoError::Service(format!(
            "PID {} no longer matches tracked process",
            pid
        )));
    }
    kill_process(pid)
}

pub async fn read_pid_file(path: &Path) -> Result<u32> {
    Ok(read_pid_record(path).await?.pid)
}

pub async fn read_pid_record(path: &Path) -> Result<PidRecord> {
    if !path.exists() {
        return Err(MihomoError::NotFound("PID file not found".to_string()));
    }

    let content = fs::read_to_string(path).await?;
    let content = content.trim();

    // New format: "<pid>:<start_time>"
    if let Some((pid_str, start_time_str)) = content.split_once(':') {
        let pid = pid_str
            .trim()
            .parse::<u32>()
            .map_err(|e| MihomoError::Service(format!("Invalid PID in file: {}", e)))?;
        let start_time = start_time_str
            .trim()
            .parse::<u64>()
            .map_err(|e| MihomoError::Service(format!("Invalid start_time in PID file: {}", e)))?;
        return Ok(PidRecord {
            pid,
            start_time: Some(start_time),
        });
    }

    // Legacy format: "<pid>"
    let pid = content
        .parse::<u32>()
        .map_err(|e| MihomoError::Service(format!("Invalid PID in file: {}", e)))?;

    Ok(PidRecord {
        pid,
        start_time: None,
    })
}

pub async fn write_pid_file(path: &Path, pid: u32) -> Result<()> {
    write_pid_record(path, pid, None).await
}

pub async fn write_pid_record(path: &Path, pid: u32, start_time: Option<u64>) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).await?;
    }
    let content = match start_time {
        Some(start_time) => format!("{}:{}", pid, start_time),
        None => pid.to_string(),
    };
    fs::write(path, content).await?;
    Ok(())
}

pub async fn remove_pid_file(path: &Path) -> Result<()> {
    if path.exists() {
        fs::remove_file(path).await?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    use tokio::fs;

    #[test]
    fn test_is_process_alive_with_invalid_pid() {
        // Test with a PID that definitely doesn't exist
        let result = is_process_alive(u32::MAX);
        assert!(!result);
    }

    #[test]
    fn test_is_process_alive_with_current_process() {
        // Test with current process PID
        let current_pid = std::process::id();
        let result = is_process_alive(current_pid);
        assert!(result);
    }

    #[test]
    fn test_kill_process_with_invalid_pid() {
        // Test killing a non-existent process (should succeed without error)
        let result = kill_process(u32::MAX);
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_pid_record_roundtrip_with_start_time() {
        let file = NamedTempFile::new().expect("create temp file");
        let path = file.path();

        write_pid_record(path, 12345, Some(67890))
            .await
            .expect("write pid record");
        let record = read_pid_record(path).await.expect("read pid record");

        assert_eq!(record.pid, 12345);
        assert_eq!(record.start_time, Some(67890));
    }

    #[tokio::test]
    async fn test_pid_record_legacy_format() {
        let file = NamedTempFile::new().expect("create temp file");
        let path = file.path();
        fs::write(path, "24680").await.expect("write legacy pid");

        let record = read_pid_record(path).await.expect("read pid record");
        assert_eq!(record.pid, 24680);
        assert_eq!(record.start_time, None);
    }

    #[test]
    fn test_is_process_alive_checked_current_process() {
        let pid = std::process::id();
        let start_time = get_process_start_time(pid);
        assert!(start_time.is_some());
        assert!(is_process_alive_checked(pid, start_time));
        assert!(!is_process_alive_checked(
            pid,
            start_time.map(|value| value.saturating_add(1))
        ));
    }
}
