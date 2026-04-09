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

#[cfg(test)]
mod tests {
    use super::Profile;
    use tempfile::tempdir;
    use tokio::fs;

    #[tokio::test]
    async fn validate_and_backup_success() {
        let temp = tempdir().expect("create temp dir");
        let path = temp.path().join("profile.yaml");
        fs::write(
            &path,
            "port: 7890\nsocks-port: 7891\nexternal-controller: 127.0.0.1:9090\n",
        )
        .await
        .expect("write config");

        let profile = Profile::new("default".to_string(), path.clone(), true);
        profile.validate().await.expect("validate profile");

        let backup = profile.backup().await.expect("backup profile");
        assert!(backup.exists());

        let content = fs::read_to_string(backup).await.expect("read backup");
        assert!(content.contains("external-controller"));
    }

    #[tokio::test]
    async fn validate_errors_for_missing_and_invalid_yaml() {
        let temp = tempdir().expect("create temp dir");
        let missing = temp.path().join("missing.yaml");
        let missing_profile = Profile::new("missing".to_string(), missing, false);
        assert!(missing_profile.validate().await.is_err());

        let invalid = temp.path().join("invalid.yaml");
        fs::write(&invalid, "invalid: yaml: [")
            .await
            .expect("write invalid yaml");
        let invalid_profile = Profile::new("invalid".to_string(), invalid, false);
        assert!(invalid_profile.validate().await.is_err());
    }
}
