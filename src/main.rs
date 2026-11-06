mod cli;
mod client;
mod config;
mod error;
mod framing;
mod server;

use clap::Parser;
use cli::{Cli, Commands};
use config::ProxyConfig;
use error::{ProxyError, Result};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let default_filter = if cli.verbose { "debug" } else { "info" };
    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(default_filter));

    tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .with_target(false)
        .init();

    match cli.command {
        Commands::Serve(args) => {
            let config = if let Some(config_path) = args.config {
                ProxyConfig::load_from_file(config_path)?
            } else if !args.server_command.is_empty() {
                let cmd = args.server_command[0].clone();
                let cmd_args = args.server_command[1..].to_vec();
                ProxyConfig::from_single_command(cmd, cmd_args)
            } else {
                return Err(ProxyError::Config(
                    "You must provide either a configuration file via --config or a command to execute (e.g. -- npx ...)"
                        .to_string(),
                ));
            };

            let host = config.host.clone().unwrap_or(args.host);
            let port = config.port.unwrap_or(args.port);

            server::run_server(config, host, port).await?;
        }
        Commands::Connect(args) => {
            client::run_client(args.url, args.reconnect).await?;
        }
    }

    Ok(())
}
