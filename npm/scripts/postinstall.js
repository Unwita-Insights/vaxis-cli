#!/usr/bin/env node

import { createWriteStream, existsSync, chmodSync, symlinkSync, unlinkSync, renameSync } from 'fs';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';
import { platform, arch } from 'os';
import https from 'https';

const __dirname = dirname(fileURLToPath(import.meta.url));
const BIN_DIR = join(__dirname, '..', 'bin');
const VERSION = process.env.npm_package_version;
const IS_GLOBAL = process.env.npm_config_global === 'true';

function getPlatformKey() {
  const os = platform();
  const cpu = arch();

  if (os === 'linux') {
    if (existsSync('/lib/ld-musl-x86_64.so.1') || existsSync('/lib/ld-musl-aarch64.so.1')) {
      return `linux-musl-${cpu}`;
    }
    return `linux-${cpu}`;
  }

  if (os === 'win32') return `windows-${cpu}`;
  return `${os}-${cpu}`;
}

const PLATFORM_MAP = {
  'darwin-arm64':     'vaxis-darwin-arm64',
  'darwin-x64':       'vaxis-darwin-x64',
  'linux-x64':        'vaxis-linux-x64',
  'linux-arm64':      'vaxis-linux-arm64',
  'linux-musl-x64':   'vaxis-linux-musl-x64',
  'windows-x64':      'vaxis-windows-x64.exe',
};

function downloadFile(url, dest) {
  return new Promise((resolve, reject) => {
    const follow = (u) => {
      https.get(u, (res) => {
        if (res.statusCode === 301 || res.statusCode === 302) {
          return follow(res.headers.location);
        }
        if (res.statusCode !== 200) {
          res.resume();
          return reject(new Error(`HTTP ${res.statusCode} from ${u}`));
        }
        const tmp = `${dest}.tmp`;
        const file = createWriteStream(tmp);
        res.pipe(file);
        file.on('finish', () => {
          file.close(() => {
            renameSync(tmp, dest);
            resolve();
          });
        });
        file.on('error', reject);
      }).on('error', reject);
    };
    follow(url);
  });
}

async function main() {
  const platformKey = getPlatformKey();
  const binaryName = PLATFORM_MAP[platformKey];

  if (!binaryName) {
    process.stderr.write(
      `[vaxis] Unsupported platform: ${platformKey}\n` +
      `[vaxis] Install from source: cargo install vaxis\n`
    );
    process.exit(0); // Don't fail the install
  }

  const ext = platform() === 'win32' ? '.exe' : '';
  const dest = join(BIN_DIR, `vaxis-native${ext}`);
  const url = `https://github.com/Unwita-Insights/vaxis-cli/releases/download/v${VERSION}/${binaryName}`;

  process.stdout.write(`[vaxis] Downloading binary for ${platformKey}...\n`);

  try {
    await downloadFile(url, dest);

    if (platform() !== 'win32') {
      chmodSync(dest, 0o755);
    }

    process.stdout.write(`[vaxis] Installed successfully.\n`);

    // Global install optimization: replace Node.js shim with direct symlink
    // This eliminates Node.js startup time on every `vaxis` invocation
    if (IS_GLOBAL && platform() !== 'win32') {
      const shim = join(BIN_DIR, 'vaxis.js');
      try {
        unlinkSync(shim);
        symlinkSync(dest, shim);
        process.stdout.write(`[vaxis] Optimized: using native binary directly.\n`);
      } catch {
        // Non-fatal — shim still works
      }
    }
  } catch (err) {
    process.stderr.write(
      `[vaxis] Binary download failed: ${err.message}\n` +
      `[vaxis] Install from source: cargo install vaxis\n`
    );
    process.exit(0); // Don't fail the install — degrade gracefully
  }
}

main();
