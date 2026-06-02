# Vercel agent-browser — Official Deep-Dive Documentation

> Source: https://github.com/vercel-labs/agent-browser
> Version: 0.27.1 · License: Apache-2.0

---

## Table of Contents

1. [What it is](#what-it-is)
2. [Why it was built](#why-it-was-built)
3. [Is it Rust? — Yes, here's why](#is-it-rust--yes-heres-why)
4. [Full Repository Structure](#full-repository-structure)
5. [Rust Implementation — What's inside cli/](#rust-implementation--whats-inside-cli)
6. [Rust Dependencies — Cargo.toml explained](#rust-dependencies--cargotoml-explained)
7. [Three Execution Modes](#three-execution-modes)
8. [The Snapshot-Ref System](#the-snapshot-ref-system)
9. [Skills System — What, Why, How](#skills-system--what-why-how)
10. [All 6 Skills explained](#all-6-skills-explained)
11. [How npm wraps a Rust binary](#how-npm-wraps-a-rust-binary)
12. [Build System](#build-system)
13. [Installation](#installation)

---

## What it is

**agent-browser** is a native Rust CLI that controls web browsers via the Chrome DevTools Protocol (CDP) — designed specifically for AI agents, not humans writing scripts.

```bash
agent-browser open https://example.com
agent-browser snapshot -i        # see interactive elements as @refs
agent-browser click @e3          # act on a ref
agent-browser get text @e5       # extract data
```

It is **not** a wrapper around Playwright or Puppeteer. It speaks directly to Chrome via WebSocket (CDP), with a persistent daemon so there is zero cold-start overhead between commands.

---

## Why it was built

Traditional browser automation tools produce token-heavy output — not suitable for LLMs:

| Tool | Output per snapshot | Problem |
|------|-------------------|---------|
| Playwright MCP | 3,000 – 5,000 tokens | Full DOM/JSON — too expensive |
| Screenshot-based | 800 – 1,500 tokens | Images are slow and imprecise |
| agent-browser | **200 – 400 tokens** | Compact @ref text only |

The result: **93% token savings** per interaction, making agent loops fast and cheap.

---

## Is it Rust? — Yes, here's why

The core binary is 100% Rust. Here is what the repo language split actually means:

```
Rust        85.4%   →  cli/src/  — the entire CLI + daemon
TypeScript  12.2%   →  packages/dashboard/  — web UI only
Other        2.4%   →  scripts/, docker/, config files
```

Node.js is **only** used for:
- `npm install` / `postinstall.js` — copies the right pre-built binary for your OS
- `bin/agent-browser.js` — a thin shim that finds and executes the Rust binary

At runtime, **no Node.js process runs**. Only the Rust daemon runs.

---

## Full Repository Structure

```
agent-browser/
├── bin/
│   └── agent-browser.js          # Node.js shim — finds and runs the Rust binary
│
├── cli/                           # ALL RUST CODE IS HERE
│   ├── Cargo.toml                 # Rust manifest
│   ├── Cargo.lock
│   ├── build.rs                   # Build script
│   ├── cdp-protocol/              # Chrome DevTools Protocol types
│   ├── tests/                     # Rust tests
│   └── src/
│       ├── main.rs                # Entry point — daemon / dashboard / CLI modes
│       ├── chat/                  # Natural language chat mode
│       ├── color/                 # Terminal color output
│       ├── commands/              # Every CLI command implementation
│       ├── connection/            # Daemon IPC connection
│       ├── doctor/                # agent-browser doctor diagnostics
│       ├── flags/                 # CLI flag parsing
│       ├── install/               # Chrome for Testing downloader
│       ├── native/                # Low-level browser automation
│       ├── output/                # Output formatting (text, JSON)
│       ├── skills/                # Skills loader
│       ├── upgrade/               # Self-upgrade logic
│       └── validation/            # Input validation
│
├── skill-data/                    # Skills — Markdown + frontmatter (NOT compiled code)
│   ├── core/
│   │   ├── SKILL.md              # Main skill doc (2000+ lines)
│   │   ├── references/
│   │   │   ├── commands.md
│   │   │   ├── snapshot-refs.md
│   │   │   ├── authentication.md
│   │   │   ├── session-management.md
│   │   │   ├── trust-boundaries.md
│   │   │   ├── profiling.md
│   │   │   ├── video-recording.md
│   │   │   └── proxy-support.md
│   │   └── templates/            # Shell script templates
│   ├── agentcore/
│   ├── dogfood/
│   ├── electron/
│   ├── slack/
│   └── vercel-sandbox/
│
├── packages/
│   └── dashboard/                 # TypeScript web dashboard UI
│
├── docker/
│   ├── Dockerfile.build
│   └── docker-compose.yml         # Cross-platform Linux/Windows builds
│
├── scripts/
│   ├── postinstall.js             # Picks correct binary for OS/arch on npm install
│   ├── copy-native.js             # Copies Cargo build output to bin/
│   ├── sync-version.js            # Keeps package.json + Cargo.toml versions in sync
│   └── windows-debug/
│
├── .claude-plugin/                # Claude Code integration config
├── AGENTS.md                      # Developer guide for AI agents working on this repo
├── CHANGELOG.md
├── README.md
├── agent-browser.schema.json      # Config file JSON schema
├── package.json
└── pnpm-workspace.yaml            # Monorepo config
```

---

## Rust Implementation — What's inside cli/

### main.rs — Three modes, one binary

The single compiled binary detects which mode to run:

```
AGENT_BROWSER_DAEMON set    →  daemon mode   (background process, manages browser)
AGENT_BROWSER_DASHBOARD set →  dashboard mode (web UI server)
neither set                 →  CLI mode       (sends commands to daemon via IPC)
```

### cli/src/commands/

Every command (`open`, `click`, `snapshot`, `get`, `fill`, etc.) is implemented as a Rust module here. Each command:
1. Parses its flags/arguments
2. Connects to the daemon via IPC
3. Sends a CDP message
4. Formats and prints the output

### cli/src/native/

The low-level browser automation layer — generates the accessibility tree, handles element refs, inlines iframes, manages tabs.

### cli/cdp-protocol/

Rust type definitions for all Chrome DevTools Protocol messages. This is what lets agent-browser speak directly to Chrome over WebSocket without any Node.js bridge.

---

## Rust Dependencies — Cargo.toml explained

```toml
[package]
name = "agent-browser"
version = "0.27.1"
edition = "2021"
```

### Core async + networking

| Crate | Purpose |
|-------|---------|
| `tokio` | Async runtime — powers the daemon |
| `tokio-tungstenite` | WebSocket client — CDP communication with Chrome |
| `futures-util` | Async stream utilities |
| `reqwest` | HTTP client — download Chrome, fetch resources |
| `socket2` | Low-level socket control |

### Browser protocol

| Crate | Purpose |
|-------|---------|
| `url` | URL parsing and encoding |
| `urlencoding` | Percent-encoding for CDP messages |
| `uuid` | Generate unique session/tab IDs |

### Data + serialization

| Crate | Purpose |
|-------|---------|
| `serde` + `serde_json` | JSON (de)serialization for CDP messages |
| `base64` | Encode screenshots, binary data |
| `hex` | Hex encoding for crypto operations |
| `similar` | Diff algorithm — powers `agent-browser diff snapshot` |

### Security + encryption

| Crate | Purpose |
|-------|---------|
| `aes-gcm` | AES-256-GCM encryption — session state files |
| `sha2` | SHA-256 hashing |
| `hmac` | HMAC message authentication |

### System + files

| Crate | Purpose |
|-------|---------|
| `dirs` | Cross-platform config/data directory paths |
| `rust-embed` | Embeds skill-data files into the binary at compile time |
| `zip` | Extract downloaded Chrome for Testing archive |
| `image` | Screenshot processing |
| `time` + `chrono` | Timestamps for logs, HAR files |
| `getrandom` | Cryptographic random number generation |

### Platform-specific

```toml
[target.'cfg(unix)'.dependencies]
libc = "0.2"       # Unix system calls

[target.'cfg(windows)'.dependencies]
windows-sys = "0.52"  # Windows process management
```

### Build profile (release)

```toml
[profile.release]
opt-level = 3       # Maximum optimization
lto = true          # Link-time optimization — smaller, faster binary
codegen-units = 1   # Single codegen unit — best optimization
strip = true        # Strip debug symbols — smaller binary
```

This is why the binary starts in under 50ms and has a tiny footprint.

---

## Three Execution Modes

### 1. CLI mode (default)

What you interact with:

```bash
agent-browser open https://example.com
agent-browser snapshot -i
agent-browser click @e1
```

Each command spawns the CLI binary, which connects to the daemon via IPC and sends the command. The daemon handles the actual browser interaction.

### 2. Daemon mode

Launched automatically on the first CLI command:

```
First CLI command
      ↓
CLI checks: is daemon running?
      ↓ no
CLI spawns daemon as background process
      ↓
Daemon connects to Chrome via CDP (WebSocket)
      ↓
CLI sends command to daemon via IPC
      ↓
Daemon executes via CDP → returns result
```

The daemon persists between commands — this is why subsequent commands are near-instant.

### 3. Dashboard mode

A web-based UI served locally for visualizing browser state, sessions, and snapshots. Built with TypeScript in `packages/dashboard/`.

---

## The Snapshot-Ref System

This is the core innovation that makes agent-browser token-efficient.

### How snapshot works

```bash
agent-browser snapshot -i
```

The Rust daemon:
1. Requests the accessibility tree from Chrome via CDP
2. Walks the tree, identifying interactive elements
3. Assigns sequential refs: `@e1`, `@e2`, `@e3` ...
4. Returns compact text instead of the full DOM

### Output comparison

**Raw accessibility tree (what Playwright MCP returns):**
```json
{
  "role": "WebArea",
  "children": [
    { "role": "heading", "name": "Log in", "level": 1 },
    { "role": "form", "children": [
      { "role": "textbox", "name": "Email", "required": true,
        "boundingBox": { "x": 120, "y": 280, "width": 360, "height": 40 } },
      { "role": "textbox", "name": "Password", "required": true,
        "boundingBox": { "x": 120, "y": 340, "width": 360, "height": 40 } },
      { "role": "button", "name": "Continue", "type": "submit" }
    ]}
  ]
}
// ~3,000 tokens
```

**agent-browser snapshot -i output:**
```
Page: Example — Log in
URL: https://example.com/login

@e1 [heading] "Log in"
@e2 [input type="email"] placeholder="Email"
@e3 [input type="password"] placeholder="Password"
@e4 [button type="submit"] "Continue"
@e5 [link] "Forgot password?"

// ~80 tokens
```

### The universal agent loop

```bash
agent-browser open <url>         # 1. Navigate
agent-browser snapshot -i        # 2. Perceive — get @refs
agent-browser click @e4          # 3. Act on a ref
agent-browser snapshot -i        # 4. Re-perceive (refs change after DOM update)
```

**Important:** Refs are only valid within a single snapshot. After any click, navigation, or DOM change — re-snapshot to get new refs.

---

## Skills System — What, Why, How

### What is a skill?

A skill is a **Markdown documentation package** with YAML frontmatter that teaches an AI agent how to handle a specific platform or domain beyond basic browser automation.

Skills are **not compiled code** — they are `.md` files embedded into the binary at build time via `rust-embed`.

### Skill file format

Every skill has a `SKILL.md` with this structure:

```markdown
---
name: core
description: Core agent-browser usage guide. Read this before running any
  agent-browser commands. Covers the snapshot-and-ref workflow, navigating
  pages, interacting with elements (click, fill, type, select), extracting
  text and data, taking screenshots, managing tabs, handling forms and auth,
  waiting for content, running multiple browser sessions in parallel, and
  troubleshooting common failures. Use when the user asks to interact with a
  website, fill a form, click something, extract data, take a screenshot,
  log into a site, test a web app, or automate any browser task.
allowed-tools: Bash(agent-browser:*), Bash(npx agent-browser:*)
---

# Core skill content here — commands, examples, patterns...
```

| Field | Purpose |
|-------|---------|
| `name` | Skill identifier |
| `description` | Tells the AI agent WHEN to load this skill |
| `allowed-tools` | Which shell commands this skill is permitted to run |
| Body | The actual documentation the agent reads |

### Why skills exist

The core CLI handles generic web automation. But different platforms have different patterns:

| Platform | Problem without a skill |
|----------|------------------------|
| Electron apps | Desktop UI — no URL bar, different DOM structure |
| Slack | Complex JS app, requires specific interaction patterns |
| AWS AgentCore | Needs cloud-specific flags and authentication |
| Vercel Sandbox | Isolated microVM — different networking setup |
| Dogfood/QA | Exploratory testing requires different mental model |

Without skills, an AI agent would try to apply web patterns to a desktop app, or forget platform-specific flags.

### How the agent uses skills

```bash
# Browse available skills
agent-browser skills

# Download and load a skill
agent-browser skills get slack
agent-browser skills get electron
```

When a skill is loaded, the AI agent reads the SKILL.md to understand:
- What commands to use for this platform
- What patterns to follow
- What pitfalls to avoid
- Which shell commands are allowed

---

## All 6 Skills Explained

### 1. `core` — The main skill

Every agent should read this first. 2000+ lines of documentation covering:
- The snapshot-ref workflow
- All navigation commands
- Element interaction patterns (click, fill, type, select)
- Waiting strategies
- Authentication and session persistence
- Multi-tab management
- Network interception
- Troubleshooting

Reference docs included:
- `commands.md` — full command reference
- `snapshot-refs.md` — how refs work
- `authentication.md` — login patterns
- `session-management.md` — saving/restoring sessions
- `trust-boundaries.md` — security constraints
- `profiling.md` — performance tracing
- `video-recording.md` — session recording
- `proxy-support.md` — proxy configuration

### 2. `electron` — Desktop app automation

For automating Electron-based desktop apps: VS Code, Slack desktop, Discord, Figma, Obsidian.

Electron apps have a different structure than web pages:
- No URL bar
- Multiple windows/views
- Main process vs renderer process
- Desktop-specific UI patterns

### 3. `slack` — Slack workspace automation

For automating Slack via its web interface — sending messages, reading channels, managing workspaces — without using the Slack API.

Useful when you need to:
- Automate Slack actions without API access
- Read message history from many channels
- Perform bulk administrative tasks

### 4. `agentcore` — AWS Bedrock AgentCore

For running agent-browser inside AWS Bedrock's AgentCore managed environment.

Covers:
- AWS-specific authentication
- AgentCore session flags (`--provider agentcore`)
- Networking within AWS infrastructure

### 5. `dogfood` — Exploratory testing / QA

For using agent-browser itself as a QA tool — bug hunting, exploratory testing, regression testing.

This skill is meta — it teaches the agent how to think like a QA tester using browser automation.

### 6. `vercel-sandbox` — Vercel microVM environments

For running browser automation inside Vercel Sandbox (isolated microVMs).

Covers networking differences, environment setup, and constraints specific to sandboxed execution.

---

## How npm wraps a Rust binary

This is a clever pattern worth understanding:

### 1. `npm install -g agent-browser` triggers `postinstall.js`

```js
// scripts/postinstall.js
// Detects OS + architecture
const platform = process.platform;   // darwin, linux, win32
const arch = process.arch;           // arm64, x64

// Picks the right pre-built binary from bin/
// e.g. bin/agent-browser-darwin-arm64
//      bin/agent-browser-linux-x64
//      bin/agent-browser-win32-x64.exe

// Copies it to the executable location
```

### 2. `bin/agent-browser.js` — the entry shim

```js
// A thin Node.js script that:
// 1. Finds the correct Rust binary for this OS/arch
// 2. Spawns it with all arguments passed through
// 3. Pipes stdin/stdout/stderr
```

### 3. The Rust binary takes over

Once spawned, the Node.js shim is done. The Rust binary runs as the actual process.

```
You run: agent-browser snapshot -i
      ↓
Node.js shim (bin/agent-browser.js) spawns...
      ↓
Rust binary (cli/src/main.rs) executes
      ↓
Connects to daemon via IPC
      ↓
Daemon sends CDP to Chrome
      ↓
Returns @ref snapshot
```

**Why this design?** npm is the universal distribution mechanism for developer tools. Publishing to npm means `npm install -g` just works everywhere — but the actual execution is native Rust with no Node.js overhead.

---

## Build System

### Building for your native platform

```bash
# Sync version between package.json and Cargo.toml
npm run version:sync

# Build native binary (current OS/arch)
npm run build:native
# → runs: cargo build --release --manifest-path cli/Cargo.toml
# → copies binary to bin/
```

### Building for all platforms

```bash
# macOS: builds both ARM64 and x64 in parallel
npm run build:macos

# Linux + Windows: built via Docker
npm run build:linux
npm run build:windows

# Everything:
npm run build:all-platforms
```

### Why Docker for Linux/Windows builds

Cross-compilation for Linux (musl libc for static binaries) and Windows from a Mac requires consistent toolchain environments — Docker provides that.

### Release

```bash
npm run release
# = version:sync + build:all-platforms + npm publish
```

---

## Installation

```bash
# Via npm (recommended — works on all platforms)
npm install -g agent-browser

# Via Homebrew (macOS)
brew install agent-browser

# Via Cargo (requires Rust toolchain)
cargo install agent-browser

# From source
git clone https://github.com/vercel-labs/agent-browser
cd agent-browser
pnpm install
npm run build:native
pnpm link
```

### First-time setup

```bash
# Download Chrome for Testing (needed on servers / fresh machines)
agent-browser install

# Verify setup
agent-browser doctor

# If you have Chrome already (local Mac)
export AGENT_BROWSER_EXECUTABLE_PATH="/Applications/Google Chrome.app/Contents/MacOS/Google Chrome"
```

---

## Key Takeaways

| Topic | Reality |
|-------|---------|
| **Language** | Rust — 100% of runtime code |
| **Node.js role** | Package distribution only — zero runtime |
| **Browser protocol** | CDP direct via WebSocket (tokio-tungstenite) |
| **Skills** | Markdown docs embedded in binary — teach agents domain knowledge |
| **Token savings** | 93% vs Playwright MCP — the primary design goal |
| **Daemon** | Persistent Rust process — eliminates cold-start between commands |
| **Refs** | `@e1`, `@e2` — stale after every DOM change, always re-snapshot |
| **Distribution** | npm package wrapping platform-specific Rust binaries |
