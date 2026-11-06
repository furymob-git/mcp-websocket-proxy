pub mod process;
pub mod router;
pub mod session;

use crate::config::ProxyConfig;
use crate::error::Result;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tracing::info;

pub async fn run_server(config: ProxyConfig, host: String, port: u16) -> Result<()> {
    let hub = router::McpHub::new(config).await?;
    let app = router::create_router(hub);

    let addr: SocketAddr = format!("{}:{}", host, port)
        .parse()
        .map_err(|e| crate::error::ProxyError::Config(format!("Invalid socket address: {}", e)))?;

    info!("mcp-websocket-proxy listening on ws://{}", addr);
    info!("Health check available at http://{}/health", addr);

    let listener = TcpListener::bind(addr).await?;
    axum::serve(listener, app.into_make_service()).await?;

    Ok(())
}
