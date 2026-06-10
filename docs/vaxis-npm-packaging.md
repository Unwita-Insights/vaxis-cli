# Vaxis CLI — npm/npx Packaging Guide

**Goal:** Publish the `vaxis` Rust binary to npm so users can run `npm install -g vaxis` or `npx vaxis` on any platform without needing Rust or cargo installed.

---

## Why npm Matters for vaxis

Publishing to npm unlocks:
- `npx vaxis mcp-server` — zero-install Claude Code MCP integration (users add it to `.mcp.json` with no binary download)
- `npm install -g vaxis` — one-line global install for non-Rust users
- Cross-platform reach: macOS arm64/x64, Linux x64/arm64, Windows x64 — all covered automatically
- Enables the same distribution path as every major JS tooling project (esbuild, biome, turborepo)

---

## Two Approaches: Which One to Use

There are two established patterns for distributing native binaries via npm. They solve the same problem differently.

### Approach A — `postinstall` + GitHub Releases download
**Used by:** Vercel agent-browser, older tools (2020–2023)

At `npm install` time, a `postinstall.js` script runs, detects the platform, and downloads the correct binary from GitHub Releases.

```
npm install -g vaxis
  → postinstall.js runs
  → detects darwin-arm64
  → downloads https://github.com/your-org/vaxis/releases/download/v0.1.0/vaxis-darwin-arm64 
  → chmod +x
  → done
```

**Pros:** Only 1 npm package to publish. Simple setup.  
**Cons:**
- `--ignore-scripts` (common in corporate/CI environments) silently breaks it — binary is never downloaded
- Requires internet access at install time beyond the npm registry
- `npm audit` flags postinstall scripts as a potential supply-chain risk
- npm warns users about running postinstall scripts

---

### Approach B — `optionalDependencies` (platform scoped packages)
**Used by:** esbuild, Biome, Turborepo, SWC, LightningCSS (all major modern Rust/native tools)

You publish one small scoped package per platform. The main `vaxis` package lists them all as `optionalDependencies`. npm/pnpm/yarn automatically install **only the one that matches the user's platform** — no scripts, no downloads, no postinstall.

```
npm install -g vaxis
  → npm sees optionalDependencies
  → detects darwin-arm64
  → installs only @vaxis/cli-darwin-arm64 (contains the binary)
  → installs vaxis (contains the Node.js shim)
  → done
```

**Pros:**
- Works even with `--ignore-scripts`
- Works in all corporate/CI environments
- No external downloads at install time — binary is in the npm registry
- No security warnings
- Industry standard as of 2024+

**Cons:** More packages to maintain (6+ scoped packages). More CI work upfront.

---

### Recommendation for vaxis

| Phase | Approach | Reason |
|---|---|---|
| **Now (Phase 1)** | Approach A: postinstall | Faster to ship — 1 package, minimal CI changes |
| **Before public launch (Phase 2)** | Approach B: optionalDependencies | Reliable everywhere, industry standard |

This doc covers **both approaches** with complete code. Start with Approach A to ship quickly, migrate to Approach B before broad release.

---

## Approach A: Postinstall + GitHub Releases

### File Structure

```
vaxis-cli/
├── Cargo.toml
├── src/
├── npm/
│   ├── package.json        ← the npm package
│   ├── bin/
│   │   └── vaxis.js        ← Node.js wrapper (the CLI entry point)
│   └── scripts/
│       └── postinstall.js  ← downloads the binary on install
```

### `npm/package.json`

```json
{
  "name": "vaxis",
  "version": "0.1.0",
  "description": "Vaxis CLI — diagram and architecture tool for AI workflows",
  "bin": {
    "vaxis": "./bin/vaxis.js"
  },
  "scripts": {
    "postinstall": "node scripts/postinstall.js"
  },
  "files": [
    "bin/",
    "scripts/"
  ],
  "engines": {
    "node": ">=18.0.0"
  },
  "license": "MIT",
  "repository": {
    "type": "git",
    "url": "https://github.com/your-org/vaxis-cli"
  }
}
```

### `npm/scripts/postinstall.js`

