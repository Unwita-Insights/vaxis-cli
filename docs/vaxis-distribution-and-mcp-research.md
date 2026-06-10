# Vaxis CLI Distribution & Claude Integration — Research Analysis

> Research date: June 2026  
> Scope: All cross-platform distribution options for the `vaxis` Rust CLI + all ways to expose it as a Claude skill via MCP

---

## TL;DR Recommendation

| Phase | What | Why |
|---|---|---|
| **Phase 1 (Week 1)** | `cargo dist init` + `vaxis mcp-server` subcommand | Free cross-platform distribution + immediate Claude Desktop/Code integration |
| **Phase 2 (Week 2–3)** | npm package + Scoop bucket | Zero-install `npx` path for Claude Code, Windows coverage |
| **Phase 3 (Later)** | Cloudflare Worker remote MCP + WinGet | Claude.ai users, mainstream Windows |

---

## Part 1: CLI Distribution Methods

### 1. cargo-dist — Recommended Backbone

**What it is:** Official Rust release automation tool from axodotdev. One `cargo dist init` command generates your entire release pipeline.

**What it outputs automatically:**
- GitHub Actions CI (cross-compiles for all 5 targets on tag push)
- Shell one-liner installer (`curl -fsSL https://... | sh`)
- PowerShell installer for Windows
- Homebrew formula (auto-committed to your tap repo)
- Windows MSI installer
- npm wrapper package
- GitHub Release tarballs + ZIP files + SHA256 checksums
- SBOM / `cargo-auditable` integration

**Setup:**
```bash
cargo install cargo-dist
cargo dist init
# Follow the interactive wizard — select: GitHub CI, shell installer, Homebrew, MSI, npm
# Create github.com/your-org/homebrew-vaxis repo
# Push a tag → everything else is automatic
git tag v0.1.0 && git push --tags
```

**Cross-compilation:** Uses `cargo-zigbuild` for Linux targets, `cargo-xwin` for Windows. macOS arm64/x64 both supported.

**Cost:** ~30 min initial setup. Zero ongoing maintenance — push a tag, CI ships.

**Tradeoffs:**

| Pro | Con |
|---|---|
| Single tool generates everything | Opinionated — hard to customize outside its model |
| Zero ongoing maintenance after init | MSI requires WiX v3 on the runner |
| Homebrew formula auto-committed | Homebrew tap still needs a separate GitHub repo |
| All 5 targets cross-compiled | Self-bootstraps from previous version (quirk) |

---

### 2. Homebrew Tap

**What it is:** A custom GitHub repo (`homebrew-vaxis`) containing a Ruby formula that points to your GitHub Release binaries.

**User experience:**
```bash
brew install your-org/vaxis/vaxis
brew upgrade vaxis
```

**How it works:** Homebrew reads the Ruby formula, detects `OS.mac?`/`OS.linux?` + `Hardware::CPU.arm?`, downloads the right binary, places it in `/opt/homebrew/bin/`.

**With cargo-dist:** Formula is auto-generated and auto-committed on every release — effectively free.

**Without cargo-dist:** You write the Ruby formula, update SHA256 hashes in CI on each release. ~2–4 hours setup.

**Coverage:** macOS + Linux only. No Windows.

**Tradeoffs:**

| Pro | Con |
|---|---|
| Native experience for Mac devs | macOS/Linux only |
| `brew upgrade vaxis` just works | Getting into `homebrew-core` requires significant adoption |
| Trusted, familiar distribution channel | Your own tap is fine — no need for core |

---

### 3. npm Global Package (critical for Claude Code integration)

This is the most important distribution method for making vaxis work as a Claude skill with zero user installation.

**Two sub-patterns:**

#### Pattern A — `optionalDependencies` (modern, recommended)
Used by esbuild, biome, turborepo, SWC, LightningCSS.

- Create scoped packages per platform: `@vaxis/cli-darwin-arm64`, `@vaxis/cli-linux-x64`, etc.
- Each contains only a `package.json` (with `os` + `cpu` fields) and the binary
- Main `vaxis` package lists all as `optionalDependencies`
- npm/pnpm/yarn automatically installs only the matching platform binary
- A tiny `bin/vaxis.js` wrapper does `require.resolve()` to find the binary and `spawnSync` it
- **Works even when `--ignore-scripts` is set** (critical in corporate environments)

