# mcp-websocket-proxy

An ultra-fast, low-RAM bidirectional proxy and router between **Model Context Protocol (MCP)** on `stdio` and **WebSocket**.

Built in Rust with Tokio and Axum. Designed for sub-millisecond frame relay latency and a minimal memory footprint (< 10 MB RAM resident set size).

---

## Why?

Most Model Context Protocol (MCP) servers communicate over standard input/output (`stdio`). While this works seamlessly for local desktop applications like Claude Desktop or Cursor, it hits barriers in several environments:

- **Sandboxed game engines & plugins**: In **Roblox Studio**, Lua scripts and plugins cannot spawn external OS subprocesses directly, but they can communicate over WebSockets or local HTTP.
- **Web applications & dashboards**: Browsers and frontend clients connect natively over WebSocket connections.
- **Remote MCP deployments**: Host MCP servers inside a container, server, or cloud VM and consume them locally in your IDE without complex SSH plumbing.

`mcp-websocket-proxy` bridges the gap in both directions.

---

## Features

- **Dual Operating Modes**:
  - `serve`: Spawns one or more local MCP server subprocesses and exposes WebSocket endpoints.
  - `connect`: Connects to a remote WebSocket MCP server and bridges it to local `stdio`.
- **Multi-Server Routing**: Route different paths to isolated MCP processes (e.g. `/servers/roblox`, `/servers/filesystem`) via a simple TOML configuration file.
- **Multi-Client Concurrency with Request ID Isolation**: Multiple clients can connect concurrently to the same server. Request IDs are dynamically namespaced so each client only receives responses to its own requests, even if client IDs overlap.
- **Server Event Broadcasting**: Unsolicited server notifications (logs, tool changes) are automatically broadcast to all connected clients on that server.
- **Minimal RAM Footprint**: Operates with ~5 to 9 MB RSS under active load.
- **Clean Lifecycle & Signal Forwarding**: Automatically terminates child processes on client disconnect or shutdown (`Ctrl+C`, SIGINT/SIGTERM).
- **Zero External Runtime Dependencies**: Single self-contained static binary across Windows, macOS, and Linux.

---

## Installation

Choose the installation method that fits your environment.

### Automated Installer Scripts (Recommended)

**Linux & macOS:**
```bash
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/eliot/mcp-websocket-proxy/releases/latest/download/mcp-websocket-proxy-installer.sh | sh
```

**Windows (PowerShell):**
```powershell
irm https://github.com/eliot/mcp-websocket-proxy/releases/latest/download/mcp-websocket-proxy-installer.ps1 | iex
```

### Via Cargo

```bash
cargo install --locked mcp-websocket-proxy
```

### Via npm / npx

For Node.js environments without a local Rust toolchain:

```bash
# Run directly with npx
npx mcp-websocket-proxy serve -- npx -y @modelcontextprotocol/server-filesystem /path/to/dir

# Or install globally
npm install -g mcp-websocket-proxy
```

### Standalone Pre-compiled Binaries

