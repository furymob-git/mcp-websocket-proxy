use crate::config::ServerConfig;
use crate::error::{ProxyError, Result};
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::mpsc;
use tracing::{error, info, warn};

pub struct McpProcess {
    #[allow(dead_code)]
    pub server_id: String,
    pub config: ServerConfig,
    stdin_tx: mpsc::Sender<String>,
    is_running: Arc<AtomicBool>,
}

impl McpProcess {
    pub async fn spawn(
        server_id: String,
        config: ServerConfig,
        stdout_tx: mpsc::Sender<String>,
    ) -> Result<Arc<Self>> {
        let (stdin_tx, stdin_rx) = mpsc::channel::<String>(256);
        let is_running = Arc::new(AtomicBool::new(true));

        let process = Arc::new(Self {
            server_id: server_id.clone(),
            config: config.clone(),
            stdin_tx,
            is_running: Arc::clone(&is_running),
        });

        Self::start_process_supervisor(server_id, config, stdin_rx, stdout_tx, is_running);

        Ok(process)
    }

    pub async fn send_line(&self, line: String) -> Result<()> {
        self.stdin_tx
            .send(line)
            .await
            .map_err(|_| ProxyError::Protocol("Process stdin channel closed".to_string()))
    }

    pub fn is_alive(&self) -> bool {
        self.is_running.load(Ordering::SeqCst)
    }

    fn start_process_supervisor(
        server_id: String,
        config: ServerConfig,
        mut stdin_rx: mpsc::Receiver<String>,
        stdout_tx: mpsc::Sender<String>,
        is_running: Arc<AtomicBool>,
    ) {
        tokio::spawn(async move {
            loop {
                info!(server = %server_id, cmd = %config.command, "Starting MCP subprocess");

                let mut cmd = Command::new(&config.command);
                cmd.args(&config.args);
                for (k, v) in &config.env {
                    cmd.env(k, v);
                }
                if let Some(ref dir) = config.working_dir {
                    cmd.current_dir(dir);
                }
                cmd.stdin(Stdio::piped());
                cmd.stdout(Stdio::piped());
                cmd.stderr(Stdio::piped());

                let mut child: Child = match cmd.spawn() {
                    Ok(c) => c,
                    Err(e) => {
                        error!(server = %server_id, error = %e, "Failed to spawn MCP subprocess");
                        is_running.store(false, Ordering::SeqCst);
                        if !config.auto_restart {
                            break;
                        }
                        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                        continue;
                    }
                };

                is_running.store(true, Ordering::SeqCst);

                let mut child_stdin = child.stdin.take().expect("Child stdin piped");
                let child_stdout = child.stdout.take().expect("Child stdout piped");
                let child_stderr = child.stderr.take().expect("Child stderr piped");

                let s_id = server_id.clone();
                let out_tx = stdout_tx.clone();
                let stdout_task = tokio::spawn(async move {
                    let mut reader = BufReader::new(child_stdout).lines();
                    while let Ok(Some(line)) = reader.next_line().await {
                        let trimmed = line.trim();
                        if !trimmed.is_empty() && out_tx.send(trimmed.to_string()).await.is_err() {
                            break;
                        }
                    }
                });

                let err_id = server_id.clone();
                let stderr_task = tokio::spawn(async move {
                    let mut reader = BufReader::new(child_stderr).lines();
                    while let Ok(Some(line)) = reader.next_line().await {
                        warn!(target: "mcp_stderr", server = %err_id, "{}", line);
                    }
                });

                // Pipe incoming lines to child stdin
                loop {
                    tokio::select! {
                        line = stdin_rx.recv() => {
                            match line {
                                Some(mut data) => {
                                    if !data.ends_with('\n') {
                                        data.push('\n');
                                    }
                                    if let Err(e) = child_stdin.write_all(data.as_bytes()).await {
                                        warn!(server = %s_id, error = %e, "Error writing to stdin");
                                        break;
                                    }
                                    let _ = child_stdin.flush().await;
                                }
                                None => {
                                    // Receiver closed, shutdown
                                    let _ = child.kill().await;
                                    return;
                                }
                            }
                        }
                        status = child.wait() => {
                            warn!(server = %s_id, status = ?status, "MCP child process terminated");
                            break;
                        }
                    }
                }

                is_running.store(false, Ordering::SeqCst);
                stdout_task.abort();
                stderr_task.abort();

                if !config.auto_restart {
                    break;
                }

                info!(server = %s_id, "Waiting before restarting MCP subprocess...");
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            }
        });
    }
}
