use crate::error::{ProxyError, Result};
use futures_util::{SinkExt, StreamExt};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;
use tracing::{error, info, warn};

pub async fn run_client(url: String, reconnect_attempts: u32) -> Result<()> {
    let mut attempts = 0;

    loop {
        info!("Connecting to WebSocket MCP server at {}...", url);

        match connect_async(&url).await {
            Ok((ws_stream, _)) => {
                info!("Connected to {}", url);
                attempts = 0; // Reset counter on successful connection

                let (mut ws_write, mut ws_read) = ws_stream.split();

                // Forward stdio to WebSocket
                let mut stdin_task = tokio::spawn(async move {
                    let stdin = tokio::io::stdin();
                    let mut reader = BufReader::new(stdin).lines();

                    while let Ok(Some(line)) = reader.next_line().await {
                        let trimmed = line.trim();
                        if !trimmed.is_empty() {
                            if let Err(e) = ws_write.send(Message::Text(trimmed.to_string())).await
                            {
                                warn!("Error sending message to WebSocket: {}", e);
                                break;
                            }
                        }
                    }
                });

                // Forward WebSocket to stdout
                let mut stdout_task = tokio::spawn(async move {
                    let mut stdout = tokio::io::stdout();

                    while let Some(msg) = ws_read.next().await {
                        match msg {
                            Ok(Message::Text(text)) => {
                                let mut line = text.to_string();
                                if !line.ends_with('\n') {
                                    line.push('\n');
                                }
                                if let Err(e) = stdout.write_all(line.as_bytes()).await {
                                    warn!("Error writing to stdout: {}", e);
                                    break;
                                }
                                let _ = stdout.flush().await;
                            }
                            Ok(Message::Binary(bin)) => {
                                if let Ok(text) = std::str::from_utf8(&bin) {
                                    let mut line = text.to_string();
                                    if !line.ends_with('\n') {
                                        line.push('\n');
                                    }
                                    let _ = stdout.write_all(line.as_bytes()).await;
                                    let _ = stdout.flush().await;
                                }
                            }
                            Ok(Message::Close(_)) => {
                                info!("WebSocket connection closed by server");
                                break;
                            }
                            Ok(Message::Ping(_)) | Ok(Message::Pong(_)) => {}
                            Ok(_) => {}
                            Err(e) => {
                                warn!("WebSocket stream error: {}", e);
                                break;
                            }
                        }
                    }
                });

                tokio::select! {
                    _ = (&mut stdin_task) => stdout_task.abort(),
                    _ = (&mut stdout_task) => stdin_task.abort(),
                }
            }
            Err(e) => {
                error!("Failed to connect to {}: {}", url, e);
            }
        }

        attempts += 1;
        if reconnect_attempts > 0 && attempts > reconnect_attempts {
            return Err(ProxyError::Protocol(format!(
                "Exceeded maximum reconnect attempts ({})",
                reconnect_attempts
            )));
        }

        let backoff = std::cmp::min(attempts * 2, 10);
        warn!("Reconnecting in {} seconds...", backoff);
        tokio::time::sleep(tokio::time::Duration::from_secs(backoff as u64)).await;
    }
}
