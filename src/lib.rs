pub mod cli;
pub mod config;
pub mod core;
pub mod proxy;
pub mod service;
pub mod version;

pub use config::{ConfigManager, Profile};
pub use core::{MihomoClient, MihomoError, Result};
pub use proxy::ProxyManager;
pub use service::{ServiceManager, ServiceStatus};
pub use version::{Channel, VersionManager};

use std::path::Path;

pub async fn install_mihomo(version: Option<&str>) -> Result<String> {
    let vm = VersionManager::new()?;
    if let Some(v) = version {
        vm.install(v).await?;
        Ok(v.to_string())
    } else {
        let version = vm.install_channel(Channel::Stable).await?;
        Ok(version)
    }
}

pub async fn start_service(config_path: &Path) -> Result<()> {
    let vm = VersionManager::new()?;
    let binary = vm.get_binary_path(None).await?;
    let sm = ServiceManager::new(binary, config_path.to_path_buf());
    sm.start().await
}

pub async fn stop_service(config_path: &Path) -> Result<()> {
    let vm = VersionManager::new()?;
    let binary = vm.get_binary_path(None).await?;
    let sm = ServiceManager::new(binary, config_path.to_path_buf());
    sm.stop().await
}

pub async fn switch_proxy(group: &str, proxy: &str) -> Result<()> {
    let cm = ConfigManager::new()?;
    let url = cm.get_external_controller().await?;
    let client = MihomoClient::new(&url, None)?;
    client.switch_proxy(group, proxy).await
}
