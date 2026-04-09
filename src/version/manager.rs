use super::channel::{fetch_latest, Channel};
use super::download::Downloader;
use crate::core::{get_home_dir, MihomoError, Result};
use semver::Version;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering as CmpOrdering;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering as AtomicOrdering};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::fs;

static TEMP_FILE_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionInfo {
    pub version: String,
    pub path: PathBuf,
    pub is_default: bool,
}

pub struct VersionManager {
    install_dir: PathBuf,
    config_file: PathBuf,
}

impl VersionManager {
    pub fn new() -> Result<Self> {
        let home = get_home_dir()?;
        Self::with_home(home)
    }

    pub fn with_home(home: PathBuf) -> Result<Self> {
        let install_dir = home.join("versions");
        let config_file = home.join("config.toml");

        Ok(Self {
            install_dir,
            config_file,
        })
    }

    pub async fn install(&self, version: &str) -> Result<()> {
        fs::create_dir_all(&self.install_dir).await?;

        let version_dir = self.install_dir.join(version);
        if version_dir.exists() {
            return Err(MihomoError::Version(format!(
                "Version {} is already installed",
                version
            )));
        }

        let binary_name = if cfg!(windows) {
            "mihomo.exe"
        } else {
            "mihomo"
        };

        // Download to a temp file under install_dir to reduce cross-device rename failures.
        let temp_path = self.temp_download_path(version, binary_name);

        let downloader = Downloader::new();
        if let Err(err) = downloader.download_version(version, &temp_path).await {
            let _ = fs::remove_file(&temp_path).await;
            return Err(err);
        }

        // Move to final location only after successful download
        fs::create_dir_all(&version_dir).await?;
        let binary_path = version_dir.join(binary_name);
        if let Err(err) = fs::rename(&temp_path, &binary_path).await {
            if err.kind() == std::io::ErrorKind::CrossesDevices {
                if let Err(copy_err) = fs::copy(&temp_path, &binary_path).await {
                    let _ = fs::remove_file(&temp_path).await;
                    return Err(copy_err.into());
                }
                let _ = fs::remove_file(&temp_path).await;
            } else {
                let _ = fs::remove_file(&temp_path).await;
                return Err(err.into());
            }
        }

        Ok(())
    }

    fn temp_download_path(&self, version: &str, binary_name: &str) -> PathBuf {
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let seq = TEMP_FILE_COUNTER.fetch_add(1, AtomicOrdering::Relaxed);
        self.install_dir.join(format!(
            ".mihomo-{}-{}-{}-{}-{}.downloading",
            version,
            std::process::id(),
            ts,
            seq,
            binary_name
        ))
    }

    pub async fn install_channel(&self, channel: Channel) -> Result<String> {
        let info = fetch_latest(channel).await?;
        self.install(&info.version).await?;
        Ok(info.version)
    }

    pub async fn list_installed(&self) -> Result<Vec<VersionInfo>> {
        if !self.install_dir.exists() {
            return Ok(vec![]);
        }

        let mut versions = vec![];
        let default_version = self.get_default().await.ok();

        let mut entries = fs::read_dir(&self.install_dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            if entry.file_type().await?.is_dir() {
                let version = entry.file_name().to_string_lossy().to_string();
                let is_default = default_version.as_ref() == Some(&version);
                versions.push(VersionInfo {
                    version,
                    path: entry.path(),
                    is_default,
                });
            }
        }

        versions.sort_by(|a, b| {
            match (
                Self::parse_semver(&a.version),
                Self::parse_semver(&b.version),
            ) {
                (Some(av), Some(bv)) => bv.cmp(&av),
                (Some(_), None) => CmpOrdering::Less,
                (None, Some(_)) => CmpOrdering::Greater,
                (None, None) => b.version.cmp(&a.version),
            }
        });
        Ok(versions)
    }

    fn parse_semver(raw: &str) -> Option<Version> {
        Version::parse(raw.trim_start_matches('v')).ok()
    }

