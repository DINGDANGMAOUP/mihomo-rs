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
        let os_name = Self::get_os_name();
        let extension = Self::get_file_extension();
        let filename = format!("mihomo-{}-{}-{}.{}", os_name, platform, version, extension);
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

        // Decompress based on file extension
        let decompressed = if extension == "zip" {
            Self::decompress_zip(&bytes)?
        } else {
            Self::decompress_gz(&bytes)?
        };

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

    fn get_file_extension() -> &'static str {
        match std::env::consts::OS {
            "windows" => "zip",
            _ => "gz",
        }
    }

    fn decompress_gz(bytes: &[u8]) -> Result<Vec<u8>> {
        use flate2::read::GzDecoder;
        use std::io::Read;

        let mut decoder = GzDecoder::new(bytes);
        let mut decompressed = Vec::new();
        decoder
            .read_to_end(&mut decompressed)
            .map_err(|e| MihomoError::Version(format!("Failed to decompress gz: {}", e)))?;
        Ok(decompressed)
    }

    fn decompress_zip(bytes: &[u8]) -> Result<Vec<u8>> {
        use std::io::{Cursor, Read};
        use zip::ZipArchive;

        let reader = Cursor::new(bytes);
        let mut archive = ZipArchive::new(reader)
            .map_err(|e| MihomoError::Version(format!("Failed to open zip archive: {}", e)))?;

        // mihomo zip archives should contain a single binary file
        if archive.len() != 1 {
            return Err(MihomoError::Version(format!(
                "Expected 1 file in zip archive, found {}",
                archive.len()
            )));
        }

        let mut file = archive.by_index(0)
            .map_err(|e| MihomoError::Version(format!("Failed to read zip entry: {}", e)))?;

        let mut decompressed = Vec::new();
        file.read_to_end(&mut decompressed)
            .map_err(|e| MihomoError::Version(format!("Failed to decompress zip: {}", e)))?;

        Ok(decompressed)
    }
}

impl Default for Downloader {
    fn default() -> Self {
        Self::new()
    }
}