```js
#!/usr/bin/env node

import { platform, arch } from 'os';
import { createWriteStream, chmodSync, existsSync } from 'fs';
import { resolve, dirname } from 'path';
import { fileURLToPath } from 'url';
import https from 'https';

const __dirname = dirname(fileURLToPath(import.meta.url));
const version = process.env.npm_package_version;

function getPlatformKey() {
  const os = platform();
  const cpu = arch();

  // Detect musl libc (Alpine Linux, Docker slim images)
  let osKey = os;
  if (os === 'linux') {
    try {
      const { execSync } = await import('child_process');
      const ldd = execSync('ldd --version 2>&1').toString();
      if (ldd.includes('musl')) osKey = 'linux-musl';
    } catch {}
  }

  return `${osKey}-${cpu}`;
}

function getBinaryName(platformKey) {
  const map = {
    'darwin-arm64':    'vaxis-darwin-arm64',
    'darwin-x64':      'vaxis-darwin-x64',
    'linux-x64':       'vaxis-linux-x64',
    'linux-arm64':     'vaxis-linux-arm64',
    'linux-musl-x64':  'vaxis-linux-musl-x64',
    'win32-x64':       'vaxis-windows-x64.exe',
  };
  return map[platformKey] ?? null;
}

function downloadFile(url, dest) {
  return new Promise((resolve, reject) => {
    const file = createWriteStream(dest);
    const follow = (u) => {
      https.get(u, (res) => {
        if (res.statusCode === 301 || res.statusCode === 302) {
          follow(res.headers.location);
          return;
        }
        if (res.statusCode !== 200) {
          reject(new Error(`HTTP ${res.statusCode} downloading ${u}`));
          return;
        }
        res.pipe(file);
        file.on('finish', () => file.close(resolve));
      }).on('error', reject);
    };
    follow(url);
  });
}

async function main() {
  const platformKey = getPlatformKey();
  const binaryName = getBinaryName(platformKey);

  if (!binaryName) {
    console.warn(`[vaxis] Unsupported platform: ${platformKey}. Install from source: cargo install vaxis`);
    process.exit(0); // Don't fail install — degrade gracefully
  }

  const url = `https://github.com/your-org/vaxis-cli/releases/download/v${version}/${binaryName}`;
  const dest = resolve(__dirname, '..', 'bin', 'vaxis-native');

  console.log(`[vaxis] Downloading binary for ${platformKey}...`);
  try {
    await downloadFile(url, dest);
    if (process.platform !== 'win32') chmodSync(dest, 0o755);
    console.log('[vaxis] Binary installed successfully.');
  } catch (err) {
    console.warn(`[vaxis] Binary download failed: ${err.message}`);
    console.warn('[vaxis] Falling back to source install: cargo install vaxis');
  }
}

main();
```

### `npm/bin/vaxis.js`

```js
#!/usr/bin/env node

import { spawnSync } from 'child_process';
import { existsSync, chmodSync } from 'fs';
import { resolve, dirname } from 'path';
import { fileURLToPath } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const binaryPath = resolve(__dirname, 'vaxis-native');
const binaryPathExe = resolve(__dirname, 'vaxis-native.exe');

const bin = process.platform === 'win32' ? binaryPathExe : binaryPath;

if (!existsSync(bin)) {
  console.error('[vaxis] Native binary not found. Run: npm install -g vaxis');
  process.exit(1);
}

// Ensure executable (in case postinstall was skipped)
if (process.platform !== 'win32') {
  try { chmodSync(bin, 0o755); } catch {}
}

const result = spawnSync(bin, process.argv.slice(2), { stdio: 'inherit' });
process.exit(result.status ?? 0);
```

---

## Approach B: optionalDependencies (Recommended Long-Term)

This is what esbuild, Biome, and Turborepo do. You publish 6 small platform packages + 1 main package.

### File Structure

```
vaxis-cli/
├── Cargo.toml
├── src/
└── npm/
    ├── vaxis/                      ← main package (what users install)
    │   ├── package.json
    │   └── bin/
    │       └── vaxis.js            ← Node.js shim
    └── platform-package.json.tmpl  ← template for platform packages (generated in CI)
```

### Published packages layout

```
npm registry:
  vaxis                        ← main package, users install this
  vaxis-darwin-arm64           ← contains vaxis binary for macOS Apple Silicon
  vaxis-darwin-x64             ← contains vaxis binary for macOS Intel
  vaxis-linux-x64              ← contains vaxis binary for Linux x64
  vaxis-linux-arm64            ← contains vaxis binary for Linux ARM
  vaxis-linux-musl-x64         ← contains vaxis binary for Alpine/musl Linux
  vaxis-win32-x64              ← contains vaxis binary for Windows x64
