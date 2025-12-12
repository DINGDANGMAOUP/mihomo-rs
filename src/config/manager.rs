use super::profile::Profile;
use crate::core::{get_home_dir, MihomoError, Result};
use std::path::PathBuf;
use tokio::fs;

pub struct ConfigManager {
    config_dir: PathBuf,
    settings_file: PathBuf,
}

impl ConfigManager {
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

    pub async fn load(&self, profile: &str) -> Result<String> {
        let path = self.config_dir.join(format!("{}.yaml", profile));
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
        fs::create_dir_all(&self.config_dir).await?;

        serde_yaml::from_str::<serde_yaml::Value>(content)?;

        let path = self.config_dir.join(format!("{}.yaml", profile));
        fs::write(&path, content).await?;

        Ok(())
    }

    pub async fn list_profiles(&self) -> Result<Vec<Profile>> {
        if !self.config_dir.exists() {
            return Ok(vec![]);
        }

        let current = self.get_current().await.ok();
        let mut profiles = vec![];

        let mut entries = fs::read_dir(&self.config_dir).await?;
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
        let path = self.config_dir.join(format!("{}.yaml", profile));
        if !path.exists() {
            return Err(MihomoError::NotFound(format!(
                "Profile '{}' not found",
                profile
            )));
        }

        let current = self.get_current().await.ok();
        if current.as_ref() == Some(&profile.to_string()) {
            return Err(MihomoError::Config(
                "Cannot delete the active profile".to_string(),
            ));
        }

        fs::remove_file(path).await?;
        Ok(())
    }

    pub async fn set_current(&self, profile: &str) -> Result<()> {
        let path = self.config_dir.join(format!("{}.yaml", profile));
        if !path.exists() {
            return Err(MihomoError::NotFound(format!(
                "Profile '{}' not found",
                profile
            )));
        }

        fs::create_dir_all(self.settings_file.parent().unwrap()).await?;

        let mut config = if self.settings_file.exists() {
            let content = fs::read_to_string(&self.settings_file).await?;
            toml::from_str(&content).unwrap_or_else(|_| toml::Value::Table(toml::map::Map::new()))
        } else {
            toml::Value::Table(toml::map::Map::new())
        };

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

        let content = toml::to_string(&config)
            .map_err(|e| MihomoError::Config(format!("Failed to serialize config: {}", e)))?;
        fs::write(&self.settings_file, content).await?;

        Ok(())
    }

    pub async fn get_current(&self) -> Result<String> {
        if !self.settings_file.exists() {
            return Ok("default".to_string());
        }

        let content = fs::read_to_string(&self.settings_file).await?;
        let config: toml::Value = toml::from_str(&content)
            .map_err(|e| MihomoError::Config(format!("Invalid config: {}", e)))?;

        Ok(config
            .get("default")
            .and_then(|d| d.get("profile"))
            .and_then(|p| p.as_str())
            .unwrap_or("default")
            .to_string())
    }

    pub async fn get_current_path(&self) -> Result<PathBuf> {
        let profile = self.get_current().await?;
        Ok(self.config_dir.join(format!("{}.yaml", profile)))
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

        let url = if controller.starts_with(':') {
            format!("http://127.0.0.1{}", controller)
        } else if controller.starts_with("http://") || controller.starts_with("https://") {
            controller.to_string()
        } else {
            format!("http://{}", controller)
        };

        log::debug!("External controller URL: {}", url);
        Ok(url)
    }
}
