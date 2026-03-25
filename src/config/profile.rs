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
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn validate_fails_when_file_missing() {
        let temp = tempdir().expect("tempdir");
        let profile = Profile::new("missing".to_string(), temp.path().join("no.yaml"), false);
        assert!(profile.validate().await.is_err());
    }

    #[tokio::test]
    async fn validate_and_backup_success() {
        let temp = tempdir().expect("tempdir");
        let path = temp.path().join("ok.yaml");
        tokio::fs::write(
            &path,
            "port: 7890\nsocks-port: 7891\nexternal-controller: 127.0.0.1:9090\n",
        )
        .await
        .expect("write yaml");

        let profile = Profile::new("ok".to_string(), path.clone(), true);
        profile.validate().await.expect("validate");

        let backup = profile.backup().await.expect("backup");
        assert!(backup.exists());
        assert_eq!(
            tokio::fs::read_to_string(path).await.unwrap(),
            tokio::fs::read_to_string(backup).await.unwrap()
        );
    }
}
