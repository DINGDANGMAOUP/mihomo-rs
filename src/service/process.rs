use crate::core::{MihomoError, Result};
use std::path::Path;
use std::process::{Command, Stdio};
use sysinfo::{Pid, ProcessesToUpdate, System};
use tokio::fs;

#[cfg(test)]
use std::sync::atomic::{AtomicBool, Ordering};

#[cfg(test)]
static FORCE_KILL_FAILURE: AtomicBool = AtomicBool::new(false);
#[cfg(test)]
static FORCE_KILL_RETURNS_FALSE: AtomicBool = AtomicBool::new(false);

fn try_kill_process(process: &sysinfo::Process) -> bool {
    #[cfg(test)]
    if FORCE_KILL_RETURNS_FALSE.load(Ordering::SeqCst) {
        return false;
    }

    process.kill()
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

    let config_dir = config.parent().ok_or_else(|| {
        MihomoError::Service(format!(
            "Invalid config path without parent: {}",
            config.display()
        ))
    })?;

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
        #[cfg(test)]
        if FORCE_KILL_FAILURE.load(Ordering::SeqCst) {
            return Err(MihomoError::Service(format!(
                "Failed to kill process {}",
                pid
            )));
        }

        if !try_kill_process(process) {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command as StdCommand;
    use tempfile::tempdir;
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

    #[test]
    #[cfg(unix)]
    fn test_kill_process_forced_failure_branch() {
        let mut child = StdCommand::new("sleep").arg("5").spawn().unwrap();
        FORCE_KILL_FAILURE.store(true, Ordering::SeqCst);

        let result = kill_process(child.id());
        FORCE_KILL_FAILURE.store(false, Ordering::SeqCst);

        assert!(matches!(result, Err(MihomoError::Service(_))));
        let _ = child.kill();
        let _ = child.wait();
    }

    #[test]
    #[cfg(unix)]
    fn test_kill_process_force_kill_returns_false_branch() {
        let mut child = StdCommand::new("sleep").arg("5").spawn().unwrap();
        FORCE_KILL_RETURNS_FALSE.store(true, Ordering::SeqCst);

        let result = kill_process(child.id());
        FORCE_KILL_RETURNS_FALSE.store(false, Ordering::SeqCst);

        assert!(matches!(result, Err(MihomoError::Service(_))));
        let _ = child.kill();
        let _ = child.wait();
    }

    #[tokio::test]
    async fn test_pid_file_round_trip() {
        let temp = tempdir().expect("tempdir");
        let pid_path = temp.path().join("runtime/mihomo.pid");

        write_pid_file(&pid_path, 4242).await.expect("write pid");
        let pid = read_pid_file(&pid_path).await.expect("read pid");
        assert_eq!(pid, 4242);

        remove_pid_file(&pid_path).await.expect("remove pid");
        assert!(!pid_path.exists());
    }

    #[tokio::test]
    async fn test_read_pid_file_with_invalid_content() {
        let temp = tempdir().expect("tempdir");
        let pid_path = temp.path().join("mihomo.pid");
        fs::write(&pid_path, "not-a-number")
            .await
            .expect("write pid");

        let err = read_pid_file(&pid_path).await.expect_err("invalid pid");
        assert!(matches!(err, MihomoError::Service(_)));
    }

    #[tokio::test]
    async fn test_spawn_daemon_with_missing_binary() {
        let temp = tempdir().expect("tempdir");
        let config = temp.path().join("config.yaml");
        fs::write(&config, "port: 7890\n")
            .await
            .expect("write config");

        let err = spawn_daemon(&temp.path().join("missing-binary"), &config)
            .await
            .expect_err("missing binary");
        assert!(matches!(err, MihomoError::NotFound(_)));
    }

    #[tokio::test]
    async fn test_spawn_daemon_with_missing_config() {
        let temp = tempdir().expect("tempdir");
        let binary = temp.path().join("mihomo");
        fs::write(&binary, "dummy").await.expect("write binary");

        let err = spawn_daemon(&binary, &temp.path().join("missing.yaml"))
            .await
            .expect_err("missing config");
        assert!(matches!(err, MihomoError::NotFound(_)));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn test_spawn_daemon_with_config_path_without_parent() {
        use std::os::unix::fs::PermissionsExt;

        let temp = tempdir().expect("tempdir");
        let binary = temp.path().join("mihomo");
        fs::write(&binary, "#!/bin/sh\nsleep 1\n")
            .await
            .expect("write binary");
        fs::set_permissions(&binary, std::fs::Permissions::from_mode(0o755))
            .await
            .expect("chmod");

        let err = spawn_daemon(&binary, std::path::Path::new("/"))
            .await
            .expect_err("invalid config path without parent");
        assert!(matches!(err, MihomoError::Service(_)));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn test_spawn_daemon_success_and_kill() {
        use std::os::unix::fs::PermissionsExt;

        let temp = tempdir().expect("tempdir");
        let binary = temp.path().join("mihomo");
        let config = temp.path().join("config.yaml");
        fs::write(&binary, "#!/bin/sh\nsleep 30\n")
            .await
            .expect("write binary");
        fs::set_permissions(&binary, std::fs::Permissions::from_mode(0o755))
            .await
            .expect("chmod");
        fs::write(&config, "port: 7890\n")
            .await
            .expect("write config");

        let pid = spawn_daemon(&binary, &config).await.expect("spawn");
        assert!(is_process_alive(pid));
        kill_process(pid).expect("kill");
    }
}
