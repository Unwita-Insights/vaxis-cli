#!/usr/bin/env node

import { spawnSync } from 'child_process';
import { existsSync, chmodSync } from 'fs';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';
import { platform, arch } from 'os';

const __dirname = dirname(fileURLToPath(import.meta.url));

function getPlatformKey() {
  const os = platform();
  const cpu = arch();

  if (os === 'linux') {
    // Detect musl libc (Alpine Linux, slim Docker images)
    if (existsSync('/lib/ld-musl-x86_64.so.1') || existsSync('/lib/ld-musl-aarch64.so.1')) {
      return `linux-musl-${cpu}`;
    }
    return `linux-${cpu}`;
  }

  if (os === 'win32') return `windows-${cpu}`;
  return `${os}-${cpu}`;
}

function getBinPath() {
  const os = platform();
  const ext = os === 'win32' ? '.exe' : '';
  return join(__dirname, `vaxis-native${ext}`);
}

const binPath = getBinPath();

if (!existsSync(binPath)) {
  const platformKey = getPlatformKey();
  process.stderr.write(
    `[vaxis] Native binary not found for platform: ${platformKey}\n` +
    `[vaxis] Try reinstalling: npm install -g @unwita-insights/vaxis\n` +
    `[vaxis] Or install from source: cargo install vaxis\n`
  );
  process.exit(1);
}

// Ensure executable bit is set (may be missing if postinstall was skipped)
if (platform() !== 'win32') {
  try { chmodSync(binPath, 0o755); } catch {}
}

const result = spawnSync(binPath, process.argv.slice(2), { stdio: 'inherit' });
process.exit(result.status ?? 0);
