use super::profile::Profile;
use crate::core::{
    find_available_port, get_home_dir, is_port_available, validate_profile_name, ErrorCode,
    MihomoError, Result,
};
use serde::Deserialize;
use std::path::{Path, PathBuf};
use tokio::fs;
use url::Url;

pub struct ConfigManager {
    config_dir: PathBuf,
    settings_file: PathBuf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigDirSource {
    Env,
    Settings,
    Default,
}

impl ConfigDirSource {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Env => "env",
            Self::Settings => "config.toml",
            Self::Default => "default",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigDirInfo {
    pub path: PathBuf,
    pub source: ConfigDirSource,
}

#[derive(Debug, Default, Deserialize)]
struct ConfigSettings {
    #[serde(default)]
    paths: PathSettings,
}

#[derive(Debug, Default, Deserialize)]
struct PathSettings {
    configs_dir: Option<String>,
}

impl ConfigManager {
    fn invalid_external_controller_error(controller: &str) -> MihomoError {
        MihomoError::config_with_code(
            ErrorCode::InvalidExternalController,
            format!("Invalid external-controller value '{}'", controller),
        )
    }

    pub fn new() -> Result<Self> {
        let home = get_home_dir()?;
        Self::with_home(home)
    }

    pub fn with_home(home: PathBuf) -> Result<Self> {
        let config_dir = home.join("configs");
        let settings_file = home.join("config.toml");

        Ok(Self {
            config_dir,
            settings_file,
        })
    }

    fn home_dir(&self) -> PathBuf {
        self.settings_file
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .to_path_buf()
    }

    fn expand_tilde(path: &str) -> Result<PathBuf> {
        if path == "~" {
            return dirs::home_dir()
                .ok_or_else(|| MihomoError::config("Could not determine home directory"));
        }

        if let Some(suffix) = path.strip_prefix("~/") {
            return dirs::home_dir()
                .ok_or_else(|| MihomoError::config("Could not determine home directory"))
                .map(|home| home.join(suffix));
        }

        Ok(PathBuf::from(path))
    }

    fn normalize_configs_dir(&self, raw_path: &str) -> Result<PathBuf> {
        let trimmed = raw_path.trim();
        if trimmed.is_empty() {
            return Err(MihomoError::config(
                "Invalid configs directory: path is empty",
            ));
        }

        let path = Self::expand_tilde(trimmed)?;
        if path.is_absolute() {
            Ok(path)
        } else {
            Ok(self.home_dir().join(path))
        }
    }

    fn resolve_config_dir_info(&self) -> Result<ConfigDirInfo> {
        if let Ok(dir) = std::env::var("MIHOMO_CONFIGS_DIR") {
            return Ok(ConfigDirInfo {
                path: self.normalize_configs_dir(&dir)?,
                source: ConfigDirSource::Env,
            });
        }

        if !self.settings_file.exists() {
            return Ok(ConfigDirInfo {
                path: self.config_dir.clone(),
                source: ConfigDirSource::Default,
            });
        }

        let content = match std::fs::read_to_string(&self.settings_file) {
            Ok(content) => content,
            Err(err) => {
                return Err(MihomoError::config(format!(
                    "Failed to read config: {}",
                    err
                )));
            }
        };

        let settings: ConfigSettings = match toml::from_str(&content) {
            Ok(settings) => settings,
            Err(err) => {
                log::warn!(
                    "Failed to parse settings file '{}': {}, falling back to default configs dir",
                    self.settings_file.display(),
                    err
                );
                return Ok(ConfigDirInfo {
                    path: self.config_dir.clone(),
                    source: ConfigDirSource::Default,
                });
            }
        };

        if let Some(path) = settings.paths.configs_dir.as_deref() {
            return Ok(ConfigDirInfo {
                path: self.normalize_configs_dir(path)?,
                source: ConfigDirSource::Settings,
            });
        }

        Ok(ConfigDirInfo {
            path: self.config_dir.clone(),
            source: ConfigDirSource::Default,
        })
    }

