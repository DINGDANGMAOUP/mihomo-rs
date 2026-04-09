use mihomo_rs::{ConfigManager, Result, VersionManager};

#[tokio::main]
async fn main() -> Result<()> {
    let temp = tempfile::tempdir()?;
    std::env::set_var("MIHOMO_HOME", temp.path());

    let cm = ConfigManager::new()?;
    let vm = VersionManager::new()?;

    cm.ensure_default_config().await?;
    let profiles = cm.list_profiles().await?;
    let versions = vm.list_installed().await?;

    println!("home: {}", temp.path().display());
    println!("profiles: {}", profiles.len());
    println!("installed versions: {}", versions.len());
    Ok(())
}
