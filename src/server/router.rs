use crate::config::ProxyConfig;
use crate::error::Result;
use crate::framing::{make_internal_id, parse_internal_id, JsonRpcMessage, JsonRpcRequest};
use crate::server::process::McpProcess;
use crate::server::session::{ClientSession, SessionRegistry};
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Path, State};
use axum::response::IntoResponse;
use axum::routing::get;
use axum::{Json, Router};
use dashmap::DashMap;
use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

pub struct McpHub {
    #[allow(dead_code)]
    pub config: ProxyConfig,
    pub processes: DashMap<String, Arc<McpProcess>>,
    pub registry: Arc<SessionRegistry>,
}

impl McpHub {
    pub async fn new(config: ProxyConfig) -> Result<Arc<Self>> {
        let registry = SessionRegistry::new();
        let hub = Arc::new(Self {
            config: config.clone(),
            processes: DashMap::new(),
            registry,
        });

        // Initialize MCP processes for each configured server
        for (server_id, server_cfg) in config.servers {
            hub.start_server(server_id, server_cfg).await?;
        }

        Ok(hub)
    }

    pub async fn start_server(
        &self,
        server_id: String,
        config: crate::config::ServerConfig,
    ) -> Result<()> {
        let (stdout_tx, mut stdout_rx) = mpsc::channel::<String>(512);
        let process = McpProcess::spawn(server_id.clone(), config, stdout_tx).await?;
        self.processes.insert(server_id.clone(), process);

        // Spawn background task to process stdout from this MCP process
        let registry = Arc::clone(&self.registry);
        let s_id = server_id.clone();

        tokio::spawn(async move {
            while let Some(line) = stdout_rx.recv().await {
                Self::handle_process_stdout(&s_id, &line, &registry).await;
            }
        });

        Ok(())
    }

    async fn handle_process_stdout(server_id: &str, line: &str, registry: &Arc<SessionRegistry>) {
        match JsonRpcMessage::parse(line) {
            Ok(JsonRpcMessage::Response(res)) => {
                // Check if this response matches a mapped internal ID
                if let Some((client_id, original_id)) = parse_internal_id(&res.id) {
                    // Send to the specific client
                    if let Some(session) = registry.clients.get(&client_id) {
                        let mut response_val = json!({
                            "jsonrpc": res.jsonrpc,
                            "id": original_id,
                        });
                        if let Some(r) = res.result {
                            response_val["result"] = r;
                        }
                        if let Some(e) = res.error {
                            response_val["error"] = e;
                        }

                        let text = response_val.to_string();
                        let _ = session.sender.send(Message::Text(text)).await;
                    }
                    registry.pending_requests.remove(&res.id.to_string());
                } else {
                    // Fallback: check pending_requests map directly
                    let id_key = res.id.to_string();
                    if let Some((_, (client_id, original_id))) =
                        registry.pending_requests.remove(&id_key)
                    {
                        if let Some(session) = registry.clients.get(&client_id) {
                            let mut response_val = json!({
                                "jsonrpc": res.jsonrpc,
                                "id": original_id,
                            });
                            if let Some(r) = res.result {
                                response_val["result"] = r;
                            }
                            if let Some(e) = res.error {
                                response_val["error"] = e;
                            }

                            let text = response_val.to_string();
                            let _ = session.sender.send(Message::Text(text)).await;
                        }
                    }
                }
            }
            Ok(JsonRpcMessage::Notification(_))
            | Ok(JsonRpcMessage::Other(_))
            | Ok(JsonRpcMessage::Batch(_)) => {
                // Broadcast notifications to all clients connected to this specific server
                let msg_text = line.to_string();
                for entry in registry.clients.iter() {
                    if entry.value().server_id == server_id {
                        let _ = entry
                            .value()
                            .sender
                            .send(Message::Text(msg_text.clone()))
                            .await;
                    }
                }
            }
            Err(e) => {
                warn!(server = %server_id, error = %e, raw = %line, "Malformed output from MCP process");
            }
            _ => {}
        }
    }
}

pub fn create_router(hub: Arc<McpHub>) -> Router {
    Router::new()
        .route("/health", get(health_handler))
        .route("/servers", get(servers_handler))
        .route("/servers/:server_id", get(ws_server_handler))
        .route("/ws", get(ws_default_handler))
        .route("/", get(ws_default_handler))
        .with_state(hub)
}

async fn health_handler(State(hub): State<Arc<McpHub>>) -> Json<Value> {
    let servers: Vec<Value> = hub
        .processes
        .iter()
        .map(|entry| {
            let id = entry.key();
            let proc = entry.value();
            json!({
                "id": id,
                "alive": proc.is_alive(),
                "connected_clients": hub.registry.client_count_for_server(id),
                "command": proc.config.command,
            })
        })
        .collect();

    Json(json!({
        "status": "ok",
        "total_clients": hub.registry.total_clients(),
        "servers": servers,
    }))
}

async fn servers_handler(State(hub): State<Arc<McpHub>>) -> Json<Value> {
    let list: Vec<String> = hub.processes.iter().map(|e| e.key().clone()).collect();
    Json(json!({ "servers": list }))
}

