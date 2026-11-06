#!/usr/bin/env node

const fs = require('fs');
const path = require('path');
const https = require('https');
const { execSync } = require('child_process');

const VERSION = require('../package.json').version;
const REPO = 'eliot/mcp-websocket-proxy';

function getTargetInfo() {
  const platform = process.platform;
  const arch = process.arch;

  if (platform === 'win32') {
    return {
      triple: 'x86_64-pc-windows-msvc',
      ext: '.zip',
      binName: 'mcp-websocket-proxy.exe',
    };
  } else if (platform === 'darwin') {
    return {
      triple: arch === 'arm64' ? 'aarch64-apple-darwin' : 'x86_64-apple-darwin',
      ext: '.tar.gz',
      binName: 'mcp-websocket-proxy',
    };
  } else if (platform === 'linux') {
    return {
      triple: arch === 'arm64' ? 'aarch64-unknown-linux-gnu' : 'x86_64-unknown-linux-gnu',
      ext: '.tar.gz',
      binName: 'mcp-websocket-proxy',
    };
  }
  throw new Error(`Unsupported platform: ${platform} ${arch}`);
}

async function main() {
  const binDir = path.join(__dirname, '..', 'dist');
  const target = getTargetInfo();
  const binPath = path.join(binDir, target.binName);

  if (fs.existsSync(binPath)) {
    return;
  }

  fs.mkdirSync(binDir, { recursive: true });

  const url = `https://github.com/${REPO}/releases/download/v${VERSION}/mcp-websocket-proxy-${target.triple}${target.ext}`;
  console.log(`[mcp-websocket-proxy] Downloading binary from ${url}...`);

  // Try downloading or notify user
  try {
    const archivePath = path.join(binDir, `archive${target.ext}`);
    const file = fs.createWriteStream(archivePath);

    https.get(url, (res) => {
      if (res.statusCode >= 300 && res.statusCode < 400 && res.headers.location) {
        https.get(res.headers.location, (redirectRes) => {
          redirectRes.pipe(file);
          file.on('finish', () => {
            file.close(() => {
              unpack(archivePath, binDir, target);
            });
          });
        });
      } else if (res.statusCode === 200) {
        res.pipe(file);
        file.on('finish', () => {
          file.close(() => {
            unpack(archivePath, binDir, target);
          });
        });
      } else {
        console.warn(`[mcp-websocket-proxy] Release binary not yet available online for v${VERSION} (${res.statusCode}). Will use system binary or build from source.`);
      }
    }).on('error', (err) => {
      console.warn(`[mcp-websocket-proxy] Download skipped: ${err.message}`);
    });
  } catch (err) {
    console.warn(`[mcp-websocket-proxy] Note: prebuilt binary download skipped: ${err.message}`);
  }
}

function unpack(archivePath, binDir, target) {
  try {
    if (target.ext === '.zip') {
      execSync(`tar -xf "${archivePath}" -C "${binDir}"`);
    } else {
      execSync(`tar -xzf "${archivePath}" -C "${binDir}"`);
    }
    const binPath = path.join(binDir, target.binName);
    if (fs.existsSync(binPath)) {
      fs.chmodSync(binPath, 0o755);
    }
    if (fs.existsSync(archivePath)) {
      fs.unlinkSync(archivePath);
    }
  } catch (e) {
    console.warn(`[mcp-websocket-proxy] Extraction warning: ${e.message}`);
  }
}

main().catch(console.error);
