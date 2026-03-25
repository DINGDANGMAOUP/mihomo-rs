use super::channel::{fetch_latest_with_client, Channel};
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
        self.install_with_base_url(version, &download_base_url())
            .await
    }

    async fn install_with_base_url(&self, version: &str, download_base_url: &str) -> Result<()> {
        fs::create_dir_all(&self.install_dir).await?;

        let version_dir = self.install_dir.join(version);
        if version_dir.exists() {
            return Err(MihomoError::Version(format!(
                "Version {} is already installed",
                version
            )));
        }

        let binary_name = Self::binary_name();

        // Download to OS temp directory first
        let temp_dir = std::env::temp_dir();
        let temp_path = temp_dir.join(format!("mihomo-{}-{}", version, binary_name));

        let downloader = Downloader::new();
        downloader
            .download_version_with_base_url(version, &temp_path, download_base_url)
            .await?;

        // Move to final location only after successful download
        fs::create_dir_all(&version_dir).await?;
        let binary_path = version_dir.join(binary_name);
        fs::rename(&temp_path, &binary_path).await?;

        Ok(())
    }

    pub async fn install_channel(&self, channel: Channel) -> Result<String> {
        self.install_channel_with_base_urls(channel, &api_base_url(), &download_base_url())
            .await
    }

    async fn install_channel_with_base_urls(
        &self,
        channel: Channel,
        api_base_url: &str,
        download_base_url: &str,
    ) -> Result<String> {
        let client = reqwest::Client::new();
        let info = fetch_latest_with_client(channel, &client, api_base_url).await?;
        self.install_with_base_url(&info.version, download_base_url)
            .await?;
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

        if let Some(parent) = self.config_file.parent() {
            fs::create_dir_all(parent).await?;
        }

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

        let binary_name = Self::binary_name();

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

    fn binary_name() -> &'static str {
        Self::binary_name_for_os(std::env::consts::OS)
    }

    fn binary_name_for_os(os: &str) -> &'static str {
        if os == "windows" {
            "mihomo.exe"
        } else {
            "mihomo"
        }
    }
}

fn api_base_url() -> String {
    std::env::var("MIHOMO_API_BASE_URL").unwrap_or_else(|_| "https://api.github.com".to_string())
}

