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
    use std::path::PathBuf;
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
        assert!(is_process_alive_checked(pid, None));
        assert!(is_process_alive_checked(pid, start_time));
        assert!(!is_process_alive_checked(
            pid,
            start_time.map(|value| value.saturating_add(1))
        ));
    }

    #[tokio::test]
    async fn test_read_write_pid_file_legacy_helpers() {
        let file = NamedTempFile::new().expect("create temp file");
        let path = file.path();

        write_pid_file(path, 4242).await.expect("write pid file");
        let pid = read_pid_file(path).await.expect("read pid file");
        assert_eq!(pid, 4242);
    }

    #[tokio::test]
    async fn test_read_pid_record_invalid_content_errors() {
        let file = NamedTempFile::new().expect("create temp file");
        let path = file.path();

        fs::write(path, "not-a-pid")
            .await
            .expect("write invalid pid");
        assert!(read_pid_record(path).await.is_err());

        fs::write(path, "1234:not-a-start-time")
            .await
            .expect("write invalid start_time");
        assert!(read_pid_record(path).await.is_err());
    }

    #[tokio::test]
    async fn test_remove_pid_file_exists_and_missing() {
        let file = NamedTempFile::new().expect("create temp file");
        let path = file.path().to_path_buf();

        remove_pid_file(&path)
            .await
            .expect("remove existing pid file");
        assert!(!path.exists());

        remove_pid_file(&path)
            .await
            .expect("remove missing pid file");
    }

    #[tokio::test]
    async fn test_spawn_daemon_reports_missing_binary_and_config() {
        let missing_binary = PathBuf::from("/definitely/not/existing/mihomo-bin");
        let missing_config = PathBuf::from("/definitely/not/existing/config.yaml");
        assert!(spawn_daemon(&missing_binary, &missing_config)
            .await
            .is_err());

        let binary_file = NamedTempFile::new().expect("create fake binary");
        assert!(spawn_daemon(binary_file.path(), &missing_config)
            .await
            .is_err());
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn test_spawn_daemon_returns_service_error_when_spawn_fails() {
        use std::os::unix::fs::PermissionsExt;

        let binary_file = NamedTempFile::new().expect("create fake binary");
        let config_file = NamedTempFile::new().expect("create fake config");

        fs::write(binary_file.path(), "#!/bin/sh\necho test\n")
            .await
            .expect("write fake binary");
        tokio::fs::set_permissions(binary_file.path(), std::fs::Permissions::from_mode(0o644))
            .await
            .expect("set non-executable permissions");

        let err = spawn_daemon(binary_file.path(), config_file.path())
            .await
            .expect_err("spawn should fail for non-executable file");
        assert!(err.to_string().contains("Failed to spawn process"));
    }

    #[test]
    fn test_kill_process_checked_rejects_mismatched_process_record() {
        let err =
            kill_process_checked(u32::MAX, Some(1)).expect_err("mismatched pid record should fail");
        assert!(err
            .to_string()
            .contains("no longer matches tracked process"));
    }

    #[tokio::test]
    async fn test_write_pid_record_creates_parent_directories() {
        let file = NamedTempFile::new().expect("create temp file");
        let nested = file
            .path()
            .parent()
            .expect("temp file parent")
            .join("nested")
            .join("dir")
            .join("mihomo.pid");

        write_pid_record(&nested, 7788, Some(9900))
            .await
            .expect("write pid record into nested path");
        let record = read_pid_record(&nested)
            .await
            .expect("read nested pid record");
        assert_eq!(record.pid, 7788);
        assert_eq!(record.start_time, Some(9900));
    }
}