    fn resolve_config_dir(&self) -> Result<PathBuf> {
        Ok(self.resolve_config_dir_info()?.path)
    }

    async fn read_settings_value(&self) -> Result<toml::Value> {
        if self.settings_file.exists() {
            let content = fs::read_to_string(&self.settings_file).await?;
            toml::from_str(&content)
                .map_err(|e| MihomoError::config(format!("Invalid config: {}", e)))
        } else {
            Ok(toml::Value::Table(toml::map::Map::new()))
        }
    }

    async fn write_settings_value(&self, config: &toml::Value) -> Result<()> {
        if let Some(parent) = self.settings_file.parent() {
            fs::create_dir_all(parent).await?;
        }

        let content = toml::to_string(config)
            .map_err(|e| MihomoError::config(format!("Failed to serialize config: {}", e)))?;
        fs::write(&self.settings_file, content).await?;
        Ok(())
    }

    pub fn get_config_dir_info(&self) -> Result<ConfigDirInfo> {
        self.resolve_config_dir_info()
    }

    pub async fn set_configs_dir(&self, path: &str) -> Result<PathBuf> {
        let trimmed = path.trim();
        let _ = self.normalize_configs_dir(trimmed)?;

        let mut config = self.read_settings_value().await?;
        if let toml::Value::Table(ref mut table) = config {
            let paths_table = table
                .entry("paths".to_string())
                .or_insert_with(|| toml::Value::Table(toml::map::Map::new()));

            if let toml::Value::Table(ref mut paths) = paths_table {
                paths.insert(
                    "configs_dir".to_string(),
                    toml::Value::String(trimmed.to_string()),
                );
            }
        }

        self.write_settings_value(&config).await?;
        self.resolve_config_dir()
    }

    pub async fn unset_configs_dir(&self) -> Result<PathBuf> {
        let mut config = self.read_settings_value().await?;

        if let toml::Value::Table(ref mut table) = config {
            let mut remove_paths_table = false;
            if let Some(toml::Value::Table(paths)) = table.get_mut("paths") {
                paths.remove("configs_dir");
                remove_paths_table = paths.is_empty();
            }

            if remove_paths_table {
                table.remove("paths");
            }
        }

        self.write_settings_value(&config).await?;
        self.resolve_config_dir()
    }

    pub async fn load(&self, profile: &str) -> Result<String> {
        validate_profile_name(profile)?;
        let config_dir = self.resolve_config_dir()?;
        let path = config_dir.join(format!("{}.yaml", profile));
        if !path.exists() {
            return Err(MihomoError::NotFound(format!(
                "Profile '{}' not found",
                profile
            )));
        }

        let content = fs::read_to_string(&path).await?;
        Ok(content)
    }

    pub async fn save(&self, profile: &str, content: &str) -> Result<()> {
        validate_profile_name(profile)?;
        let config_dir = self.resolve_config_dir()?;
        fs::create_dir_all(&config_dir).await?;

        serde_yaml::from_str::<serde_yaml::Value>(content)?;

        let path = config_dir.join(format!("{}.yaml", profile));
        fs::write(&path, content).await?;

        Ok(())
    }

    pub async fn list_profiles(&self) -> Result<Vec<Profile>> {
        let config_dir = self.resolve_config_dir()?;
        if !config_dir.exists() {
            return Ok(vec![]);
        }

        let current = self.get_current().await.ok();
        let mut profiles = vec![];

        let mut entries = fs::read_dir(&config_dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("yaml") {
                let name = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("")
                    .to_string();
                let active = current.as_ref() == Some(&name);
                profiles.push(Profile::new(name, path, active));
            }
        }

        profiles.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(profiles)
    }

