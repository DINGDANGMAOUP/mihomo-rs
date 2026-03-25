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
        self.download_version_with_base_url(version, dest, &download_base_url())
            .await
    }

    pub(crate) async fn download_version_with_base_url(
        &self,
        version: &str,
        dest: &Path,
        base_url: &str,
    ) -> Result<()> {
        let platform = Self::detect_platform();
        let os_name = Self::get_os_name();
        let extension = Self::get_file_extension();
        let filename = format!("mihomo-{}-{}-{}.{}", os_name, platform, version, extension);
        let url = format!(
            "{}/MetaCubeX/mihomo/releases/download/{}/{}",
            base_url, version, filename
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

        let decompressed = Self::decompress_by_extension(extension, &bytes)?;

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
        Self::map_os_name(std::env::consts::OS)
    }

    fn map_os_name(os: &str) -> &'static str {
        match os {
            "linux" => "linux",
            "macos" => "darwin",
            "windows" => "windows",
            _ => "linux",
        }
    }

    fn detect_platform() -> String {
        Self::map_platform(std::env::consts::ARCH).to_string()
    }

    fn map_platform(arch: &str) -> &'static str {
        match arch {
            "x86_64" => "amd64",
            "aarch64" => "arm64",
            "arm" => "armv7",
            _ => "amd64",
        }
    }

    fn get_file_extension() -> &'static str {
        Self::file_extension_for_os(std::env::consts::OS)
    }

    fn file_extension_for_os(os: &str) -> &'static str {
        match os {
            "windows" => "zip",
            _ => "gz",
        }
    }

    fn decompress_by_extension(extension: &str, bytes: &[u8]) -> Result<Vec<u8>> {
        if extension == "zip" {
            Self::decompress_zip(bytes)
        } else {
            Self::decompress_gz(bytes)
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

        let mut file = archive
            .by_index(0)
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

fn download_base_url() -> String {
    std::env::var("MIHOMO_DOWNLOAD_BASE_URL").unwrap_or_else(|_| "https://github.com".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::Server;
    use std::sync::{Mutex, OnceLock};

    fn env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    #[test]
    fn test_get_os_name() {
        // Test that get_os_name returns one of the expected values
        let os_name = Downloader::get_os_name();
        assert!(
            os_name == "linux" || os_name == "darwin" || os_name == "windows",
            "OS name should be linux, darwin, or windows, got: {}",
            os_name
        );
    }

    #[test]
    fn test_detect_platform() {
        // Test that detect_platform returns a valid platform string
        let platform = Downloader::detect_platform();
        assert!(
            platform == "amd64" || platform == "arm64" || platform == "armv7",
            "Platform should be amd64, arm64, or armv7, got: {}",
            platform
        );
    }

    #[test]
    fn test_get_file_extension() {
        // Test that get_file_extension returns either zip or gz
        let extension = Downloader::get_file_extension();
        assert!(
            extension == "zip" || extension == "gz",
            "Extension should be zip or gz, got: {}",
            extension
        );
    }

    #[test]
    fn test_map_os_name_unknown_defaults_to_linux() {
        assert_eq!(Downloader::map_os_name("weird-os"), "linux");
    }

    #[test]
    fn test_map_platform_unknown_defaults_to_amd64() {
        assert_eq!(Downloader::map_platform("mips"), "amd64");
    }

    #[test]
    fn test_file_extension_for_windows_and_unknown() {
        assert_eq!(Downloader::file_extension_for_os("windows"), "zip");
        assert_eq!(Downloader::file_extension_for_os("unknown"), "gz");
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn test_windows_uses_zip() {
        assert_eq!(Downloader::get_file_extension(), "zip");
        assert_eq!(Downloader::get_os_name(), "windows");
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn test_linux_uses_gz() {
        assert_eq!(Downloader::get_file_extension(), "gz");
        assert_eq!(Downloader::get_os_name(), "linux");
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_macos_uses_gz() {
        assert_eq!(Downloader::get_file_extension(), "gz");
        assert_eq!(Downloader::get_os_name(), "darwin");
    }

    #[test]
    fn test_decompress_gz() {
        use flate2::write::GzEncoder;
        use flate2::Compression;
        use std::io::Write;

        // Create test data
        let test_data = b"Hello, this is test data for gzip compression!";

        // Compress the data
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(test_data).unwrap();
        let compressed = encoder.finish().unwrap();

        // Test decompression
        let decompressed = Downloader::decompress_gz(&compressed).unwrap();
        assert_eq!(decompressed, test_data);
    }

    #[test]
    fn test_decompress_zip() {
        use std::io::{Cursor, Write};
        use zip::write::SimpleFileOptions;
        use zip::ZipWriter;

        // Create test data
        let test_data = b"Hello, this is test data for zip compression!";

        // Create a zip file in memory with a single entry
        let mut zip_buffer = Cursor::new(Vec::new());
        {
            let mut zip = ZipWriter::new(&mut zip_buffer);
            let options = SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Deflated)
                .unix_permissions(0o755);

            zip.start_file("mihomo", options).unwrap();
            zip.write_all(test_data).unwrap();
            zip.finish().unwrap();
        }

        let compressed = zip_buffer.into_inner();

        // Test decompression
        let decompressed = Downloader::decompress_zip(&compressed).unwrap();
        assert_eq!(decompressed, test_data);
    }

    #[test]
    fn test_decompress_by_extension_zip_branch() {
        use std::io::{Cursor, Write};
        use zip::write::SimpleFileOptions;
        use zip::ZipWriter;

        let mut zip_buffer = Cursor::new(Vec::new());
        {
            let mut zip = ZipWriter::new(&mut zip_buffer);
            let options = SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Deflated)
                .unix_permissions(0o755);
            zip.start_file("mihomo", options).unwrap();
            zip.write_all(b"zip-binary").unwrap();
            zip.finish().unwrap();
        }

        let decompressed = Downloader::decompress_by_extension("zip", &zip_buffer.into_inner())
            .expect("zip branch should decode");
        assert_eq!(decompressed, b"zip-binary");
    }

    #[test]
    fn test_decompress_zip_with_multiple_files_fails() {
        use std::io::{Cursor, Write};
        use zip::write::SimpleFileOptions;
        use zip::ZipWriter;

        // Create a zip file with multiple entries
        let mut zip_buffer = Cursor::new(Vec::new());
        {
            let mut zip = ZipWriter::new(&mut zip_buffer);
            let options =
                SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

            // Add first file
            zip.start_file("file1", options).unwrap();
            zip.write_all(b"File 1 content").unwrap();

            // Add second file
            zip.start_file("file2", options).unwrap();
            zip.write_all(b"File 2 content").unwrap();

            zip.finish().unwrap();
        }

        let compressed = zip_buffer.into_inner();

        // Test that decompression fails with multiple files
        let result = Downloader::decompress_zip(&compressed);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Expected 1 file in zip archive, found 2"));
    }

    #[test]
    fn test_decompress_gz_with_invalid_data() {
        let invalid_data = b"This is not gzip compressed data";
        let result = Downloader::decompress_gz(invalid_data);
        assert!(result.is_err());
    }

    #[test]
    fn test_decompress_zip_with_invalid_data() {
        let invalid_data = b"This is not zip compressed data";
        let result = Downloader::decompress_zip(invalid_data);
        assert!(result.is_err());
    }

    #[test]
    fn test_filename_format() {
        // Test that the filename format is correct for different platforms
        let version = "v1.19.17";
        let platform = Downloader::detect_platform();
        let os_name = Downloader::get_os_name();
        let extension = Downloader::get_file_extension();

        let filename = format!("mihomo-{}-{}-{}.{}", os_name, platform, version, extension);

        // Verify the filename matches expected pattern
        assert!(filename.starts_with("mihomo-"));
        assert!(filename.contains(version));
        assert!(filename.ends_with(".zip") || filename.ends_with(".gz"));
    }

    #[tokio::test]
    async fn test_download_version_with_base_url_http_error() {
        let mut server = Server::new_async().await;
        let version = "v1.19.17";
        let platform = Downloader::detect_platform();
        let os_name = Downloader::get_os_name();
        let extension = Downloader::get_file_extension();
        let filename = format!("mihomo-{}-{}-{}.{}", os_name, platform, version, extension);
        let path = format!(
            "/MetaCubeX/mihomo/releases/download/{}/{}",
            version, filename
        );

        let mock = server
            .mock("GET", path.as_str())
            .with_status(404)
            .create_async()
            .await;

        let temp = tempfile::tempdir().unwrap();
        let dest = temp.path().join("mihomo");
        let downloader = Downloader::new();
        let result = downloader
            .download_version_with_base_url(version, &dest, &server.url())
            .await;

        mock.assert_async().await;
        assert!(result.is_err());
    }

    #[tokio::test]
    #[allow(clippy::await_holding_lock)]
    async fn test_download_version_public_wrapper_uses_env_base_url() {
        let _guard = env_lock().lock().expect("env lock");
        let mut server = Server::new_async().await;
        let version = "v1.19.18";
        let platform = Downloader::detect_platform();
        let os_name = Downloader::get_os_name();
        let extension = Downloader::get_file_extension();
        let filename = format!("mihomo-{}-{}-{}.{}", os_name, platform, version, extension);
        let path = format!(
            "/MetaCubeX/mihomo/releases/download/{}/{}",
            version, filename
        );

        use flate2::write::GzEncoder;
        use flate2::Compression;
        use std::io::Write;
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(b"wrapper-binary").unwrap();
        let gz = encoder.finish().unwrap();

        let mock = server
            .mock("GET", path.as_str())
            .with_status(200)
            .with_body(gz)
            .create_async()
            .await;

        let old = std::env::var("MIHOMO_DOWNLOAD_BASE_URL").ok();
        // SAFETY: env updates are serialized in this module via env_lock.
        unsafe { std::env::set_var("MIHOMO_DOWNLOAD_BASE_URL", server.url()) };

        let temp = tempfile::tempdir().unwrap();
        let dest = temp.path().join("mihomo");
        let downloader = Downloader::default();
        downloader.download_version(version, &dest).await.unwrap();

        mock.assert_async().await;
        assert!(dest.exists());

        if let Some(prev) = old {
            // SAFETY: env updates are serialized in this module via env_lock.
            unsafe { std::env::set_var("MIHOMO_DOWNLOAD_BASE_URL", prev) };
        } else {
            // SAFETY: env updates are serialized in this module via env_lock.
            unsafe { std::env::remove_var("MIHOMO_DOWNLOAD_BASE_URL") };
        }
    }

    #[tokio::test]
    #[cfg(not(target_os = "windows"))]
    async fn test_download_version_with_base_url_gz_success() {
        use flate2::write::GzEncoder;
        use flate2::Compression;
        use std::io::Write;

        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(b"fake-binary").unwrap();
        let gz = encoder.finish().unwrap();

        let mut server = Server::new_async().await;
        let version = "v1.19.17";
        let platform = Downloader::detect_platform();
        let os_name = Downloader::get_os_name();
        let filename = format!("mihomo-{}-{}-{}.gz", os_name, platform, version);
        let path = format!(
            "/MetaCubeX/mihomo/releases/download/{}/{}",
            version, filename
        );

        let mock = server
            .mock("GET", path.as_str())
            .with_status(200)
            .with_body(gz)
            .create_async()
            .await;

        let temp = tempfile::tempdir().unwrap();
        let dest = temp.path().join("mihomo");
        let downloader = Downloader::new();
        downloader
            .download_version_with_base_url(version, &dest, &server.url())
            .await
            .unwrap();

        mock.assert_async().await;
        assert!(dest.exists());
    }
}