fn download_base_url() -> String {
    std::env::var("MIHOMO_DOWNLOAD_BASE_URL").unwrap_or_else(|_| "https://github.com".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::write::GzEncoder;
    use flate2::Compression;
    use mockito::Server;
    use std::io::Write;
    use std::sync::{Mutex, OnceLock};
    use tempfile::tempdir;

    fn env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    fn expected_binary_path(home: &std::path::Path, version: &str) -> PathBuf {
        let binary_name = if cfg!(windows) {
            "mihomo.exe"
        } else {
            "mihomo"
        };
        home.join("versions").join(version).join(binary_name)
    }

    fn expected_platform() -> &'static str {
        match std::env::consts::ARCH {
            "x86_64" => "amd64",
            "aarch64" => "arm64",
            "arm" => "armv7",
            _ => "amd64",
        }
    }

    fn expected_os_name() -> &'static str {
        match std::env::consts::OS {
            "linux" => "linux",
            "macos" => "darwin",
            "windows" => "windows",
            _ => "linux",
        }
    }

    fn expected_extension() -> &'static str {
        if cfg!(windows) {
            "zip"
        } else {
            "gz"
        }
    }

    #[test]
    fn binary_name_for_os_windows_and_unix() {
        assert_eq!(VersionManager::binary_name_for_os("windows"), "mihomo.exe");
        assert_eq!(VersionManager::binary_name_for_os("linux"), "mihomo");
    }

    #[tokio::test]
    async fn install_with_base_url_download_error() {
        let temp = tempdir().unwrap();
        let vm = VersionManager::with_home(temp.path().to_path_buf()).unwrap();
        let version = "v9.9.9";
        let platform = expected_platform();
        let os_name = expected_os_name();
        let extension = expected_extension();
        let filename = format!("mihomo-{}-{}-{}.{}", os_name, platform, version, extension);
        let path = format!(
            "/MetaCubeX/mihomo/releases/download/{}/{}",
            version, filename
        );

        let mut server = Server::new_async().await;
        let mock = server
            .mock("GET", path.as_str())
            .with_status(404)
            .create_async()
            .await;

        let err = vm
            .install_with_base_url(version, &server.url())
            .await
            .unwrap_err();

        mock.assert_async().await;
        assert!(matches!(err, MihomoError::Version(_)));
    }

    #[tokio::test]
    #[cfg(not(target_os = "windows"))]
    async fn install_with_base_url_success_non_windows() {
        let temp = tempdir().unwrap();
        let vm = VersionManager::with_home(temp.path().to_path_buf()).unwrap();
        let version = "v9.9.8";
        let platform = expected_platform();
        let os_name = expected_os_name();
        let filename = format!("mihomo-{}-{}-{}.gz", os_name, platform, version);
        let path = format!(
            "/MetaCubeX/mihomo/releases/download/{}/{}",
            version, filename
        );

        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(b"mihomo-bin").unwrap();
        let gz = encoder.finish().unwrap();

        let mut server = Server::new_async().await;
        let mock = server
            .mock("GET", path.as_str())
            .with_status(200)
            .with_body(gz)
            .create_async()
            .await;

        vm.install_with_base_url(version, &server.url())
            .await
            .unwrap();

        mock.assert_async().await;
        assert!(expected_binary_path(temp.path(), version).exists());
    }

    #[tokio::test]
    #[cfg(not(target_os = "windows"))]
    async fn install_channel_with_base_urls_success() {
        let temp = tempdir().unwrap();
        let vm = VersionManager::with_home(temp.path().to_path_buf()).unwrap();
        let version = "v9.9.7";
        let platform = expected_platform();
        let os_name = expected_os_name();
        let filename = format!("mihomo-{}-{}-{}.gz", os_name, platform, version);
        let download_path = format!(
            "/MetaCubeX/mihomo/releases/download/{}/{}",
            version, filename
        );

        let mut api = Server::new_async().await;
        let latest_mock = api
            .mock("GET", "/repos/MetaCubeX/mihomo/releases/latest")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(format!(
                r#"{{"tag_name":"{}","published_at":"2026-03-25T12:00:00Z"}}"#,
                version
            ))
            .create_async()
            .await;

        let mut dl = Server::new_async().await;
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(b"mihomo-bin-channel").unwrap();
        let gz = encoder.finish().unwrap();
        let download_mock = dl
            .mock("GET", download_path.as_str())
            .with_status(200)
            .with_body(gz)
            .create_async()
            .await;

        let installed = vm
            .install_channel_with_base_urls(Channel::Stable, &api.url(), &dl.url())
            .await
            .unwrap();

        latest_mock.assert_async().await;
        download_mock.assert_async().await;
        assert_eq!(installed, version);
        assert!(expected_binary_path(temp.path(), version).exists());
    }

    #[tokio::test]
    #[cfg(not(target_os = "windows"))]
    #[allow(clippy::await_holding_lock)]
    async fn install_channel_public_uses_env_base_urls() {
        let _guard = env_lock().lock().expect("env lock");
        let temp = tempdir().unwrap();
        let vm = VersionManager::with_home(temp.path().to_path_buf()).unwrap();
        let version = "v9.9.6";
        let platform = expected_platform();
        let os_name = expected_os_name();
        let filename = format!("mihomo-{}-{}-{}.gz", os_name, platform, version);
        let download_path = format!(
            "/MetaCubeX/mihomo/releases/download/{}/{}",
            version, filename
        );

        let mut api = Server::new_async().await;
        let latest_mock = api
            .mock("GET", "/repos/MetaCubeX/mihomo/releases/latest")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(format!(
                r#"{{"tag_name":"{}","published_at":"2026-03-26T12:00:00Z"}}"#,
                version
            ))
            .create_async()
            .await;

        let mut dl = Server::new_async().await;
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(b"mihomo-public-wrapper").unwrap();
        let gz = encoder.finish().unwrap();
        let download_mock = dl
            .mock("GET", download_path.as_str())
            .with_status(200)
            .with_body(gz)
            .create_async()
            .await;

        let old_api = std::env::var("MIHOMO_API_BASE_URL").ok();
        let old_dl = std::env::var("MIHOMO_DOWNLOAD_BASE_URL").ok();
        // SAFETY: env updates are serialized in this module via env_lock.
        unsafe {
            std::env::set_var("MIHOMO_API_BASE_URL", api.url());
            std::env::set_var("MIHOMO_DOWNLOAD_BASE_URL", dl.url());
        }

        let installed = vm.install_channel(Channel::Stable).await.unwrap();
        latest_mock.assert_async().await;
        download_mock.assert_async().await;
        assert_eq!(installed, version);
        assert!(expected_binary_path(temp.path(), version).exists());

        if let Some(prev) = old_api {
            // SAFETY: env updates are serialized in this module via env_lock.
            unsafe { std::env::set_var("MIHOMO_API_BASE_URL", prev) };
        } else {
            // SAFETY: env updates are serialized in this module via env_lock.
            unsafe { std::env::remove_var("MIHOMO_API_BASE_URL") };
        }
        if let Some(prev) = old_dl {
            // SAFETY: env updates are serialized in this module via env_lock.
            unsafe { std::env::set_var("MIHOMO_DOWNLOAD_BASE_URL", prev) };
        } else {
            // SAFETY: env updates are serialized in this module via env_lock.
            unsafe { std::env::remove_var("MIHOMO_DOWNLOAD_BASE_URL") };
        }
    }
}