    pub async fn delete_profile(&self, profile: &str) -> Result<()> {
        validate_profile_name(profile)?;
        let config_dir = self.resolve_config_dir()?;
        let path = config_dir.join(format!("{}.yaml", profile));
        if !path.exists() {
            return Err(MihomoError::NotFound(format!(
                "Profile '{}' not found",
                profile
            )));
        }

        let current = self.get_current().await.ok();
        if current.as_ref() == Some(&profile.to_string()) {
            return Err(MihomoError::config("Cannot delete the active profile"));
        }

        fs::remove_file(path).await?;
        Ok(())
    }

    pub async fn set_current(&self, profile: &str) -> Result<()> {
        validate_profile_name(profile)?;
        let config_dir = self.resolve_config_dir()?;
        let path = config_dir.join(format!("{}.yaml", profile));
        if !path.exists() {
            return Err(MihomoError::NotFound(format!(
                "Profile '{}' not found",
                profile
            )));
        }

        let mut config = self.read_settings_value().await?;

        if let toml::Value::Table(ref mut table) = config {
            let default_table = table
                .entry("default".to_string())
                .or_insert_with(|| toml::Value::Table(toml::map::Map::new()));

            if let toml::Value::Table(ref mut default) = default_table {
                default.insert(
                    "profile".to_string(),
                    toml::Value::String(profile.to_string()),
                );
            }
        }

        self.write_settings_value(&config).await?;

        Ok(())
    }

    pub async fn get_current(&self) -> Result<String> {
        if !self.settings_file.exists() {
            return Ok("default".to_string());
        }

        let content = fs::read_to_string(&self.settings_file).await?;
        let config: toml::Value = toml::from_str(&content)
            .map_err(|e| MihomoError::config(format!("Invalid config: {}", e)))?;

        Ok(config
            .get("default")
            .and_then(|d| d.get("profile"))
            .and_then(|p| p.as_str())
            .unwrap_or("default")
            .to_string())
    }

    pub async fn get_current_path(&self) -> Result<PathBuf> {
        let profile = self.get_current().await?;
        validate_profile_name(&profile)?;
        Ok(self.resolve_config_dir()?.join(format!("{}.yaml", profile)))
    }

    /// Ensure a default config file exists, create one if it doesn't
    pub async fn ensure_default_config(&self) -> Result<()> {
        let profile = self.get_current().await?;
        validate_profile_name(&profile)?;
        let path = self.resolve_config_dir()?.join(format!("{}.yaml", profile));

        if !path.exists() {
            log::info!("Default config '{}' not found, creating...", profile);

            let port = find_available_port(9090).ok_or_else(|| {
                MihomoError::config("No available ports found in range 9090-9190")
            })?;

            let default_config = format!(
                r#"# mihomo configuration
port: 7890
socks-port: 7891
allow-lan: false
mode: rule
log-level: info
external-controller: 127.0.0.1:{}
"#,
                port
            );

            self.save(&profile, &default_config).await?;
            log::info!("Created default config at: {}", path.display());
        }

        Ok(())
    }

    pub async fn get_external_controller(&self) -> Result<String> {
        let profile = self.get_current().await?;
        log::debug!("Reading external-controller from profile: {}", profile);

        let content = self.load(&profile).await?;
        let config: serde_yaml::Value = serde_yaml::from_str(&content)?;

        let controller = config
            .get("external-controller")
            .and_then(|v| v.as_str())
            .unwrap_or("127.0.0.1:9090");

        let url = Self::normalize_external_controller(controller)?;

        log::debug!("External controller URL: {}", url);
        Ok(url)
    }

