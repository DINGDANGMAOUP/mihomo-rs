use mihomo_rs::{Result, VersionManager};

#[tokio::main]
async fn main() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let vm = VersionManager::with_home(temp.path().to_path_buf())?;

    let versions = vm.list_installed().await?;
    println!("installed versions: {}", versions.len());

    match vm.get_default().await {
        Ok(v) => println!("default version: {v}"),
        Err(e) => println!("default not set: {e}"),
    }

    Ok(())
}
