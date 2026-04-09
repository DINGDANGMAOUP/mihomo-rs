mod config;
mod connection;
mod proxy;
mod service;
mod telemetry;
mod version;

use crate::cli::Commands;

pub async fn run_cli_command(command: Commands) -> anyhow::Result<()> {
    match command {
        Commands::Install { version } => version::handle_install(version).await,
        Commands::Update => version::handle_update().await,
        Commands::Default { version } => version::handle_default(version).await,
        Commands::List => version::handle_list().await,
        Commands::ListRemote { limit } => version::handle_list_remote(limit).await,
        Commands::Uninstall { version } => version::handle_uninstall(version).await,
        Commands::Config { action } => config::handle_config(action).await,
        Commands::Start => service::handle_start().await,
        Commands::Stop => service::handle_stop().await,
        Commands::Restart => service::handle_restart().await,
        Commands::Status => service::handle_status().await,
        Commands::Proxy { action } => proxy::handle_proxy(action).await,
        Commands::Logs { level } => telemetry::handle_logs(level).await,
        Commands::Traffic => telemetry::handle_traffic().await,
        Commands::Memory => telemetry::handle_memory().await,
        Commands::Connection { action } => connection::handle_connection(action).await,
    }
}

fn truncate_for_display(input: &str, max_chars: usize) -> String {
    input.chars().take(max_chars).collect()
}