async fn ws_default_handler(
    ws: WebSocketUpgrade,
    State(hub): State<Arc<McpHub>>,
) -> impl IntoResponse {
    let server_id = if hub.processes.contains_key("default") {
        "default".to_string()
    } else if let Some(first) = hub.processes.iter().next() {
        first.key().clone()
    } else {
        "default".to_string()
    };

    ws.on_upgrade(move |socket| handle_socket(socket, hub, server_id))
}

async fn ws_server_handler(
    Path(server_id): Path<String>,
    ws: WebSocketUpgrade,
    State(hub): State<Arc<McpHub>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, hub, server_id))
}

async fn handle_socket(socket: WebSocket, hub: Arc<McpHub>, server_id: String) {
    let client_id = uuid::Uuid::new_v4().to_string();
    info!(client = %client_id, server = %server_id, "WebSocket client connected");

    let (mut ws_tx, mut ws_rx) = socket.split();
    let (client_tx, mut client_rx) = mpsc::channel::<Message>(128);

    let session = ClientSession {
        id: client_id.clone(),
        server_id: server_id.clone(),
        sender: client_tx,
    };

    hub.registry.register_client(session);

    // Forward messages from client_rx to ws_tx
    let mut send_task = tokio::spawn(async move {
        while let Some(msg) = client_rx.recv().await {
            if ws_tx.send(msg).await.is_err() {
                break;
            }
        }
    });

    let hub_ref = Arc::clone(&hub);
    let c_id = client_id.clone();
    let s_id = server_id.clone();

    // Process incoming WebSocket messages from the client
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = ws_rx.next().await {
            match msg {
                Message::Text(text) => {
                    let text_str = text.as_str();
                    handle_client_text_message(&c_id, &s_id, text_str, &hub_ref).await;
                }
                Message::Binary(bin) => {
                    if let Ok(text_str) = std::str::from_utf8(&bin) {
                        handle_client_text_message(&c_id, &s_id, text_str, &hub_ref).await;
                    }
                }
                Message::Ping(_) | Message::Pong(_) => {}
                Message::Close(_) => break,
            }
        }
    });

    // Wait until one of the tasks finishes
    tokio::select! {
        _ = (&mut send_task) => recv_task.abort(),
        _ = (&mut recv_task) => send_task.abort(),
    }

    info!(client = %client_id, server = %server_id, "WebSocket client disconnected");
    hub.registry.unregister_client(&client_id);
}

async fn handle_client_text_message(
    client_id: &str,
    server_id: &str,
    text: &str,
    hub: &Arc<McpHub>,
) {
    let proc = match hub.processes.get(server_id) {
        Some(p) => p,
        None => {
            warn!(server = %server_id, "Attempted to message non-existent server");
            return;
        }
    };

    match JsonRpcMessage::parse(text) {
        Ok(JsonRpcMessage::Request(mut req)) => {
            // Rewrite ID to partition this request to this client
            let raw_id = req.id.clone();
            let internal_id = make_internal_id(client_id, &raw_id);

            hub.registry
                .pending_requests
                .insert(internal_id.clone(), (client_id.to_string(), raw_id));

            req.id = Value::String(internal_id);

            if let Ok(serialized) = serde_json::to_string(&req) {
                if let Err(e) = proc.send_line(serialized).await {
                    error!(server = %server_id, error = %e, "Failed to send request to MCP process");
                }
            }
        }
        Ok(JsonRpcMessage::Notification(_)) => {
            // Notifications have no ID, pass-through directly
            if let Err(e) = proc.send_line(text.to_string()).await {
                error!(server = %server_id, error = %e, "Failed to send notification to MCP process");
            }
        }
        Ok(JsonRpcMessage::Batch(items)) => {
            // Handle batch: rewrite IDs for requests within the batch
            let mut rewritten_batch = Vec::new();
            for item in items {
                if let Ok(JsonRpcMessage::Request(mut req)) =
                    serde_json::from_value::<JsonRpcRequest>(item.clone())
                        .map(JsonRpcMessage::Request)
                {
                    let raw_id = req.id.clone();
                    let internal_id = make_internal_id(client_id, &raw_id);
                    hub.registry
                        .pending_requests
                        .insert(internal_id.clone(), (client_id.to_string(), raw_id));
                    req.id = Value::String(internal_id);
                    if let Ok(val) = serde_json::to_value(req) {
                        rewritten_batch.push(val);
                    }
                } else {
                    rewritten_batch.push(item);
                }
            }

            if let Ok(serialized) = serde_json::to_string(&rewritten_batch) {
                let _ = proc.send_line(serialized).await;
            }
        }
        Ok(JsonRpcMessage::Response(_)) | Ok(JsonRpcMessage::Other(_)) => {
            // Forward raw line
            let _ = proc.send_line(text.to_string()).await;
        }
        Err(e) => {
            debug!(client = %client_id, error = %e, "Ignoring invalid JSON message from client");
        }
    }
}
