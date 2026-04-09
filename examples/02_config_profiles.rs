use mihomo_rs::{ConfigManager, Result};

#[tokio::main]
async fn main() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let cm = ConfigManager::with_home(temp.path().to_path_buf())?;

    let profile = r#"port: 7890
socks-port: 7891
allow-lan: false
mode: rule
log-level: info
external-controller: 127.0.0.1:9090
"#;

    cm.save("work", profile).await?;
    cm.save("gaming", profile).await?;
    cm.set_current("gaming").await?;

    for p in cm.list_profiles().await? {
        println!("{} active={}", p.name, p.active);
    }

    println!("current profile: {}", cm.get_current().await?);
    println!("controller: {}", cm.get_external_controller().await?);
    Ok(())
}
