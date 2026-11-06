use crate::error::{ProxyError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env: HashMap<String, String>,
    #[serde(default)]
    pub working_dir: Option<String>,
    #[serde(default = "default_true")]
    pub auto_restart: bool,
    #[serde(default)]
    pub description: Option<String>,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProxyConfig {
    #[serde(default)]
    pub host: Option<String>,
    #[serde(default)]
    pub port: Option<u16>,
    #[serde(default)]
    pub servers: HashMap<String, ServerConfig>,
}

impl ProxyConfig {
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = std::fs::read_to_string(path.as_ref())
            .map_err(|e| ProxyError::Config(format!("Failed to read config file: {}", e)))?;
        let config: ProxyConfig = toml::from_str(&content)
            .map_err(|e| ProxyError::Config(format!("Failed to parse config file: {}", e)))?;
        Ok(config)
    }

    pub fn from_single_command(command: String, args: Vec<String>) -> Self {
        let mut servers = HashMap::new();
        servers.insert(
            "default".to_string(),
            ServerConfig {
                command,
                args,
                env: HashMap::new(),
                working_dir: None,
                auto_restart: true,
                description: Some("Default MCP server".to_string()),
            },
        );

        ProxyConfig {
            host: None,
            port: None,
            servers,
        }
    }
}
