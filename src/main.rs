use clap::Parser;
use mihomo_rs::cli::{format_cli_error, print_error, run_cli_command_with_exit, Cli, Commands};

#[tokio::main]
async fn main() {
    match run().await {
        Ok(code) => {
            if code != 0 {
                std::process::exit(code);
            }
        }
        Err((is_doctor, error)) => {
            print_error(&format_cli_error(&error));
            let code = if is_doctor { 2 } else { 1 };
            std::process::exit(code);
        }
    }
}

async fn run() -> Result<i32, (bool, anyhow::Error)> {
    let cli = Cli::parse();
    let is_doctor = matches!(&cli.command, Commands::Doctor { .. });
    let command = cli.command;

    env_logger::Builder::from_default_env()
        .filter_level(if cli.verbose {
            log::LevelFilter::Debug
        } else {
            log::LevelFilter::Info
        })
        .init();

    run_cli_command_with_exit(command)
        .await
        .map_err(|error| (is_doctor, error))
}
