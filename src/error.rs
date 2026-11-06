use thiserror::Error;

#[derive(Error, Debug)]
#[allow(dead_code)]
pub enum ProxyError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("WebSocket error: {0}")]
    WebSocket(#[from] Box<tokio_tungstenite::tungstenite::Error>),

    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Process exited with status: {0}")]
    ProcessExited(std::process::ExitStatus),

    #[error("Process spawn failed: {0}")]
    ProcessSpawn(String),

    #[error("Server '{0}' not found")]
    ServerNotFound(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Protocol error: {0}")]
    Protocol(String),
}

impl From<tokio_tungstenite::tungstenite::Error> for ProxyError {
    fn from(err: tokio_tungstenite::tungstenite::Error) -> Self {
        ProxyError::WebSocket(Box::new(err))
    }
}

pub type Result<T> = std::result::Result<T, ProxyError>;