```

### `npm/vaxis/package.json`

```json
{
  "name": "vaxis",
  "version": "0.1.0",
  "description": "Vaxis CLI — diagram and architecture tool for AI workflows",
  "bin": {
    "vaxis": "./bin/vaxis.js"
  },
  "files": ["bin/"],
  "optionalDependencies": {
    "vaxis-darwin-arm64":   "0.1.0",
    "vaxis-darwin-x64":     "0.1.0",
    "vaxis-linux-x64":      "0.1.0",
    "vaxis-linux-arm64":    "0.1.0",
    "vaxis-linux-musl-x64": "0.1.0",
    "vaxis-win32-x64":      "0.1.0"
  },
  "engines": { "node": ">=18.0.0" },
  "license": "MIT"
}
```

### `npm/platform-package.json.tmpl`

```json
{
  "name": "${node_pkg}",
  "version": "${node_version}",
  "os": ["${node_os}"],
  "cpu": ["${node_arch}"],
  "bin": { "vaxis": "./bin/vaxis${node_ext}" },
  "files": ["bin/"],
  "license": "MIT"
}
```

### `npm/vaxis/bin/vaxis.js`

```js
#!/usr/bin/env node

import { spawnSync } from 'child_process';
import { resolve } from 'path';

function getBinaryPath() {
  let os = process.platform;
  const cpu = process.arch;

  // Detect musl (Alpine Linux)
  if (os === 'linux') {
    try {
      const { execSync } = await import('child_process');
      if (execSync('ldd --version 2>&1').toString().includes('musl')) {
        os = 'linux-musl';
      }
    } catch {}
  }

  const ext = os === 'win32' ? '.exe' : '';
  const pkg = os === 'win32' ? `vaxis-win32-${cpu}` : `vaxis-${os}-${cpu}`;

  try {
    return require.resolve(`${pkg}/bin/vaxis${ext}`);
  } catch {
    throw new Error(
      `vaxis binary not found for ${os}-${cpu}.\n` +
      `Try: npm install -g vaxis\n` +
      `Or install from source: cargo install vaxis`
    );
  }
}

const bin = getBinaryPath();
const result = spawnSync(bin, process.argv.slice(2), { stdio: 'inherit' });
process.exit(result.status ?? 0);
```

**Why `require.resolve` instead of a hardcoded path:** This lets Node.js find the binary inside `node_modules/vaxis-darwin-arm64/bin/vaxis` regardless of where npm installed it (local, global, pnpm hoisting, etc.).

---

## GitHub Actions CI Workflow

This workflow runs on every `v*` tag push. It:
1. Builds all 6 platform binaries in parallel
2. Publishes each as a scoped npm package
3. Then publishes the main `vaxis` package

**`.github/workflows/npm-publish.yml`**

```yaml
name: Publish to npm

on:
  push:
    tags: ["v*.*.*"]

jobs:
  publish-platform-packages:
    name: Build & publish ${{ matrix.build.name }}
    runs-on: ${{ matrix.build.os }}
    strategy:
      fail-fast: false
      matrix:
        build:
          - name: darwin-arm64
            os: macos-14
            target: aarch64-apple-darwin
            node_os: darwin
            node_arch: arm64
            node_ext: ""
          - name: darwin-x64
            os: macos-14
            target: x86_64-apple-darwin
            node_os: darwin
            node_arch: x64
            node_ext: ""
          - name: linux-x64
            os: ubuntu-22.04
            target: x86_64-unknown-linux-gnu
            node_os: linux
            node_arch: x64
            node_ext: ""
          - name: linux-arm64
            os: ubuntu-22.04
            target: aarch64-unknown-linux-gnu
            node_os: linux
            node_arch: arm64
            node_ext: ""
          - name: linux-musl-x64
            os: ubuntu-22.04
            target: x86_64-unknown-linux-musl
            node_os: linux-musl
            node_arch: x64
            node_ext: ""
          - name: win32-x64
            os: windows-2022
            target: x86_64-pc-windows-msvc
            node_os: win32
            node_arch: x64
            node_ext: ".exe"

    steps:
      - uses: actions/checkout@v4

      - name: Install Rust target
        run: rustup target add ${{ matrix.build.target }}

      - name: Install cross-compiler (Linux ARM64)
        if: matrix.build.target == 'aarch64-unknown-linux-gnu'
        run: |
          sudo apt-get update
          sudo apt-get install -y gcc-aarch64-linux-gnu

      - name: Install musl tools (Linux musl)
        if: matrix.build.target == 'x86_64-unknown-linux-musl'
        run: sudo apt-get install -y musl-tools

      - name: Build binary
        run: cargo build --release --target ${{ matrix.build.target }}

      - name: Setup Node.js
        uses: actions/setup-node@v4
        with:
          node-version: "20"
          registry-url: "https://registry.npmjs.org"

      - name: Publish platform package
        shell: bash
        run: |
          VERSION="${GITHUB_REF_NAME#v}"
          PKG="vaxis-${{ matrix.build.node_os }}-${{ matrix.build.node_arch }}"
          EXT="${{ matrix.build.node_ext }}"
          
          mkdir -p "npm/${PKG}/bin"
          
          # Generate package.json from template
          node_pkg="$PKG" \
          node_version="$VERSION" \
          node_os="${{ matrix.build.node_os }}" \
          node_arch="${{ matrix.build.node_arch }}" \
          node_ext="$EXT" \
          envsubst < npm/platform-package.json.tmpl > "npm/${PKG}/package.json"
          
          # Copy binary
          SRC="target/${{ matrix.build.target }}/release/vaxis${EXT}"
          cp "$SRC" "npm/${PKG}/bin/vaxis${EXT}"
          
          # Publish
          cd "npm/${PKG}"
          npm publish --access public
        env:
          NODE_AUTH_TOKEN: ${{ secrets.NPM_TOKEN }}

  publish-main-package:
    name: Publish main vaxis package
    needs: publish-platform-packages  # wait for all platform packages first
    runs-on: ubuntu-22.04
    steps:
      - uses: actions/checkout@v4

      - name: Setup Node.js
        uses: actions/setup-node@v4
        with:
          node-version: "20"
          registry-url: "https://registry.npmjs.org"

      - name: Update version and publish
        run: |
          VERSION="${GITHUB_REF_NAME#v}"
          cd npm/vaxis
          # Stamp the version from the git tag
          npm version "$VERSION" --no-git-tag-version
          # Update optionalDependencies versions to match
          node -e "
            const fs = require('fs');
            const pkg = JSON.parse(fs.readFileSync('package.json'));
            for (const k of Object.keys(pkg.optionalDependencies)) {
              pkg.optionalDependencies[k] = process.env.VERSION;
            }
            fs.writeFileSync('package.json', JSON.stringify(pkg, null, 2));
          " VERSION="$VERSION"
          npm publish --access public
        env:
          NODE_AUTH_TOKEN: ${{ secrets.NPM_TOKEN }}
