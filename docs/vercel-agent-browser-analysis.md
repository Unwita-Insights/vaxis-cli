# Vercel agent-browser — Deep Analysis

**Repo:** https://github.com/vercel-labs/agent-browser
**Version:** 0.27.0
**License:** Apache-2.0
**Language split:** Rust 85.4% · TypeScript 12.2% · Other 2.4%

---

## Table of Contents

1. [What it is](#what-it-is)
2. [Why Vercel built it](#why-vercel-built-it)
3. [Architecture](#architecture)
4. [Technology Stack](#technology-stack)
5. [Core Innovation — The Ref System](#core-innovation--the-ref-system)
6. [All Commands (50+)](#all-commands-50)
7. [All Global Flags](#all-global-flags)
8. [Skills System](#skills-system)
9. [AI Integration](#ai-integration)
10. [Security & Agent Safeguards](#security--agent-safeguards)
11. [Installation](#installation)
12. [How to use with Claude Code](#how-to-use-with-claude-code)

---

## What it is

**agent-browser** is a native Rust CLI for browser automation built specifically for AI agents. It is not a wrapper around Playwright or Puppeteer — it communicates directly with browsers via Chrome DevTools Protocol (CDP) with a persistent daemon.

```bash
# Install
npm install -g agent-browser

# Basic usage
agent-browser open https://example.com
agent-browser snapshot -i          # see interactive elements as @refs
agent-browser click @e1            # click element 1
agent-browser me                   # show current page info
```

---

## Why Vercel built it

Existing browser automation tools were designed for humans writing scripts, not for LLMs calling tools. Key problems:

| Problem | Impact |
|---------|--------|
| Full DOM dumps as output | 3000–5000 tokens per page — expensive and slow |
| Fragile CSS/XPath selectors | Break when sites update, agents get stuck |
| Node.js runtime per command | Slow cold start, heavy memory usage |
| No agent-native lifecycle | No daemon, no session persistence by default |
| MCP overhead | Full page state returned even for simple actions |

**Vercel's solution:** A Rust daemon that boots once and persists, combined with a ref-based snapshot system that returns 200–400 tokens instead of thousands.

**Result:** 93% context window savings vs MCP-based alternatives.

---

## Architecture

```
┌──────────────────────────────────────────────────────────┐
│  CLI (Rust binary)                                       │
│  - Parses subcommands and flags                          │
│  - Sends requests to daemon via IPC                      │
│  - Auto-launches daemon on first command                 │
└──────────────────────────────────────────────────────────┘
                        ↓  IPC
┌──────────────────────────────────────────────────────────┐
│  Daemon (Pure Rust, persistent)                          │
│  - Chrome DevTools Protocol (CDP) client                 │
│  - Browser instance management                          │
│  - Session + cookie persistence                         │
│  - Accessibility tree generation                        │
│  - Network interception & mocking                       │
│  - WebSocket streaming                                  │
└──────────────────────────────────────────────────────────┘
                        ↓  CDP
┌──────────────────────────────────────────────────────────┐
│  Browser Engine                                          │
│  - Chrome for Testing (default, auto-downloaded)         │
│  - Lightpanda (alternative, lighter)                    │
│  - Cloud: Browserless / Browserbase / Browser Use /     │
│           Kernel / AWS AgentCore / iOS Safari           │
└──────────────────────────────────────────────────────────┘
```

### Why a persistent daemon?

```
Without daemon:  each command = cold start = 200-500ms overhead
With daemon:     each command = IPC message = ~5ms overhead

For an agent making 50 actions per task:
  Without daemon: 50 × 300ms = 15 seconds of pure startup time
  With daemon:    50 × 5ms   = 0.25 seconds
```

---

## Technology Stack

### Core

| Component | Technology |
|-----------|-----------|
| CLI binary | Rust |
| Async runtime | Tokio |
| Browser protocol | Chrome DevTools Protocol (CDP) direct |
| JSON serialization | Serde + serde_json |
| CLI argument parsing | Clap |
| Dashboard UI | TypeScript + React |
| Build system | Cargo (Rust) + pnpm |
| Package manager | pnpm ≥11.0.0 |
| Node.js (build only) | ≥24.0.0 — NOT needed at runtime |

### Browser engines supported

| Engine | Command | Use case |
|--------|---------|---------|
| Chrome for Testing | default | Full web compatibility |
| Lightpanda | `--engine lightpanda` | Lighter, faster for simple pages |
| Browserless | `--provider browserless` | Cloud, no local Chrome needed |
| Browserbase | `--provider browserbase` | Cloud + CAPTCHA solving |
| Browser Use | `--provider browseruse` | Cloud |
| AWS AgentCore | `--provider agentcore` | AWS-native deployments |
| iOS Safari | `--provider ios` | Mobile testing via Appium |

---

## Core Innovation — The Ref System

Instead of returning full DOM or accessibility tree dumps, agent-browser assigns short **element references** (`@e1`, `@e2`) to interactive elements in a snapshot.

### Traditional MCP/Playwright approach

```json
{
  "role": "WebArea",
  "children": [
    {
      "role": "button",
      "name": "Submit",
      "description": "Submit the form",
      "boundingBox": { "x": 120, "y": 340, "width": 80, "height": 32 },
      "attributes": { "class": "btn btn-primary", "id": "submit-btn" }
    }
    // ... hundreds more nodes
  ]
}
// Token count: ~3000–5000
```

### agent-browser snapshot output

```
interactive elements:
  @e1  button  "Submit"
  @e2  input   type=text  placeholder="Search"
  @e3  link    "Sign in"  url=https://example.com/login
  @e4  select  "Country"  options=[India, USA, UK]

// Token count: ~200–400
```

Then the agent simply does:
```bash
agent-browser click @e1
```

### Why refs are better than selectors

| | CSS selector | XPath | agent-browser ref |
|---|---|---|---|
| Stability | Breaks on class change | Breaks on DOM change | Stable within snapshot |
| Token cost | Low (just the selector) | Low | Low |
| Agent-friendly | Must know exact selector | Must know exact path | LLM reads snapshot, picks ref |
| After navigation | Invalid | Invalid | Re-snapshot, new refs |

---

## All Commands (50+)

### Navigation

```bash
agent-browser open <url>          # open URL in browser
agent-browser goto <url>          # navigate current tab
agent-browser back                # browser back
agent-browser forward             # browser forward
agent-browser reload              # refresh page
agent-browser close               # close browser
agent-browser quit / exit         # stop daemon + close
```

### Tab Management

```bash
agent-browser tab                 # list open tabs
agent-browser tab new [url]       # open new tab
agent-browser tab close [id]      # close tab by ID
agent-browser frame <selector>    # navigate into iframe
```

### Interaction

```bash
agent-browser click <sel>         # left click
agent-browser dblclick <sel>      # double click
agent-browser type <sel> <text>   # type into element (appends)
agent-browser fill <sel> <text>   # fill field (replaces)
agent-browser select <sel> <val>  # pick dropdown option
agent-browser check <sel>         # check checkbox
agent-browser uncheck <sel>       # uncheck checkbox
agent-browser hover <sel>         # hover over element
agent-browser focus <sel>         # focus element
agent-browser press <key>         # keyboard press (e.g. Enter, Tab)
agent-browser drag <src> <target> # drag and drop
agent-browser upload <sel> <file> # file upload
```

### Perception

```bash
agent-browser snapshot            # full accessibility tree
agent-browser snapshot -i         # interactive elements only (most token-efficient)
agent-browser snapshot -c         # include content/text nodes
agent-browser snapshot -d <n>     # limit depth to n levels
agent-browser snapshot -s "<sel>" # snapshot scoped to selector
agent-browser screenshot [path]   # take screenshot
agent-browser screenshot --annotate  # screenshot with element labels
```

### Data Extraction

```bash
agent-browser get text <sel>      # get visible text
agent-browser get html <sel>      # get innerHTML
agent-browser get attr <sel> <attr>  # get attribute value
agent-browser get value <sel>     # get input value
agent-browser get title           # page title
agent-browser get url             # current URL
agent-browser get box <sel>       # bounding box (x, y, w, h)
agent-browser get styles <sel>    # computed CSS styles
```

### Semantic Locators (accessibility-based)

```bash
agent-browser find role <role>           # by ARIA role
agent-browser find text <text>           # by visible text
agent-browser find label <label>         # by form label
agent-browser find placeholder <text>    # by input placeholder
agent-browser find alt <text>            # by image alt text
agent-browser find title <text>          # by title attribute
agent-browser find testid <id>           # by data-testid
```

### Waiting

```bash
agent-browser wait <selector>           # wait for element visible
agent-browser wait <ms>                 # pause N milliseconds
agent-browser wait --text "text"        # wait for text to appear
agent-browser wait --url "pattern"      # wait for URL match
agent-browser wait --load networkidle   # wait for network quiet
```

### Network

```bash
agent-browser network route <url>       # intercept requests to URL
agent-browser network requests          # view all tracked requests
agent-browser network har start         # start HAR recording
agent-browser network har stop [path]   # stop and save HAR
```

### Storage & Cookies

```bash
agent-browser cookies get              # list all cookies
agent-browser cookies set <n> <v>      # set a cookie
agent-browser cookies clear            # clear all cookies
agent-browser storage local            # view localStorage
agent-browser storage session          # view sessionStorage
```

### Auth & Session Persistence

```bash
agent-browser state save <name>        # save cookies + storage to file
agent-browser state load <name>        # restore saved session
agent-browser state list               # list saved states
agent-browser state show <name>        # inspect a state
agent-browser state clear              # delete all states
agent-browser state rename <old> <new> # rename a state
agent-browser state clean --older-than <days>  # cleanup old states
```

### DevTools & Debugging

```bash
agent-browser eval <js>                # execute JavaScript
agent-browser console                  # view console messages
agent-browser errors                   # view JS exceptions
agent-browser trace start              # start trace recording
agent-browser trace stop [path]        # stop and save trace
agent-browser profiler start           # start CPU profiler
agent-browser profiler stop            # stop profiler
agent-browser highlight <sel>          # highlight element visually
agent-browser inspect                  # open Chrome DevTools
agent-browser vitals                   # Web Vitals (LCP, CLS, TTFB)
```

### React-specific

```bash
agent-browser react tree               # React component tree
agent-browser react inspect <sel>      # inspect React component props/state
```

### Diff & Comparison

```bash
agent-browser diff snapshot            # compare two accessibility trees
agent-browser diff screenshot --baseline <img>  # visual pixel diff
agent-browser diff url <url1> <url2>   # compare two pages
```

### Browser Configuration

```bash
agent-browser set viewport <w> <h>    # window size
agent-browser set device <name>       # device emulation (e.g. "iPhone 16 Pro")
agent-browser set geo <lat> <lng>     # spoof geolocation
agent-browser set headers <json>      # set HTTP headers
agent-browser set credentials <u> <p> # HTTP basic auth
agent-browser set media dark|light    # color scheme
```

### Batch & Chat (AI mode)

```bash
agent-browser batch "cmd1" "cmd2"     # run multiple commands
agent-browser batch --json < cmds.json  # batch from JSON input
agent-browser chat "<instruction>"    # natural language, single shot
agent-browser chat                    # interactive AI REPL
```

### Setup & Info

```bash
agent-browser install                 # install browser engine
agent-browser upgrade                 # upgrade to latest version
agent-browser doctor                  # check setup health
agent-browser doctor --fix            # auto-fix issues
agent-browser skills                  # browse available skills
agent-browser skills get <name>       # download a skill
agent-browser connect <port>          # connect to existing CDP session
agent-browser stream enable|disable   # WebSocket streaming
agent-browser stream status           # streaming status
```

---

## All Global Flags

```bash
--session <name>              # isolated browser session (named)
--session-name <name>         # auto-save/restore session by name
--profile <name|path>         # Chrome profile
--state <path>                # load saved auth state (cookies + localStorage)
--headers <json>              # HTTP headers for all requests
--executable-path <path>      # custom browser binary
--extension <path>            # load Chrome extension (repeatable)
--enable <feature>            # built-in init scripts (e.g. react-devtools)
--engine chrome|lightpanda    # browser engine selection
--provider <name>             # cloud provider (browserless, browserbase, etc.)
--json                        # JSON output format (machine-readable)
--headed                      # show browser window (not headless)
--device <name>               # device emulation
--color-scheme <scheme>       # dark / light / no-preference
--user-agent <ua>             # custom User-Agent
--proxy <url>                 # proxy server (supports auth)
--ignore-https-errors         # skip SSL validation
--download-path <path>        # file download directory
--allowed-domains <list>      # domain allowlist (agent safety)
--action-policy <path>        # policy file restricting actions
--confirm-actions <list>      # require confirmation for listed actions
--content-boundaries          # wrap output in boundary markers
--max-output <chars>          # truncate long outputs
--debug                       # debug logging
--verbose / -v                # verbose (chat mode)
--quiet / -q                  # quiet (chat mode)
```

### Key environment variables

```bash
AGENT_BROWSER_SESSION="default"
AGENT_BROWSER_HEADED=true
AGENT_BROWSER_PROVIDER="browserbase"
AGENT_BROWSER_IDLE_TIMEOUT_MS=300000
AGENT_BROWSER_ENCRYPTION_KEY="..."       # AES-256-GCM for state files
AGENT_BROWSER_STATE_EXPIRE_DAYS=7
AGENT_BROWSER_SKILLS_DIR="/custom/path"
AI_GATEWAY_API_KEY="..."                  # Vercel AI Gateway for chat
AI_GATEWAY_MODEL="claude-sonnet-4-6"     # model for chat mode
```

---

## Skills System

Skills are reusable automation modules stored in `skill-data/`:

```
skill-data/
├── core/           # fundamental browser automation capabilities
├── agentcore/      # AWS Bedrock AgentCore integration
├── dogfood/        # internal testing/validation
├── electron/       # Electron app automation
├── slack/          # Slack automation
└── vercel-sandbox/ # Vercel environment integration
```

### Browse and install skills

```bash
agent-browser skills                    # list available skills
agent-browser skills get slack          # install Slack skill
agent-browser skills get electron       # install Electron skill
```

Skills are composable — they wrap common sequences of commands into reusable named actions.

---

## AI Integration

### With Claude Code (recommended)

agent-browser is designed to be called as a shell tool by Claude Code:

```
You: "Go to HackerNews and summarize the top 5 posts"

Claude Code runs:
  agent-browser open https://news.ycombinator.com
  agent-browser snapshot -i
  agent-browser get text @e1   # reads top post
  agent-browser get text @e2
  ... (summarizes)
```

### Chat mode (built-in AI)

```bash
# Single instruction
agent-browser chat "Find all products under ₹500 on this page"

# Interactive REPL
agent-browser chat
> What's on this page?
> Click the login button
> Fill in email as test@example.com
```

Chat mode connects to Vercel AI Gateway — configure the model via:
```bash
export AI_GATEWAY_MODEL="claude-sonnet-4-6"
export AI_GATEWAY_API_KEY="your-key"
```

### Supported AI integrations

| Tool | Support |
|------|---------|
| Claude Code | Native (`.claude-plugin/` config) |
| Cursor | Shell command execution |
| GitHub Copilot | CLI commands |
| Gemini | Via AI Gateway |
| Vercel AI Gateway | Powers built-in chat mode |

---

## Security & Agent Safeguards

For safe deployment of autonomous agents:

```bash
# Restrict to specific domains only
agent-browser --allowed-domains "example.com,api.example.com" open https://example.com

# Block sensitive actions via policy file
agent-browser --action-policy ./policy.json open https://example.com

# Require human confirmation for destructive actions
agent-browser --confirm-actions "submit,delete,logout" open https://example.com

# Wrap output in parse-friendly markers
agent-browser --content-boundaries snapshot -i
```

### Encrypted state files

Saved sessions (cookies, localStorage) are encrypted with AES-256-GCM:

```bash
export AGENT_BROWSER_ENCRYPTION_KEY="your-256-bit-key"
agent-browser state save production-login
```

---

## Installation

```bash
# npm (recommended)
npm install -g agent-browser

# Homebrew (macOS)
brew install agent-browser

# Cargo (requires Rust)
cargo install agent-browser

# Local npm
npm install agent-browser

# From source
git clone https://github.com/vercel-labs/agent-browser
pnpm install && pnpm build && pnpm link
```

### Platforms

| OS | Architecture | Supported |
|----|-------------|-----------|
| macOS | ARM64 (Apple Silicon) | ✓ |
| macOS | x64 (Intel) | ✓ |
| Linux | ARM64 | ✓ |
| Linux | x64 | ✓ |
| Windows | x64 | ✓ |

---

## How to use with Claude Code

### Option A — Direct shell calls

Claude Code calls `agent-browser` as a shell command. No config needed.

```
You: "Scrape the pricing page of linear.app"

Claude internally runs:
  Bash: agent-browser open https://linear.app/pricing
  Bash: agent-browser snapshot
  Bash: agent-browser get text @e5
```

### Option B — Add as MCP server

```bash
# Add to Claude Code settings
claude mcp add agent-browser -- agent-browser mcp
```

Then Claude sees browser tools natively without shell calls.

### Option C — Use in a Rust project (like our claude-cli)

From our Rust CLI, shell out to agent-browser:

```rust
use std::process::Command;

fn browser_snapshot() -> String {
    let output = Command::new("agent-browser")
        .args(["snapshot", "-i"])
        .output()
        .expect("agent-browser not found");
    String::from_utf8_lossy(&output.stdout).to_string()
}

fn browser_click(ref_id: &str) {
    Command::new("agent-browser")
        .args(["click", ref_id])
        .status()
        .expect("click failed");
}
```

---

## Token efficiency comparison

| Tool | Tokens per snapshot | Approach |
|------|-------------------|---------|
| agent-browser snapshot -i | ~200–400 | Ref-based compact text |
| Playwright MCP | ~3000–5000 | Full accessibility JSON |
| Screenshot-based agents | ~800–1500 | Image tokens |
| Raw HTML dump | ~10,000+ | Full DOM text |

**93% token savings** vs MCP-based Playwright — the primary reason Vercel built this tool.