    /// Ensure external-controller is configured in the current profile
    /// If not present or port is occupied, add/update it with an available port
    pub async fn ensure_external_controller(&self) -> Result<String> {
        let profile = self.get_current().await?;
        let content = self.load(&profile).await?;
        let mut config: serde_yaml::Value = serde_yaml::from_str(&content)?;

        let needs_update = match config.get("external-controller").and_then(|v| v.as_str()) {
            Some(controller) => {
                if controller.starts_with('/') || controller.starts_with("unix://") {
                    false
                } else {
                    let has_explicit_scheme =
                        controller.starts_with("http://") || controller.starts_with("https://");
                    if !has_explicit_scheme
                        && !controller.starts_with(':')
                        && !controller.contains(':')
                    {
                        log::warn!(
                            "Invalid external-controller value without port: {}",
                            controller
                        );
                        true
                    } else {
                        match Self::normalize_external_controller(controller)
                            .ok()
                            .and_then(|normalized| Url::parse(&normalized).ok())
                        {
                            Some(url) => {
                                let host = url.host_str().unwrap_or_default();
                                let port = url.port_or_known_default();
                                if Self::is_local_controller_host(host) {
                                    match port {
                                        Some(p) => !is_port_available(p),
                                        None => true,
                                    }
                                } else {
                                    false
                                }
                            }
                            None => {
                                log::warn!("Invalid external-controller value: {}", controller);
                                true
                            }
                        }
                    }
                }
            }
            None => {
                log::info!("external-controller not found in config, adding default");
                true
            }
        };

        if needs_update {
            let port = find_available_port(9090).ok_or_else(|| {
                MihomoError::config("No available ports found in range 9090-9190")
            })?;

            let controller_addr = format!("127.0.0.1:{}", port);
            log::info!("Setting external-controller to {}", controller_addr);

            if let serde_yaml::Value::Mapping(ref mut map) = config {
                map.insert(
                    serde_yaml::Value::String("external-controller".to_string()),
                    serde_yaml::Value::String(controller_addr.clone()),
                );
            }

            let updated_content = serde_yaml::to_string(&config)?;
            self.save(&profile, &updated_content).await?;

            Ok(format!("http://{}", controller_addr))
        } else {
            self.get_external_controller().await
        }
    }

    fn is_local_controller_host(host: &str) -> bool {
        matches!(host, "127.0.0.1" | "localhost" | "0.0.0.0" | "::1")
    }

    fn normalize_external_controller(controller: &str) -> Result<String> {
        let controller = controller.trim();
        if controller.is_empty() {
            return Err(Self::invalid_external_controller_error("<empty>"));
        }

        if controller.starts_with('/') {
            return Ok(controller.to_string());
        }
        if controller.starts_with("unix://") {
            if controller.trim_start_matches("unix://").is_empty() {
                return Err(Self::invalid_external_controller_error(controller));
            }
            return Ok(controller.to_string());
        }
        if controller.contains("://")
            && !controller.starts_with("http://")
            && !controller.starts_with("https://")
        {
            return Err(Self::invalid_external_controller_error(controller));
        }

        let url = if controller.starts_with(':') {
            format!("http://127.0.0.1{}", controller)
        } else if controller.starts_with("http://") || controller.starts_with("https://") {
            controller.to_string()
        } else {
            format!("http://{}", controller)
        };

        let parsed =
            Url::parse(&url).map_err(|_| Self::invalid_external_controller_error(controller))?;
        if parsed.host_str().is_none() {
            return Err(Self::invalid_external_controller_error(controller));
        }

        Ok(url)
    }
}

#[cfg(test)]
mod tests {
    use super::ConfigDirSource;
    use super::ConfigManager;
    use std::sync::OnceLock;
    use tempfile::tempdir;
    use tokio::fs;
    use tokio::sync::Mutex;

    fn env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    fn sample_config() -> &'static str {
        "port: 7890\nsocks-port: 7891\nexternal-controller: 127.0.0.1:9090\n"
    }

    #[test]
    fn is_local_controller_host_matches_expected_values() {
        assert!(ConfigManager::is_local_controller_host("127.0.0.1"));
        assert!(ConfigManager::is_local_controller_host("localhost"));
        assert!(ConfigManager::is_local_controller_host("0.0.0.0"));
        assert!(ConfigManager::is_local_controller_host("::1"));
        assert!(!ConfigManager::is_local_controller_host("example.com"));
        assert!(!ConfigManager::is_local_controller_host("192.168.1.1"));
    }

