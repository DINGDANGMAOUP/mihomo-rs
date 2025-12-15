use super::channel::{fetch_latest, Channel};
use super::download::Downloader;
use crate::core::{get_home_dir, MihomoError, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::fs;

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

        // Download to OS temp directory first
        let temp_dir = std::env::temp_dir();
        let temp_path = temp_dir.join(format!("mihomo-{}-{}", version, binary_name));

        let downloader = Downloader::new();
        downloader.download_version(version, &temp_path).await?;

        // Move to final location only after successful download
        fs::create_dir_all(&version_dir).await?;
        let binary_path = version_dir.join(binary_name);
        fs::rename(&temp_path, &binary_path).await?;

        Ok(())
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

        versions.sort_by(|a, b| b.version.cmp(&a.version));
        Ok(versions)
    }

    pub async fn set_default(&self, version: &str) -> Result<()> {
        let version_dir = self.install_dir.join(version);
        if !version_dir.exists() {
            return Err(MihomoError::NotFound(format!(
                "Version {} is not installed",
                version
            )));
        }

        fs::create_dir_all(self.config_file.parent().unwrap()).await?;

        let config = format!("[default]\nversion = \"{}\"\n", version);
        fs::write(&self.config_file, config).await?;

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

        let default_version = self.get_default().await.ok();
        if default_version.as_ref() == Some(&version.to_string()) {
            return Err(MihomoError::Version(
                "Cannot uninstall the default version".to_string(),
            ));
        }

        fs::remove_dir_all(version_dir).await?;
        Ok(())
    }
}
