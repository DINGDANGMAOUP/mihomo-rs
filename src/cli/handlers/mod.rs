mod config;
mod connection;
mod doctor;
mod proxy;
mod service;
mod telemetry;
mod version;

use crate::cli::Commands;

pub async fn run_cli_command(command: Commands) -> anyhow::Result<()> {
    let exit_code = run_cli_command_with_exit(command).await?;
    if exit_code == 0 {
        Ok(())
    } else {
        anyhow::bail!("command exited with status {}", exit_code)
    }
}

pub async fn run_cli_command_with_exit(command: Commands) -> anyhow::Result<i32> {
    match command {
        Commands::Version { action } => version::handle_version(action).await.map(|_| 0),
        Commands::Install { version } => version::handle_install(version).await.map(|_| 0),
        Commands::Update => version::handle_update().await.map(|_| 0),
        Commands::Default { version } => version::handle_default(version).await.map(|_| 0),
        Commands::List => version::handle_list().await.map(|_| 0),
        Commands::ListRemote { limit } => version::handle_list_remote(limit).await.map(|_| 0),
        Commands::Uninstall { version } => version::handle_uninstall(version).await.map(|_| 0),
        Commands::Config { action } => config::handle_config(action).await.map(|_| 0),
        Commands::Service { action } => service::handle_service(action).await.map(|_| 0),
        Commands::Start => service::handle_start().await.map(|_| 0),
        Commands::Stop => service::handle_stop().await.map(|_| 0),
        Commands::Restart => service::handle_restart().await.map(|_| 0),
        Commands::Status => service::handle_status().await.map(|_| 0),
        Commands::Proxy { action } => proxy::handle_proxy(action).await.map(|_| 0),
        Commands::Logs { level } => telemetry::handle_logs(level).await.map(|_| 0),
        Commands::Traffic => telemetry::handle_traffic().await.map(|_| 0),
        Commands::Memory => telemetry::handle_memory().await.map(|_| 0),
        Commands::Connection { action } => connection::handle_connection(action).await.map(|_| 0),
        Commands::Doctor { action } => doctor::handle_doctor(action).await,
    }
}

fn truncate_for_display(input: &str, max_chars: usize) -> String {
    input.chars().take(max_chars).collect()
}
