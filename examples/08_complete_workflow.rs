use mihomo_rs::{
    install_mihomo, start_service, stop_service, ConfigManager, ConnectionManager, MihomoClient,
    ProxyManager, Result,
};

#[tokio::main]
async fn main() -> Result<()> {
    let cm = ConfigManager::new()?;
    cm.ensure_default_config().await?;
    cm.ensure_external_controller().await?;

    // 1) install a version (or latest stable when None)
    let version = install_mihomo(None).await?;
    println!("installed version: {version}");

    // 2) start service with current profile
    let config_path = cm.get_current_path().await?;
    start_service(&config_path).await?;

    // 3) query runtime data
    let controller = cm.get_external_controller().await?;
    let client = MihomoClient::new(&controller, None)?;

    let pm = ProxyManager::new(client.clone());
    let conn = ConnectionManager::new(client);

    println!("proxy groups: {}", pm.list_groups().await?.len());
    println!("connections: {}", conn.list().await?.len());

    // 4) graceful shutdown
    stop_service(&config_path).await?;
    Ok(())
}