    #[test]
    fn normalize_external_controller_validates_and_normalizes() {
        assert_eq!(
            ConfigManager::normalize_external_controller(":10090").expect("normalize colon"),
            "http://127.0.0.1:10090"
        );
        assert_eq!(
            ConfigManager::normalize_external_controller("127.0.0.1:9090")
                .expect("normalize host:port"),
            "http://127.0.0.1:9090"
        );
        assert_eq!(
            ConfigManager::normalize_external_controller("https://example.com:18443")
                .expect("keep https"),
            "https://example.com:18443"
        );
        let empty_err = ConfigManager::normalize_external_controller("")
            .expect_err("empty controller should fail");
        assert!(empty_err
            .to_string()
            .contains("Invalid external-controller value '<empty>'"));
        assert!(ConfigManager::normalize_external_controller("://invalid").is_err());
        assert!(ConfigManager::normalize_external_controller("unix://").is_err());
    }

    #[test]
    fn config_manager_new_smoke() {
        let manager = ConfigManager::new().expect("config manager should be constructible");
        let _ = manager.settings_file.clone();
    }

    #[tokio::test]
    async fn list_profiles_ignores_non_yaml_entries() {
        let _guard = env_lock().lock().await;
        let old_value = std::env::var("MIHOMO_CONFIGS_DIR").ok();
        std::env::remove_var("MIHOMO_CONFIGS_DIR");

        let temp = tempdir().expect("create temp dir");
        let manager =
            ConfigManager::with_home(temp.path().to_path_buf()).expect("create config manager");
        let config_dir = manager.resolve_config_dir().expect("resolve config dir");

        fs::create_dir_all(&config_dir)
            .await
            .expect("create config dir");
        fs::write(
            config_dir.join("notes.txt"),
            "this should not be treated as profile",
        )
        .await
        .expect("write non-yaml file");

        let profiles = manager.list_profiles().await.expect("list profiles");
        assert!(profiles.is_empty());

        if let Some(value) = old_value {
            std::env::set_var("MIHOMO_CONFIGS_DIR", value);
        }
    }

    #[tokio::test]
    async fn get_current_path_uses_selected_profile() {
        let _guard = env_lock().lock().await;
        let old_value = std::env::var("MIHOMO_CONFIGS_DIR").ok();
        std::env::remove_var("MIHOMO_CONFIGS_DIR");

        let temp = tempdir().expect("create temp dir");
        let manager =
            ConfigManager::with_home(temp.path().to_path_buf()).expect("create config manager");

        manager
            .save(
                "alpha",
                "port: 7890\nsocks-port: 7891\nexternal-controller: 127.0.0.1:9090\n",
            )
            .await
            .expect("save alpha profile");
        manager
            .set_current("alpha")
            .await
            .expect("set current profile");

        let current_path = manager.get_current_path().await.expect("get current path");
        assert_eq!(
            current_path,
            manager
                .resolve_config_dir()
                .expect("resolve config dir")
                .join("alpha.yaml")
        );

        if let Some(value) = old_value {
            std::env::set_var("MIHOMO_CONFIGS_DIR", value);
        }
    }

