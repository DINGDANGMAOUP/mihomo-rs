use crate::core::Result;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Profile {
    pub name: String,
    pub path: PathBuf,
    pub active: bool,
}

impl Profile {
    pub fn new(name: String, path: PathBuf, active: bool) -> Self {
        Self { name, path, active }
    }

    pub async fn validate(&self) -> Result<()> {
        if !self.path.exists() {
            return Err(crate::core::MihomoError::Config(format!(
                "Profile file does not exist: {}",
                self.path.display()
            )));
        }

        let content = tokio::fs::read_to_string(&self.path).await?;
        serde_yaml::from_str::<serde_yaml::Value>(&content)?;

        Ok(())
    }

    pub async fn backup(&self) -> Result<PathBuf> {
        let backup_path = self.path.with_extension("yaml.bak");
        tokio::fs::copy(&self.path, &backup_path).await?;
        Ok(backup_path)
    }
}
