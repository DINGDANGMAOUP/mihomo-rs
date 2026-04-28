use crate::cli::{print_info, print_success, print_table, ConfigAction, ConfigKey};
use crate::config::{ConfigDirSource, ConfigManager};

pub async fn handle_config(action: ConfigAction) -> anyhow::Result<()> {
    let cm = ConfigManager::new()?;

    match action {
        ConfigAction::List => {
            let profiles = cm.list_profiles().await?;
            if profiles.is_empty() {
                print_info("No profiles found");
            } else {
                let rows: Vec<Vec<String>> = profiles
                    .iter()
                    .map(|p| {
                        vec![
                            if p.active { "* " } else { "  " }.to_string() + &p.name,
                            p.path.display().to_string(),
                        ]
                    })
                    .collect();
                print_table(&["Profile", "Path"], rows);
            }
        }
        ConfigAction::Current => {
            let profile = cm.get_current().await?;
            let path = cm.get_current_path().await?;
            print_table(
                &["Profile", "Path"],
                vec![vec![profile, path.display().to_string()]],
            );
        }
        ConfigAction::Path => {
            let info = cm.get_config_dir_info()?;
            println!("{}", info.path.display());
        }
        ConfigAction::Set { key, value } => match key {
            ConfigKey::ConfigsDir => {
                let resolved = cm.set_configs_dir(&value).await?;
                print_success(&format!("Set configs-dir to '{}'", resolved.display()));
                if cm.get_config_dir_info()?.source == ConfigDirSource::Env {
                    print_info("MIHOMO_CONFIGS_DIR is set and currently overrides config.toml");
                }
            }
        },
        ConfigAction::Unset { key } => match key {
            ConfigKey::ConfigsDir => {
                let resolved = cm.unset_configs_dir().await?;
                print_success(&format!(
                    "Unset configs-dir, now using '{}'",
                    resolved.display()
                ));
                if cm.get_config_dir_info()?.source == ConfigDirSource::Env {
                    print_info("MIHOMO_CONFIGS_DIR is set and currently overrides config.toml");
                }
            }
        },
        ConfigAction::Use { profile } => {
            cm.set_current(&profile).await?;
            print_success(&format!("Switched to profile '{}'", profile));
        }
        ConfigAction::Show { profile } => {
            let profile = if let Some(p) = profile {
                p
            } else {
                cm.get_current()
                    .await
                    .unwrap_or_else(|_| "default".to_string())
            };
            let content = cm.load(&profile).await?;
            println!("{}", content);
        }
        ConfigAction::Delete { profile } => {
            cm.delete_profile(&profile).await?;
            print_success(&format!("Deleted profile '{}'", profile));
        }
    }

    Ok(())
}