#### Pattern B — `postinstall` script (agent-browser's approach)
- Single `vaxis` package with `scripts.postinstall`
- `postinstall.js` detects `process.platform` + `process.arch`, downloads/copies the correct binary, sets `chmod +x`
- Simpler to set up (1 package instead of 6+)
- Can be blocked by security policies that disable postinstall scripts

**Why npm is critical for Claude Code:**
```json
// .mcp.json in any project repo — zero binary install needed
{
  "mcpServers": {
    "vaxis": {
      "command": "npx",
      "args": ["-y", "vaxis", "mcp-server"]
    }
  }
}
```

**Setup cost:** High for initial setup (~1–2 days). Zero ongoing — CI publishes on tag.

**Reference:** [orhun.dev — Packaging Rust for npm](https://blog.orhun.dev/packaging-rust-for-npm/)

---

### 4. curl One-Liner

```bash
curl -fsSL https://install.vaxis.app | sh
```

**How it works:** Shell script detects `uname -s` + `uname -m`, maps to the correct GitHub Release asset, downloads, verifies SHA256, installs to `$HOME/.local/bin`.

**cargo-dist generates this for free.** The shell installer and a PowerShell equivalent are generated as release artifacts automatically.

**Tradeoffs:**

| Pro | Con |
|---|---|
| No package manager needed | `curl \| sh` security concerns (mitigated by SHA256 check) |
| Works in CI pipelines | No upgrade mechanism |
| Free with cargo-dist | Linux/macOS only (PowerShell covers Windows separately) |

---

### 5. GitHub Releases (baseline — always needed)

Every other method builds on top of this. cargo-dist uploads platform tarballs + ZIP files + SHA256 checksums automatically.

Also enables `cargo-binstall` for free: `cargo binstall vaxis` detects your platform and downloads the pre-built binary from GitHub Releases instead of compiling.

---

### 6. Windows-Specific Options

| Method | How | Cost | Audience |
|---|---|---|---|
| **MSI** | cargo-dist auto-generates | Free | Windows GUI users |
| **Scoop** (self-hosted bucket) | JSON manifest in your own GitHub repo | Low | Windows devs, no admin required |
| **WinGet** | PR to Microsoft's `winget-pkgs` repo, reviewed | Medium (1–2 week review) | Mainstream Windows |
| **Chocolatey** | NuGet XML manifest, community review | Medium | Enterprise Windows |

**Recommendation:** Start with Scoop (self-hosted, no approval process, developer-friendly). Submit to WinGet when you have a stable v1.

**Scoop setup:**
```bash
# Create github.com/your-org/scoop-vaxis repo
# Add vaxis.json manifest
# Users run:
scoop bucket add vaxis https://github.com/your-org/scoop-vaxis
scoop install vaxis
```

---

### 7. Docker (secondary — CI use only)

```bash
docker run ghcr.io/your-org/vaxis:latest diagrams list
```

Good for reproducible CI pipelines. Poor daily-driver UX (slow startup, need volume mounts for file access). Not a primary distribution method.

---

### Complete Distribution Comparison

| Method | Platforms | Setup Cost | UX | Upgrade | Best for |
|---|---|---|---|---|---|
| **cargo-dist** | All | Very Low | — (generates others) | Via shell/MSI/brew | Automation backbone |
| **Homebrew** | macOS + Linux | Low (free with cargo-dist) | Excellent | `brew upgrade` | Mac/Linux devs |
| **npm optionalDeps** | All | High (1–2 days) | Good | `npm update -g` | JS devs, `npx` MCP |
| **npm postinstall** | All | Medium (1 day) | Good | `npm update -g` | JS devs (simpler setup) |
| **curl one-liner** | macOS + Linux | Free (cargo-dist) | Good | Manual re-run | Quick adoption |
| **GitHub Releases** | All | Zero | Manual | None | Baseline for all |
| **cargo-binstall** | All | Zero | Good | Via binstall | Rust devs |
| **Scoop** | Windows | Low | Good | `scoop update` | Windows devs |
| **WinGet** | Windows | Medium | Good | `winget upgrade` | Mainstream Windows |
| **Chocolatey** | Windows | Medium | Good | `choco upgrade` | Enterprise Windows |
| **MSI** | Windows | Free (cargo-dist) | Excellent | Windows installer | Windows GUI users |
| **Docker** | All | Low | Poor | `docker pull` | CI pipelines only |

---

## Part 2: Claude Skill Integration via MCP

### What MCP Is

**Model Context Protocol (MCP)** is Anthropic's open standard for connecting AI models to external tools. Created 2024, donated to Linux Foundation/Agentic AI Foundation December 2025. Think of it as USB-C for AI tool integration.

**Core primitives:**
- **Tools** — functions Claude can call (like function calling)
- **Resources** — data sources Claude can read
- **Prompts** — reusable prompt templates

---

### Transport Types

| Transport | How | Best for |
|---|---|---|
| **stdio (local)** | Claude spawns your binary as a child process; JSON-RPC over stdin/stdout | Local CLIs, tools needing filesystem access |
| **Streamable HTTP (remote)** | Public HTTPS endpoint at `https://your-server.com/mcp`; HTTP POST/GET | SaaS tools, shared services |
| **SSE (legacy)** | Server-Sent Events — being deprecated as of 2025 spec | Avoid for new implementations |

---

### Option A: Local stdio MCP in Rust — Best for Claude Desktop/Code

Add a `vaxis mcp-server` subcommand to the existing binary. Uses the `rmcp` crate (official Rust MCP SDK, 4.7M+ downloads on crates.io).

**`Cargo.toml`:**
```toml
rmcp = { version = "0.16", features = ["server", "transport-io", "macros"] }
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
schemars = "0.8"
```

**`src/commands/mcp.rs` (skeleton):**
```rust
use rmcp::{ServerHandler, model::*, schemars, tool};

#[derive(Clone)]
pub struct VaxisServer {
    base_url: String,
    token: String,
}

#[rmcp::tool(tool_box)]
impl VaxisServer {
    #[tool(description = "List all apps for the current user")]
    async fn list_apps(&self) -> Result<CallToolResult, McpError> {
        // call vaxis API, return result
    }

    #[tool(description = "List diagrams for an app")]
    async fn list_diagrams(&self, #[tool(param)] app_id: String) -> Result<CallToolResult, McpError> {
        // call vaxis API
    }
}

pub async fn run_mcp_server(base_url: String, token: String) -> anyhow::Result<()> {
    // CRITICAL: Never println!() in MCP mode — it corrupts the JSON-RPC stream
    // All logging must go to stderr via tracing
    tracing_subscriber::fmt().with_writer(std::io::stderr).init();

    let server = VaxisServer { base_url, token };
    let transport = rmcp::transport::io::stdio();
    server.serve(transport).await?.waiting().await?;
    Ok(())
}
```

**Claude Desktop config** (`~/Library/Application Support/Claude/claude_desktop_config.json`):
```json
{
  "mcpServers": {
    "vaxis": {
      "command": "/usr/local/bin/vaxis",
      "args": ["mcp-server"],
      "env": {
        "VAXIS_TOKEN": "your-token-here"
      }
    }
  }
}
```

**Claude Code (project-wide, committed to repo)** — create `.mcp.json` in project root:
```json
{
  "mcpServers": {
    "vaxis": {
      "command": "vaxis",
      "args": ["mcp-server"]
    }
  }
}
```

**Via `npx` (zero binary install — requires npm package):**
```json
{
  "mcpServers": {
    "vaxis": {
      "command": "npx",
      "args": ["-y", "vaxis", "mcp-server"]
    }
  }
}
```

**Add via CLI:**
```bash
claude mcp add --transport stdio vaxis -- /usr/local/bin/vaxis mcp-server
```

---

### Option B: Remote MCP on Cloudflare Workers — Best for Claude.ai users

Add an `/mcp` route to the existing `apps/api` Cloudflare Worker. No binary install needed — users paste a URL.

**Dependencies:**
```bash
npm install @modelcontextprotocol/sdk
# OR
npm install @cloudflare/workers-mcp
```

**`src/routes/mcp.ts` (outline):**
```typescript
import { McpAgent } from '@cloudflare/agents/mcp';
import { MCP } from '@cloudflare/workers-mcp';

export class VaxisMCP extends McpAgent {
  server = new MCP({ name: 'vaxis', version: '1.0.0' });

  async init() {
    this.server.tool('list_apps', 'List all apps', {}, async () => {
      // call your existing D1-backed logic
    });

    this.server.tool('list_diagrams', 'List diagrams for an app',
      { app_id: z.string() },
      async ({ app_id }) => { /* ... */ }
    );
  }
}
```

**`src/index.ts`:**
```typescript
app.all('/mcp/*', (c) => VaxisMCP.mount('/mcp').fetch(c.req.raw, c.env));
```

**Auth:** Read `Authorization: Bearer <token>` header — same token the CLI uses from `~/.vaxis/config.toml`.

**Claude Desktop** (uses `mcp-remote` bridge for clients without native Streamable HTTP support):
```json
{
  "mcpServers": {
    "vaxis": {
      "command": "npx",
      "args": ["mcp-remote", "https://api.vaxis.app/mcp"]
    }
  }
}
```

**Claude.ai:** Settings → Integrations → + Add → paste `https://api.vaxis.app/mcp`

---

### Option C: Claude API MCP Connector

For developers building apps on the Claude API — connect to your remote MCP server in the API call itself:

```bash
curl https://api.anthropic.com/v1/messages \
  -H "x-api-key: $ANTHROPIC_KEY" \
  -H "anthropic-beta: mcp-client-2025-11-20" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "claude-opus-4-7",
    "mcp_servers": [
      {"type": "url", "url": "https://api.vaxis.app/mcp", "name": "vaxis"}
    ],
    "tools": [{"type": "mcp_toolset", "mcp_server_name": "vaxis"}],
    "messages": [{"role": "user", "content": "List my diagrams"}]
  }'
```

Requires: public HTTPS endpoint + OAuth or Bearer token auth.

---

### MCP Options Comparison

| Approach | User setup | Where runs | Auth | Best for |
|---|---|---|---|---|
| **Local stdio (Rust binary)** | Install binary + add to config | User's machine | Token in env | Claude Desktop, Claude Code, power users |
| **npx MCP server** | Add JSON config (no binary) | User's machine via Node | Token in env | Zero-install Claude Code |
| **Cloudflare Worker remote** | Paste URL or `npx mcp-remote URL` | Cloudflare edge | OAuth / Bearer | Claude.ai, no install |
| **Claude API connector** | API call config | Your HTTPS server | Bearer token | Developers building Claude apps |

---

## Part 3: Vercel agent-browser — How They Built It

[GitHub: vercel-labs/agent-browser](https://github.com/vercel-labs/agent-browser)  
[npm: agent-browser](https://www.npmjs.com/package/agent-browser)

### What It Is

Browser automation CLI for AI agents. Rust daemon controls Chrome via Chrome DevTools Protocol (CDP). No Playwright or Node.js runtime required.

- **85% Rust** (CLI + daemon)
- **12% TypeScript** (tooling, postinstall, npm packaging)

### How They Distribute

**Primary: npm** (`npm install -g agent-browser`)

Architecture:
- Single `agent-browser` npm package
- `package.json` `bin` points to `./bin/agent-browser.js` (a tiny Node shim)
- `scripts.postinstall` runs `scripts/postinstall.js` which:
  1. Detects `process.platform` + `process.arch`
  2. Copies the correct pre-compiled Rust binary from the package
  3. Sets `chmod +x`
- Binaries bundled for: macOS arm64, macOS x64, Linux arm64, Linux x64, Windows x64

**Secondary:** Homebrew (`brew install agent-browser`), Cargo (`cargo install agent-browser`)

### Their CI/Release Pipeline

**Trigger:** On PR merge, CI compares `package.json` version vs. published npm version. If they differ, the release pipeline starts.

**Build matrix:** 5 platform targets in parallel on GitHub Actions.

**Per-platform steps:**
1. Set up Rust toolchain for the target triple
2. `cargo build --release --target <triple>`
3. Place binary in the npm package `bin/<platform>/` directory
4. Set executable permissions

**Security:**
- **OIDC trusted publishing** — no stored npm tokens. GitHub Actions' identity is trusted directly by npm via `NODE_AUTH_TOKEN: ${{ secrets.NPM_TOKEN }}` replaced by OIDC
- Binary file size validation to catch empty/corrupt builds

**Release notes:** Auto-extracted from `CHANGELOG.md` between `<!-- release:start -->` and `<!-- release:end -->` markers.

### Key Lesson: postinstall vs. optionalDependencies

agent-browser uses postinstall (simpler, 1 package). The tradeoff vs. optionalDependencies:

| | postinstall (agent-browser) | optionalDependencies (esbuild/biome) |
|---|---|---|
| Number of npm packages | 1 | 6+ |
| Setup complexity | Low | High |
| `--ignore-scripts` safe | No | Yes |
| Security scrutiny | Higher | Lower |
| Download size | All binaries bundled | Only your platform |

**For vaxis in 2026:** The optionalDependencies pattern is preferred for new tools — `--ignore-scripts` is commonly set in corporate environments that are installing dev tools. But starting with postinstall (agent-browser's approach) is faster to ship.

### Their Claude Code Skill Pattern

They use SKILL.md files (same system as vaxis skills):
- `skills/agent-browser/SKILL.md` — minimal stub that redirects to `agent-browser skills get core`
- `skill-data/core/SKILL.md` — the full tool documentation loaded by Claude Code
- When Claude Code loads the skill, it reads these files to understand what the tool does

This is separate from MCP — it's a documentation-as-code pattern.

---

## Implementation Roadmap

### Week 1 — Baseline distribution + local MCP

1. **`cargo dist init`**
   ```bash
   cargo install cargo-dist
   cd vaxis-cli
   cargo dist init
   # Select: GitHub CI, shell installer, Homebrew, MSI, npm
   ```
   Creates: `release.yml`, Homebrew formula automation, shell/PowerShell installer, MSI.

2. **Create `homebrew-vaxis` repo** at `github.com/your-org/homebrew-vaxis`
   cargo-dist will auto-commit the formula here on each release.

3. **Add `vaxis mcp-server` subcommand** in `src/commands/mcp.rs` using `rmcp`
   - Add `rmcp` + `tokio` to `Cargo.toml`
   - Register `vaxis mcp-server` in `src/cli.rs`
   - Expose tools: `list_apps`, `list_diagrams`, `show_diagram`, `generate_diagram`
   - Add `.mcp.json` to the vaxis-cli repo root for instant team adoption

4. **Tag and push:**
   ```bash
   git tag v0.1.0 && git push --tags
   ```
   CI ships: Homebrew formula, shell installer, MSI, GitHub Release binaries.

---

### Week 2–3 — npm + Windows + zero-install MCP

5. **npm package** (postinstall approach to start fast, migrate to optionalDeps later)
   - Follow agent-browser's `postinstall.js` pattern
   - Add OIDC trusted publishing to GitHub Actions
   - Publish `vaxis` to npm
   - Users can then: `npx vaxis mcp-server` with no binary install

6. **Scoop bucket** — create `github.com/your-org/scoop-vaxis`
   ```json
   {
     "version": "0.1.0",
     "url": "https://github.com/your-org/vaxis-cli/releases/download/v0.1.0/vaxis-x86_64-pc-windows-msvc.zip",
     "hash": "SHA256:<hash>",
     "bin": "vaxis.exe"
   }
   ```
   Add SHA256 update step to release CI.

---

### Week 4+ — Remote MCP + mainstream Windows

7. **Cloudflare Worker remote MCP** — add `/mcp` route to `apps/api`
   - Install `@cloudflare/workers-mcp` or `@modelcontextprotocol/sdk`
   - Create `src/routes/mcp.ts`
   - Deploy via `wrangler deploy`

8. **WinGet submission** — submit YAML manifest to `winget-pkgs` via PR
   - Use `komac` to auto-generate manifests from GitHub Releases
   - Expect 1–2 week review

---

## References

- [cargo-dist docs](https://axodotdev.github.io/cargo-dist/)
- [Packaging Rust for npm — orhun.dev](https://blog.orhun.dev/packaging-rust-for-npm/)
- [Publishing Binaries on npm — Sentry Blog](https://blog.sentry.io/publishing-binaries-on-npm)
- [vercel-labs/agent-browser — GitHub](https://github.com/vercel-labs/agent-browser)
- [rmcp — Rust MCP SDK](https://crates.io/crates/rmcp)
- [Build a Remote MCP Server — Cloudflare Agents Docs](https://developers.cloudflare.com/agents/guides/remote-mcp-server/)
- [MCP Connector — Claude API Docs](https://platform.claude.com/docs/en/agents-and-tools/mcp-connector)
- [Connect Claude Code to tools via MCP — Claude Code Docs](https://code.claude.com/docs/en/mcp)
- [cloudflare/workers-mcp — GitHub](https://github.com/cloudflare/workers-mcp)
- [cargo-binstall — GitHub](https://github.com/cargo-bins/cargo-binstall)