    pub async fn set_default(&self, version: &str) -> Result<()> {
        let version_dir = self.install_dir.join(version);
        if !version_dir.exists() {
            return Err(MihomoError::NotFound(format!(
                "Version {} is not installed",
                version
            )));
        }

        if let Some(parent) = self.config_file.parent() {
            fs::create_dir_all(parent).await?;
        }

        let mut config = if self.config_file.exists() {
            let content = fs::read_to_string(&self.config_file).await?;
            toml::from_str::<toml::Value>(&content)
                .map_err(|e| MihomoError::Config(format!("Invalid config: {}", e)))?
        } else {
            toml::Value::Table(toml::map::Map::new())
        };

        if let toml::Value::Table(ref mut table) = config {
            let default_table = table
                .entry("default".to_string())
                .or_insert_with(|| toml::Value::Table(toml::map::Map::new()));
            if let toml::Value::Table(ref mut default) = default_table {
                default.insert(
                    "version".to_string(),
                    toml::Value::String(version.to_string()),
                );
            }
        }

        let content = toml::to_string(&config)
            .map_err(|e| MihomoError::Config(format!("Failed to serialize config: {}", e)))?;
        fs::write(&self.config_file, content).await?;

        Ok(())
    }

    pub async fn get_default(&self) -> Result<String> {
        if !self.config_file.exists() {
            return Err(MihomoError::NotFound("No default version set".to_string()));
        }

        let content = fs::read_to_string(&self.config_file).await?;
        let config: toml::Value = toml::from_str(&content)
            .map_err(|e| MihomoError::Config(format!("Invalid config: {}", e)))?;

        config
            .get("default")
            .and_then(|d| d.get("version"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| MihomoError::Config("No default version in config".to_string()))
    }

    pub async fn get_binary_path(&self, version: Option<&str>) -> Result<PathBuf> {
        let version = if let Some(v) = version {
            v.to_string()
        } else {
            self.get_default().await?
        };

        let binary_name = if cfg!(windows) {
            "mihomo.exe"
        } else {
            "mihomo"
        };

        let path = self.install_dir.join(&version).join(binary_name);
        if !path.exists() {
            return Err(MihomoError::NotFound(format!(
                "Binary not found for version {}",
                version
            )));
        }

        Ok(path)
    }

    pub async fn uninstall(&self, version: &str) -> Result<()> {
        let version_dir = self.install_dir.join(version);
        if !version_dir.exists() {
            return Err(MihomoError::NotFound(format!(
                "Version {} is not installed",
                version
            )));
        }

        let default_version = match self.get_default().await {
            Ok(v) => Some(v),
            Err(MihomoError::NotFound(_)) => None,
            Err(err) => return Err(err),
        };
        if default_version.as_ref() == Some(&version.to_string()) {
            return Err(MihomoError::Version(
                "Cannot uninstall the default version".to_string(),
            ));
        }

        fs::remove_dir_all(version_dir).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::VersionManager;
    use std::path::PathBuf;
    use tempfile::tempdir;
    use tokio::fs;

    #[test]
    fn test_temp_download_path_is_unique() {
        let vm = VersionManager::with_home(PathBuf::from("/tmp/mihomo-rs-test-home"))
            .expect("version manager should be created");

        let path1 = vm.temp_download_path("v1.0.0", "mihomo");
        let path2 = vm.temp_download_path("v1.0.0", "mihomo");

        assert_ne!(path1, path2);
    }

    #[tokio::test]
    async fn test_list_installed_uses_semver_order() {
        let temp = tempdir().expect("create temp dir");
        let vm = VersionManager::with_home(temp.path().to_path_buf())
            .expect("version manager should be created");

        fs::create_dir_all(vm.install_dir.join("v1.9.0"))
            .await
            .expect("create v1.9.0");
        fs::create_dir_all(vm.install_dir.join("v1.10.0"))
            .await
            .expect("create v1.10.0");
        fs::create_dir_all(vm.install_dir.join("v1.2.0"))
            .await
            .expect("create v1.2.0");

        let names: Vec<String> = vm
            .list_installed()
            .await
            .expect("list installed")
            .into_iter()
            .map(|v| v.version)
            .collect();

        assert_eq!(
            names,
            vec![
                "v1.10.0".to_string(),
                "v1.9.0".to_string(),
                "v1.2.0".to_string(),
            ]
        );
    }
}
