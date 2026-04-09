use clap::Parser;
use mihomo_rs::cli::{format_cli_error, print_error, run_cli_command, Cli};

#[tokio::main]
async fn main() {
    if let Err(e) = run().await {
        print_error(&format_cli_error(&e));
        std::process::exit(1);
    }
}

async fn run() -> anyhow::Result<()> {
    let cli = Cli::parse();

    env_logger::Builder::from_default_env()
        .filter_level(if cli.verbose {
            log::LevelFilter::Debug
        } else {
            log::LevelFilter::Info
        })
        .init();

    run_cli_command(cli.command).await
}
