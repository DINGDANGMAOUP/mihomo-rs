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

#[cfg(test)]
mod tests {
    use super::*;

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
}