```

**Key ordering rule:** `publish-main-package` has `needs: publish-platform-packages`. The main package cannot publish first — npm will reject optional dependencies that don't exist yet in the registry.

---

## How Users Install and Use It

```bash
# Global install
npm install -g vaxis
vaxis diagrams list --app <id>

# One-shot with npx (no install)
npx vaxis diagrams list --app <id>

# Zero-install MCP server (for Claude Code .mcp.json)
npx -y vaxis mcp-server
```

---

## One-Time Setup: npm Account + Secrets

1. Create an npm account at [npmjs.com](https://www.npmjs.com)
2. Create an npm organization: `your-org` (for scoped packages like `@your-org/vaxis-linux-x64`) OR use unscoped names like `vaxis-linux-x64`
3. Generate an npm token: `npm token create --type=granular` → scope to publish only
4. Add to GitHub repo secrets: `Settings → Secrets → NPM_TOKEN`
5. For OIDC trusted publishing (no stored tokens): configure npm provenance — see [npm provenance docs](https://docs.npmjs.com/generating-provenance-statements)

---

## agent-browser vs. This Approach

| | agent-browser (postinstall + GH Releases) | This doc (optionalDependencies) |
|---|---|---|
| Number of npm packages | 1 | 7 (1 main + 6 platform) |
| `--ignore-scripts` safe | ❌ Breaks | ✅ Works |
| Corporate CI | ❌ Often blocked | ✅ Works |
| External download at install | ❌ GitHub Releases HTTP | ✅ Only npm registry |
| npm audit warnings | ❌ Flags postinstall | ✅ Clean |
| Setup complexity | Low | Medium |
| Industry standard 2026 | No (older pattern) | Yes (esbuild/biome/turbo) |

agent-browser's approach **works** — but it has real failure modes in enterprise environments. The optionalDependencies pattern is more work upfront but significantly more reliable in production.

---

## Automation: cargo-npm (Optional)

[cargo-npm](https://github.com/abemedia/cargo-npm) automates most of the above. It generates the package structure, handles binary compilation for all targets, manages metadata, and includes runtime shims with musl detection built in.

```bash
cargo install cargo-npm
cargo npm publish
```

Worth evaluating if the manual CI setup feels like too much maintenance.

---

## References

- [Packaging Rust Applications for the NPM Registry — orhun.dev](https://blog.orhun.dev/packaging-rust-for-npm/)
- [Publishing Binaries on npm — Sentry Blog](https://sentry.engineering/blog/publishing-binaries-on-npm)
- [Distributing Platform-Specific Binaries with npm — MagicBell](https://www.magicbell.com/blog/distributing-platform-specific-binaries-with-npm)
- [cargo-npm — GitHub](https://github.com/abemedia/cargo-npm)
- [esbuild npm packaging — reference implementation](https://github.com/evanw/esbuild/tree/main/npm)
- [agent-browser postinstall.js — reference](https://github.com/vercel-labs/agent-browser/blob/main/scripts/postinstall.js)
