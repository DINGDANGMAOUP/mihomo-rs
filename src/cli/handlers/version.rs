use crate::cli::{print_info, print_success, print_table, VersionAction};
use crate::version::{Channel, VersionManager};

pub async fn handle_version(action: VersionAction) -> anyhow::Result<()> {
    match action {
        VersionAction::Install { version } => handle_install(version).await,
        VersionAction::Update => handle_update().await,
        VersionAction::Use { version } => handle_default(version).await,
        VersionAction::List => handle_list().await,
        VersionAction::ListRemote { limit } => handle_list_remote(limit).await,
        VersionAction::Uninstall { version } => handle_uninstall(version).await,
    }
}

pub async fn handle_install(version: Option<String>) -> anyhow::Result<()> {
    let vm = VersionManager::new()?;
    let version = if let Some(v) = version {
        if let Ok(channel) = v.parse::<Channel>() {
            print_info(&format!("Installing {} channel...", channel.as_str()));
            vm.install_channel(channel).await?
        } else {
            print_info(&format!("Installing version {}...", v));
            vm.install(&v).await?;
            v
        }
    } else {
        print_info("Installing stable channel...");
        vm.install_channel(Channel::Stable).await?
    };
    print_success(&format!("Installed version {}", version));
    Ok(())
}

pub async fn handle_update() -> anyhow::Result<()> {
    let vm = VersionManager::new()?;
    print_info("Updating to latest stable version...");
    let version = vm.install_channel(Channel::Stable).await?;
    vm.set_default(&version).await?;
    print_success(&format!("Updated to version {}", version));
    Ok(())
}

pub async fn handle_default(version: String) -> anyhow::Result<()> {
    let vm = VersionManager::new()?;
    vm.set_default(&version).await?;
    print_success(&format!("Set default version to {}", version));
    Ok(())
}

pub async fn handle_list() -> anyhow::Result<()> {
    let vm = VersionManager::new()?;
    let versions = vm.list_installed().await?;
    if versions.is_empty() {
        print_info("No versions installed");
    } else {
        let rows: Vec<Vec<String>> = versions
            .iter()
            .map(|v| {
                vec![
                    if v.is_default { "* " } else { "  " }.to_string() + &v.version,
                    v.path.display().to_string(),
                ]
            })
            .collect();
        print_table(&["Version", "Path"], rows);
    }
    Ok(())
}

pub async fn handle_list_remote(limit: usize) -> anyhow::Result<()> {
    print_info(&format!("Fetching {} latest releases...", limit));
    let releases = crate::version::fetch_releases(limit).await?;
    if releases.is_empty() {
        print_info("No releases found");
    } else {
        let rows: Vec<Vec<String>> = releases
            .iter()
            .map(|r| {
                vec![
                    r.version.clone(),
                    r.name.clone(),
                    if r.prerelease { "Yes" } else { "No" }.to_string(),
                    super::truncate_for_display(&r.published_at, 10),
                ]
            })
            .collect();
        print_table(&["Version", "Name", "Prerelease", "Date"], rows);
    }
    Ok(())
}

pub async fn handle_uninstall(version: String) -> anyhow::Result<()> {
    let vm = VersionManager::new()?;
    vm.uninstall(&version).await?;
    print_success(&format!("Uninstalled version {}", version));
    Ok(())
}
