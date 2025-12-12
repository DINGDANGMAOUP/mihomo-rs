use crate::core::{MihomoError, Result};
use std::path::Path;
use tokio::fs;
use tokio::io::AsyncWriteExt;

pub struct Downloader {
    client: reqwest::Client,
}

impl Downloader {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }

    pub async fn download_version(&self, version: &str, dest: &Path) -> Result<()> {
        let platform = Self::detect_platform();
        let filename = format!("mihomo-{}-{}-{}.gz", Self::get_os_name(), platform, version);
        let url = format!(
            "https://github.com/MetaCubeX/mihomo/releases/download/{}/{}",
            version, filename
        );

        let resp = self
            .client
            .get(&url)
            .header("User-Agent", "mihomo-rs")
            .send()
            .await?;

        if !resp.status().is_success() {
            return Err(MihomoError::Version(format!(
                "Failed to download version {}: HTTP {}",
                version,
                resp.status()
            )));
        }

        let bytes = resp.bytes().await?;

        use flate2::read::GzDecoder;
        use std::io::Read;
        let mut decoder = GzDecoder::new(&bytes[..]);
        let mut decompressed = Vec::new();
        decoder
            .read_to_end(&mut decompressed)
            .map_err(|e| MihomoError::Version(format!("Failed to decompress: {}", e)))?;

        let mut file = fs::File::create(dest).await?;
        file.write_all(&decompressed).await?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = file.metadata().await?.permissions();
            perms.set_mode(0o755);
            fs::set_permissions(dest, perms).await?;
        }

        Ok(())
    }

    fn get_os_name() -> &'static str {
        match std::env::consts::OS {
            "linux" => "linux",
            "macos" => "darwin",
            "windows" => "windows",
            _ => "linux",
        }
    }

    fn detect_platform() -> String {
        let arch = std::env::consts::ARCH;
        match arch {
            "x86_64" => "amd64",
            "aarch64" => "arm64",
            "arm" => "armv7",
            _ => "amd64",
        }
        .to_string()
    }
}

impl Default for Downloader {
    fn default() -> Self {
        Self::new()
    }
}
