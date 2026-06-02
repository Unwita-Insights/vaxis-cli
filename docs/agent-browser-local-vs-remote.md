# agent-browser: Local vs Remote Machine Setup

## The Two Components

There are two separate things that often get confused:

```
agent-browser (CLI)       ← the controller, installed via npm
Chrome for Testing        ← the actual browser being controlled
```

`agent-browser` is just a remote control. It needs a real browser process to send commands to via Chrome DevTools Protocol (CDP).

```
agent-browser CLI
      ↓  CDP commands
Chrome / Chromium         ← renders pages, executes JS
      ↓
Web Page
```

---

## Local Machine (Mac / Windows)

On your local machine, Chrome is already installed. You don't need to download anything extra.

### Setup

```bash
# 1. Install CLI
npm install -g agent-browser

# 2. Point to your existing Chrome
export AGENT_BROWSER_EXECUTABLE_PATH="/Applications/Google Chrome.app/Contents/MacOS/Google Chrome"

# 3. Run
agent-browser open https://example.com
```

Or pass it inline per command:

```bash
agent-browser \
  --executable-path "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome" \
  open https://example.com
```

### Why skip `agent-browser install` locally

`agent-browser install` downloads **Chrome for Testing** — a separate Chromium build locked to a specific version. On a local machine you already have Chrome, so this is unnecessary.

---

## Remote Machine (Linux Server / EC2 / VPS)

On a remote server you have no GUI and no browser pre-installed. This is where `agent-browser install` is essential.

### Setup

```bash
# 1. Install CLI
npm install -g agent-browser

# 2. Download Chrome for Testing (no GUI needed)
agent-browser install

# 3. Verify everything is working
agent-browser doctor

# 4. Run in headless mode (no display on server)
agent-browser open https://example.com
```

Headless is the default on servers — no `--headed` flag needed.

### Why `agent-browser install` is needed remotely

| Reason | Explanation |
|--------|-------------|
| No Chrome pre-installed | Servers ship with no browser |
| No GUI / display | Chrome for Testing runs headless out of the box |
| Version locked | No auto-updates — stable for automation |
| Sandboxing tuned for CLI | Reduced sandboxing, better for headless environments |

---

## Chrome for Testing vs Regular Chrome

| | Regular Chrome | Chrome for Testing |
|---|---|---|
| Auto-updates | Yes — breaks pinned automation | No — version locked |
| Purpose | Daily browsing | Automation only |
| GUI required | Yes | No (headless-first) |
| Installed by | You / IT | `agent-browser install` |
| Best for | Local development | CI/CD, remote servers |

---

## Local vs Remote: Side-by-side

| | Local Mac | Remote Server |
|---|---|---|
| Chrome available | Yes (pre-installed) | No |
| `agent-browser install` | Skip | **Required** |
| `--executable-path` | Point to your Chrome | Not needed |
| `--headed` | Optional (can see window) | Always false (no display) |
| `agent-browser doctor` | Optional | Run to verify setup |

---

## Decision guide

```
Are you on a local machine with Chrome installed?
  YES → skip install, use --executable-path
  NO  → run agent-browser install first

Is there a display / GUI available?
  YES → --headed true  (can watch browser)
  NO  → headless (default, no flag needed)
```

---

## Practical remote workflow

```bash
# SSH into your server
ssh user@your-server

# Install node if not present
curl -fsSL https://fnm.vercel.app/install | bash
fnm use --install-if-missing 22

# Install agent-browser
npm install -g agent-browser

# Install Chrome for Testing
agent-browser install

# Verify
agent-browser doctor

# Start automating (headless by default)
agent-browser open https://example.com
agent-browser snapshot -i
agent-browser get title
```

---

## Cloud browser providers (alternative to install)

On a remote machine you can also skip local Chrome entirely and use a cloud provider:

```bash
# Use Browserless (cloud Chrome)
agent-browser --provider browserless open https://example.com

# Use Browserbase (cloud Chrome + CAPTCHA solving)
agent-browser --provider browserbase open https://example.com
```

With cloud providers, no `agent-browser install` is needed — the browser runs on their infrastructure, not your server.

| Provider | Best for |
|----------|---------|
| `browserless` | High-volume scraping |
| `browserbase` | CAPTCHA solving, session management |
| `browseruse` | AI-native browser tasks |
| `agentcore` | AWS-native deployments |

---

## Key takeaway

```
Local machine   →  Chrome already exists → use --executable-path
Remote machine  →  nothing installed    → agent-browser install
Cloud provider  →  no install at all    → --provider flag
```
