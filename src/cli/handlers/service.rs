use crate::cli::handlers::telemetry;
use crate::cli::{print_info, print_success, ServiceAction};
use crate::config::ConfigManager;
use crate::service::{ServiceManager, ServiceStatus};
use crate::version::VersionManager;

pub async fn handle_service(action: ServiceAction) -> anyhow::Result<()> {
    match action {
        ServiceAction::Start => handle_start().await,
        ServiceAction::Stop => handle_stop().await,
        ServiceAction::Restart => handle_restart().await,
        ServiceAction::Status => handle_status().await,
        ServiceAction::Logs { level } => telemetry::handle_logs(level).await,
        ServiceAction::Traffic => telemetry::handle_traffic().await,
        ServiceAction::Memory => telemetry::handle_memory().await,
    }
}

pub async fn handle_start() -> anyhow::Result<()> {
    let vm = VersionManager::new()?;
    let cm = ConfigManager::new()?;

    cm.ensure_default_config().await?;
    let controller_url = cm.ensure_external_controller().await?;
    log::info!("External controller configured at: {}", controller_url);

    let binary = vm.get_binary_path(None).await?;
    let config = cm.get_current_path().await?;
    let sm = ServiceManager::new(binary, config);
    sm.start().await?;
    print_success("Service started");

    Ok(())
}

pub async fn handle_stop() -> anyhow::Result<()> {
    let vm = VersionManager::new()?;
    let cm = ConfigManager::new()?;
    let binary = vm.get_binary_path(None).await?;
    let config = cm.get_current_path().await?;
    let sm = ServiceManager::new(binary, config);
    sm.stop().await?;
    print_success("Service stopped");
    Ok(())
}

pub async fn handle_restart() -> anyhow::Result<()> {
    let vm = VersionManager::new()?;
    let cm = ConfigManager::new()?;
    let binary = vm.get_binary_path(None).await?;
    let config = cm.get_current_path().await?;
    let sm = ServiceManager::new(binary, config);

    if sm.is_running().await {
        sm.stop().await?;
    }

    cm.ensure_default_config().await?;
    let controller_url = cm.ensure_external_controller().await?;
    log::info!("External controller configured at: {}", controller_url);

    sm.start().await?;
    print_success("Service restarted");
    Ok(())
}

pub async fn handle_status() -> anyhow::Result<()> {
    let vm = VersionManager::new()?;
    let cm = ConfigManager::new()?;
    let binary = vm.get_binary_path(None).await?;
    let config = cm.get_current_path().await?;
    let sm = ServiceManager::new(binary, config);
    match sm.status().await? {
        ServiceStatus::Running(pid) => {
            print_success(&format!("Service is running (PID: {})", pid));
        }
        ServiceStatus::Stopped => {
            print_info("Service is stopped");
        }
    }
    Ok(())
}
