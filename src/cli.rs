use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "mcp-websocket-proxy",
    author = "Eliot",
    version,
    about = "Ultra-fast, low-RAM Model Context Protocol (MCP) stdio-to-WebSocket proxy and router.",
    long_about = None
)]
pub struct Cli {
    #[arg(short, long, global = true, help = "Enable verbose debug logging")]
    pub verbose: bool,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    #[command(about = "Start a WebSocket server relaying to one or more MCP servers")]
    Serve(ServeArgs),

    #[command(
        about = "Connect to a remote WebSocket MCP server and expose a local stdio interface"
    )]
    Connect(ConnectArgs),
}

#[derive(Args, Debug)]
pub struct ServeArgs {
    #[arg(
        short = 'H',
        long,
        default_value = "127.0.0.1",
        help = "Host address to listen on"
    )]
    pub host: String,

    #[arg(short, long, default_value_t = 8080, help = "Port to listen on")]
    pub port: u16,

    #[arg(
        short,
        long,
        help = "Path to a TOML configuration file for multi-server setups"
    )]
    pub config: Option<PathBuf>,

    #[arg(
        trailing_var_arg = true,
        allow_hyphen_values = true,
        help = "MCP server command to execute (e.g. -- npx @modelcontextprotocol/server-filesystem /path)"
    )]
    pub server_command: Vec<String>,
}

#[derive(Args, Debug)]
pub struct ConnectArgs {
    #[arg(
        help = "Target WebSocket URL (e.g. ws://127.0.0.1:8080 or ws://127.0.0.1:8080/servers/engine)"
    )]
    pub url: String,

    #[arg(
        short,
        long,
        default_value_t = 3,
        help = "Max reconnection attempts on disconnect (0 for infinite)"
    )]
    pub reconnect: u32,
}
