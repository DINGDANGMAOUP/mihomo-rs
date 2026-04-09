use crate::cli::{print_info, print_success, print_table, ConfigAction};
use crate::config::ConfigManager;

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