Pre-compiled standalone binaries for each supported platform (Windows x64/ARM64, Linux x64/musl/ARM64, macOS Apple Silicon & Intel) are published on the [GitHub Releases](https://github.com/eliot/mcp-websocket-proxy/releases) page.

---

## Usage

### Mode 1: `serve` (Expose local stdio MCP servers over WebSocket)

#### Single Server Execution
Directly launch a command behind the proxy:

```bash
# Example with an official MCP filesystem server
mcp-websocket-proxy serve --port 8080 -- npx -y @modelcontextprotocol/server-filesystem C:\Users\eliot\Desktop

# Listen on all interfaces instead of localhost
mcp-websocket-proxy serve --host 0.0.0.0 --port 9000 -- python -m my_mcp_server
```

#### Multi-Server Routing via Configuration File
Define multiple MCP servers in a `servers.toml` file:

```bash
mcp-websocket-proxy serve --config servers.toml
```

Example `servers.toml`:

```toml
[host]
host = "127.0.0.1"
port = 8080

[servers.roblox]
command = "python"
args = ["-m", "mcp_server_roblox"]
auto_restart = true

[servers.filesystem]
command = "npx"
args = ["-y", "@modelcontextprotocol/server-filesystem", "./assets"]
auto_restart = true
```

Clients can then connect directly to specific server paths:
- `ws://127.0.0.1:8080/servers/roblox` $\rightarrow$ Routes to `roblox` server
- `ws://127.0.0.1:8080/servers/filesystem` $\rightarrow$ Routes to `filesystem` server
- `ws://127.0.0.1:8080/ws` or `ws://127.0.0.1:8080/` $\rightarrow$ Routes to `default` server

#### Built-in Healthcheck & Monitoring
- `GET http://127.0.0.1:8080/health` : Returns JSON status with active server processes, uptime, and connected client counts.
- `GET http://127.0.0.1:8080/servers` : Returns a list of available server IDs.

---

### Roblox Studio Lua Integration

In your Roblox Studio script or plugin, connect over WebSocket and send JSON-RPC requests:

```lua
-- Roblox Studio Script / Plugin
local HttpService = game:GetService("HttpService")

-- Example message handler (adjust for your WebSocket library/transport)
local function onMessage(rawMessage)
    local response = HttpService:JSONDecode(rawMessage)
    print("Received MCP response:", response)
end

local function sendMcpRequest(wsConnection, method, params, id)
    local payload = HttpService:JSONEncode({
        jsonrpc = "2.0",
        id = id or 1,
        method = method,
        params = params or {}
    })
    wsConnection:Send(payload)
end

-- Call an MCP tool
sendMcpRequest(wsConnection, "tools/list", {}, 1)
```

---

### Mode 2: `connect` (Consume a remote WebSocket MCP server in stdio clients)

Bridge a remote WebSocket MCP endpoint into a local standard I/O process. Ideal for tools like Claude Desktop or Cursor that expect a local executable.

```bash
mcp-websocket-proxy connect ws://192.168.1.50:8080
```

#### Claude Desktop Configuration (`claude_desktop_config.json`)

```json
{
  "mcpServers": {
    "remote-tools": {
      "command": "mcp-websocket-proxy",
      "args": ["connect", "ws://my-remote-server.local:8080"]
    }
  }
}
```

---

## CLI Reference

```text
mcp-websocket-proxy [OPTIONS] <COMMAND>

COMMANDS:
  serve      Start a WebSocket server relaying to one or more MCP servers
  connect    Connect to a remote WebSocket MCP server and expose a local stdio interface
  help       Print help information

OPTIONS FOR 'serve':
  -H, --host <HOST>        Host address to listen on [default: 127.0.0.1]
  -p, --port <PORT>        Port to listen on [default: 8080]
  -c, --config <PATH>      Path to a TOML configuration file for multi-server setups
  -v, --verbose            Enable verbose debug logging
  [SERVER_COMMAND]...      MCP command to execute when running without --config

OPTIONS FOR 'connect':
  <URL>                    Target WebSocket URL (e.g. ws://127.0.0.1:8080)
  -r, --reconnect <COUNT>  Max reconnection attempts on disconnect (0 for infinite) [default: 3]
```

---

## Performance & Memory

Measurements collected on Windows 11 & Linux x86_64:

| Metric | Measured Value |
| :--- | :--- |
| **Idle Memory (Working Set / RSS)** | ~ 5.4 MB |
| **Under Load Memory (Active concurrent clients)** | ~ 9.0 MB |
| **Binary Startup Time** | < 12 ms |
| **Relay Latency per Frame** | < 0.25 ms |
| **Standalone Binary Size** | ~ 2.3 MB |

---

## Contributing

See [CONVENTIONS.md](./CONVENTIONS.md) for branch naming standards, conventional commit rules, and release guidelines.

```bash
# Clone repository
git clone https://github.com/eliot/mcp-websocket-proxy.git
cd mcp-websocket-proxy

# Run test suite
cargo test

# Run linter and formatting checks
cargo clippy --all-targets --all-features -- -D warnings
cargo fmt --all -- --check
```

---

## License

This project is licensed under the [MIT License](./LICENSE).
