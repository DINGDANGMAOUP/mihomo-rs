use mihomo_rs::{Result, ServiceManager};

#[tokio::main]
async fn main() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let config = temp.path().join("config.yaml");
    tokio::fs::write(&config, "port: 7890\nexternal-controller: 127.0.0.1:9090\n").await?;

    let sm = ServiceManager::with_home(
        temp.path().join("missing-mihomo-binary"),
        config,
        temp.path().to_path_buf(),
    );

    println!("service running: {}", sm.is_running().await);
    println!("service status: {:?}", sm.status().await?);
    Ok(())
}