    #[tokio::test]
    async fn unit_module_profile_lifecycle_hits_file_branches() {
        let _guard = env_lock().lock().await;
        let old_value = std::env::var("MIHOMO_CONFIGS_DIR").ok();
        std::env::remove_var("MIHOMO_CONFIGS_DIR");

        let temp = tempdir().expect("create temp dir");
        let manager =
            ConfigManager::with_home(temp.path().to_path_buf()).expect("create config manager");

        assert_eq!(
            manager
                .list_profiles()
                .await
                .expect("list without dir should work")
                .len(),
            0
        );

        manager
            .save("alpha", sample_config())
            .await
            .expect("save alpha");
        manager
            .save("beta", sample_config())
            .await
            .expect("save beta");

        let loaded = manager.load("alpha").await.expect("load alpha");
        assert!(loaded.contains("external-controller"));

        let profiles = manager.list_profiles().await.expect("list profiles");
        assert_eq!(profiles.len(), 2);

        manager.set_current("beta").await.expect("set beta current");
        manager
            .set_current("alpha")
            .await
            .expect("set alpha current");
        assert_eq!(
            manager.get_current().await.expect("read current profile"),
            "alpha"
        );
        assert_eq!(
            manager.get_current_path().await.expect("current path"),
            manager
                .resolve_config_dir()
                .expect("resolve config dir")
                .join("alpha.yaml")
        );

        manager
            .save(
                "alpha",
                "port: 7890\nsocks-port: 7891\nexternal-controller: https://example.com:18443\n",
            )
            .await
            .expect("rewrite alpha with remote external controller");
        assert_eq!(
            manager
                .ensure_external_controller()
                .await
                .expect("ensure external controller"),
            "https://example.com:18443"
        );

        manager
            .delete_profile("beta")
            .await
            .expect("delete non-active profile should succeed");
        assert!(!manager
            .resolve_config_dir()
            .expect("resolve config dir")
            .join("beta.yaml")
            .exists());

        if let Some(value) = old_value {
            std::env::set_var("MIHOMO_CONFIGS_DIR", value);
        }
    }

    #[tokio::test]
    async fn resolve_config_dir_supports_paths_config_in_settings() {
        let _guard = env_lock().lock().await;
        let old_value = std::env::var("MIHOMO_CONFIGS_DIR").ok();
        std::env::remove_var("MIHOMO_CONFIGS_DIR");

        let temp = tempdir().expect("create temp dir");
        let home = temp.path().to_path_buf();
        let manager = ConfigManager::with_home(home.clone()).expect("create config manager");

        fs::write(
            home.join("config.toml"),
            "[paths]\nconfigs_dir = \"cloud/configs\"\n",
        )
        .await
        .expect("write settings");

        let resolved = manager
            .get_config_dir_info()
            .expect("resolve config dir info");
        assert_eq!(resolved.path, home.join("cloud/configs"));
        assert_eq!(resolved.source, ConfigDirSource::Settings);

        if let Some(value) = old_value {
            std::env::set_var("MIHOMO_CONFIGS_DIR", value);
        }
    }

    #[tokio::test]
    async fn resolve_config_dir_prefers_env_override() {
        let _guard = env_lock().lock().await;
        let temp = tempdir().expect("create temp dir");
        let home = temp.path().to_path_buf();
        let manager = ConfigManager::with_home(home.clone()).expect("create config manager");

        fs::write(
            home.join("config.toml"),
            "[paths]\nconfigs_dir = \"cloud/configs\"\n",
        )
        .await
        .expect("write settings");

        let env_dir = home.join("override-configs");
        let old_value = std::env::var("MIHOMO_CONFIGS_DIR").ok();
        std::env::set_var("MIHOMO_CONFIGS_DIR", &env_dir);

        let resolved = manager
            .get_config_dir_info()
            .expect("resolve config dir info");
        assert_eq!(resolved.path, env_dir);
        assert_eq!(resolved.source, ConfigDirSource::Env);

        if let Some(value) = old_value {
            std::env::set_var("MIHOMO_CONFIGS_DIR", value);
        } else {
            std::env::remove_var("MIHOMO_CONFIGS_DIR");
        }
    }

    #[tokio::test]
    async fn resolve_config_dir_defaults_when_not_configured() {
        let _guard = env_lock().lock().await;
        let old_value = std::env::var("MIHOMO_CONFIGS_DIR").ok();
        std::env::remove_var("MIHOMO_CONFIGS_DIR");

        let temp = tempdir().expect("create temp dir");
        let home = temp.path().to_path_buf();
        let manager = ConfigManager::with_home(home.clone()).expect("create config manager");

        let resolved = manager
            .get_config_dir_info()
            .expect("resolve config dir info");
        assert_eq!(resolved.path, home.join("configs"));
        assert_eq!(resolved.source, ConfigDirSource::Default);

        if let Some(value) = old_value {
            std::env::set_var("MIHOMO_CONFIGS_DIR", value);
        }
    }
}
