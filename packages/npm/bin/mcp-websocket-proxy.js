#!/usr/bin/env node

const path = require('path');
const fs = require('fs');
const { spawn } = require('child_process');

function getBinName() {
  return process.platform === 'win32' ? 'mcp-websocket-proxy.exe' : 'mcp-websocket-proxy';
}

function findBinary() {
  const localDist = path.join(__dirname, '..', 'dist', getBinName());
  if (fs.existsSync(localDist)) {
    return localDist;
  }

  // Check target/release for local development
  const localCargo = path.join(__dirname, '..', '..', '..', 'target', 'release', getBinName());
  if (fs.existsSync(localCargo)) {
    return localCargo;
  }

  return getBinName();
}

const binPath = findBinary();
const child = spawn(binPath, process.argv.slice(2), {
  stdio: 'inherit',
});

child.on('error', (err) => {
  console.error(`[mcp-websocket-proxy] Failed to run binary '${binPath}': ${err.message}`);
  console.error(`Please install mcp-websocket-proxy via cargo: 'cargo install mcp-websocket-proxy'`);
  process.exit(1);
});

child.on('exit', (code) => {
  process.exit(code ?? 0);
});
