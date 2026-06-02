# Rust CLI with Google OAuth — Complete Guide

## Table of Contents

1. [Overview](#overview)
2. [How Google OAuth works in a CLI](#how-google-oauth-works-in-a-cli)
3. [Project Setup](#project-setup)
4. [Project Structure](#project-structure)
5. [Dependencies](#dependencies)
6. [Implementation Walkthrough](#implementation-walkthrough)
   - [main.rs — Entry point](#mainrs--entry-point)
   - [cli.rs — Command definitions](#clirs--command-definitions)
   - [config.rs — Store user data](#configrs--store-user-data)
   - [commands/login.rs — OAuth flow](#commandsloginrs--oauth-flow)
   - [commands/me.rs — Show profile](#commandsmers--show-profile)
   - [commands/logout.rs — Clear session](#commandslogoutrs--clear-session)
7. [Google Cloud Setup](#google-cloud-setup)
8. [Testing](#testing)
9. [What v0.2 adds](#what-v02-adds)

---

## Overview

This is a Rust-based CLI tool that authenticates users via **Google OAuth**, stores their profile locally, and exposes simple commands to interact with that profile.

Inspired by how professional CLIs handle auth:
- `gh auth login` — GitHub CLI
- `gcloud auth login` — Google Cloud CLI
- `goose` — Block's AI agent CLI (Rust)

### Commands

```bash
claude-cli login     # open browser → Google login → store profile
claude-cli me        # display stored name, email, avatar
claude-cli logout    # clear stored credentials
```

---

## How Google OAuth works in a CLI

Standard CLIs use the **PKCE (Proof Key for Code Exchange)** flow — designed for apps that cannot safely store a client secret (like native apps and CLIs).

```
┌─────────────────────────────────────────────────────────┐
│                    PKCE OAuth Flow                      │
└─────────────────────────────────────────────────────────┘

1. User runs: claude-cli login

2. CLI generates a random "code_verifier" and its hash "code_challenge"

3. CLI opens browser with Google auth URL:
   https://accounts.google.com/o/oauth2/auth
     ?client_id=YOUR_CLIENT_ID
     &redirect_uri=http://localhost:8080/callback
     &response_type=code
     &scope=openid email profile
     &code_challenge=<hash>
     &code_challenge_method=S256

4. User logs in and approves on Google's page

5. Google redirects to: http://localhost:8080/callback?code=AUTH_CODE

6. CLI's local HTTP server (tiny_http) catches the callback

7. CLI exchanges the code + code_verifier for an access token:
   POST https://oauth2.googleapis.com/token

8. CLI uses access token to fetch user profile:
   GET https://www.googleapis.com/oauth2/v2/userinfo

9. CLI saves profile to: ~/.config/claude-cli/config.toml

10. Done — user is logged in
```

---

## Project Setup

### 1. Create the Rust project

```bash
cargo new claude-cli
cd claude-cli
```

### 2. Add dependencies to Cargo.toml

```toml
[package]
name = "claude-cli"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "claude-cli"
path = "src/main.rs"

[dependencies]
clap = { version = "4", features = ["derive"] }
tokio = { version = "1", features = ["full"] }
reqwest = { version = "0.12", features = ["json"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
toml = "0.8"
oauth2 = "4"
tiny_http = "0.12"
open = "5"
colored = "2"
dirs = "5"
url = "2"
rand = "0.8"
sha2 = "0.10"
base64 = { version = "0.22", features = ["alloc"] }
```

---

## Project Structure

```
claude-cli/
├── Cargo.toml
└── src/
    ├── main.rs              # entry point — runs the CLI
    ├── cli.rs               # clap command definitions
    ├── config.rs            # read/write ~/.config/claude-cli/config.toml
    └── commands/
        ├── mod.rs
        ├── login.rs         # Google OAuth PKCE flow
        ├── me.rs            # display stored user profile
        └── logout.rs        # clear credentials
```

---

## Dependencies

| Crate | Version | Why |
|-------|---------|-----|
| `clap` | 4 | Argument parsing — subcommands, flags, help text |
| `tokio` | 1 | Async runtime for HTTP requests |
| `reqwest` | 0.12 | HTTP client — fetch Google user profile |
| `oauth2` | 4 | OAuth PKCE helpers — code verifier, challenge, token exchange |
| `tiny_http` | 0.12 | Minimal local HTTP server — catches OAuth callback |
| `open` | 5 | Opens the browser on any OS |
| `serde` + `toml` | - | Serialize/deserialize config file |
| `colored` | 2 | Color terminal output |
| `dirs` | 5 | Cross-platform home/config directory paths |

---

## Implementation Walkthrough

### main.rs — Entry point

```rust
mod cli;
mod config;
mod commands;

use clap::Parser;
use cli::{Cli, Commands};

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Login => commands::login::run().await,
        Commands::Me => commands::me::run(),
        Commands::Logout => commands::logout::run(),
    }
}
```

---

### cli.rs — Command definitions

```rust
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "claude-cli")]
#[command(about = "A CLI tool with Google authentication")]
#[command(version = "0.1.0")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Log in with your Google account
    Login,

    /// Show your stored profile details
    Me,

    /// Log out and clear stored credentials
    Logout,
}
```

---

### config.rs — Store user data

Reads and writes `~/.config/claude-cli/config.toml`.

```rust
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Config {
    pub user: Option<UserProfile>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UserProfile {
    pub name: String,
    pub email: String,
    pub picture: String,   // avatar URL
    pub access_token: String,
}

fn config_path() -> PathBuf {
    let mut path = dirs::config_dir().expect("cannot find config dir");
    path.push("claude-cli");
    path.push("config.toml");
    path
}

pub fn load() -> Config {
    let path = config_path();
    if !path.exists() {
        return Config::default();
    }
    let content = fs::read_to_string(&path).unwrap_or_default();
    toml::from_str(&content).unwrap_or_default()
}

pub fn save(config: &Config) {
    let path = config_path();
    fs::create_dir_all(path.parent().unwrap()).expect("cannot create config dir");
    let content = toml::to_string(config).expect("cannot serialize config");
    fs::write(&path, content).expect("cannot write config file");
}

pub fn clear() {
    let path = config_path();
    if path.exists() {
        fs::remove_file(path).expect("cannot remove config file");
    }
}
```

**Config file location:**

```
~/.config/claude-cli/config.toml     # Linux / Mac
C:\Users\<name>\AppData\Roaming\claude-cli\config.toml  # Windows
```

**Config file contents after login:**

```toml
[user]
name = "Mani Kumar"
email = "mani@example.com"
picture = "https://lh3.googleusercontent.com/..."
access_token = "ya29.a0AfH6SM..."
```

---

### commands/login.rs — OAuth flow

```rust
use colored::Colorize;
use std::io::Read;
use crate::config::{self, UserProfile};

// Your Google OAuth app credentials
const CLIENT_ID: &str = "YOUR_GOOGLE_CLIENT_ID";
const CLIENT_SECRET: &str = "YOUR_GOOGLE_CLIENT_SECRET";
const REDIRECT_URI: &str = "http://localhost:8080/callback";
const AUTH_URL: &str = "https://accounts.google.com/o/oauth2/auth";
const TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
const USERINFO_URL: &str = "https://www.googleapis.com/oauth2/v2/userinfo";

pub async fn run() {
    // Step 1: Generate PKCE code verifier and challenge
    let code_verifier = generate_code_verifier();
    let code_challenge = generate_code_challenge(&code_verifier);

    // Step 2: Build the authorization URL
    let auth_url = format!(
        "{}?client_id={}&redirect_uri={}&response_type=code\
         &scope=openid%20email%20profile\
         &code_challenge={}&code_challenge_method=S256",
        AUTH_URL, CLIENT_ID, REDIRECT_URI, code_challenge
    );

    // Step 3: Open browser
    println!("{}", "Opening browser for Google login...".cyan());
    open::that(&auth_url).expect("failed to open browser");

    // Step 4: Start local server and wait for callback
    println!("{}", "Waiting for Google to redirect back...".dimmed());
    let code = wait_for_callback();

    // Step 5: Exchange code for access token
    println!("{}", "Exchanging code for token...".dimmed());
    let access_token = exchange_code_for_token(&code, &code_verifier).await;

    // Step 6: Fetch user profile
    let profile = fetch_user_profile(&access_token).await;

    // Step 7: Save to config
    let mut cfg = config::load();
    cfg.user = Some(UserProfile {
        name: profile.name.clone(),
        email: profile.email.clone(),
        picture: profile.picture.clone(),
        access_token,
    });
    config::save(&cfg);

    println!("{} Logged in as {} ({})",
        "✓".green().bold(),
        profile.name.green(),
        profile.email.dimmed()
    );
}

fn wait_for_callback() -> String {
    // Spin up a minimal HTTP server on localhost:8080
    let server = tiny_http::Server::http("localhost:8080").unwrap();
    let request = server.recv().unwrap();

    // Parse ?code=... from the URL
    let url = request.url().to_string();
    let code = url
        .split("code=").nth(1).unwrap_or("")
        .split('&').next().unwrap_or("")
        .to_string();

    // Send a response to close the browser tab
    let response = tiny_http::Response::from_string(
        "<html><body><h2>Login successful! You can close this tab.</h2></body></html>"
    );
    let _ = request.respond(response);

    code
}

async fn exchange_code_for_token(code: &str, code_verifier: &str) -> String {
    let client = reqwest::Client::new();
    let params = [
        ("code", code),
        ("client_id", CLIENT_ID),
        ("client_secret", CLIENT_SECRET),
        ("redirect_uri", REDIRECT_URI),
        ("grant_type", "authorization_code"),
        ("code_verifier", code_verifier),
    ];

    let resp: serde_json::Value = client
        .post(TOKEN_URL)
        .form(&params)
        .send().await.unwrap()
        .json().await.unwrap();

    resp["access_token"].as_str().unwrap().to_string()
}

async fn fetch_user_profile(access_token: &str) -> GoogleProfile {
    let client = reqwest::Client::new();
    client
        .get(USERINFO_URL)
        .bearer_auth(access_token)
        .send().await.unwrap()
        .json::<GoogleProfile>().await.unwrap()
}

#[derive(serde::Deserialize)]
struct GoogleProfile {
    name: String,
    email: String,
    picture: String,
}

// PKCE helpers
fn generate_code_verifier() -> String {
    use rand::Rng;
    let bytes: Vec<u8> = (0..64).map(|_| rand::thread_rng().gen()).collect();
    base64::encode_config(bytes, base64::URL_SAFE_NO_PAD)
}

fn generate_code_challenge(verifier: &str) -> String {
    use sha2::{Sha256, Digest};
    let hash = Sha256::digest(verifier.as_bytes());
    base64::encode_config(hash, base64::URL_SAFE_NO_PAD)
}
```

---

### commands/me.rs — Show profile

```rust
use colored::Colorize;
use crate::config;

pub fn run() {
    let cfg = config::load();

    match cfg.user {
        Some(user) => {
            println!("{}", "─────────────────────────".dimmed());
            println!("  {}  {}", "Name:".bold(), user.name.green());
            println!("  {}  {}", "Email:".bold(), user.email);
            println!("  {}  {}", "Avatar:".bold(), user.picture.dimmed());
            println!("{}", "─────────────────────────".dimmed());
        }
        None => {
            println!("{} Not logged in. Run {} first.",
                "✗".red(),
                "claude-cli login".yellow()
            );
            std::process::exit(1);
        }
    }
}
```

---

### commands/logout.rs — Clear session

```rust
use colored::Colorize;
use crate::config;

pub fn run() {
    let cfg = config::load();

    if cfg.user.is_none() {
        println!("{} Already logged out.", "!".yellow());
        return;
    }

    config::clear();
    println!("{} Logged out successfully.", "✓".green());
}
```

---

## Google Cloud Setup

Before running the CLI you need a Google OAuth app.

### Steps

1. Go to [console.cloud.google.com](https://console.cloud.google.com)
2. Create a new project (or select existing)
3. Navigate to **APIs & Services → Library**
4. Search for and enable: **Google People API** (or **Google+ API**)
5. Navigate to **APIs & Services → Credentials**
6. Click **Create Credentials → OAuth 2.0 Client ID**
7. Application type: **Desktop app**
8. Name: `claude-cli`
9. Copy your **Client ID** and **Client Secret**
10. Paste them into `commands/login.rs`:

```rust
const CLIENT_ID: &str = "123456789-abc.apps.googleusercontent.com";
const CLIENT_SECRET: &str = "GOCSPX-your-secret";
```

### No redirect URI setup needed

Google automatically allows `http://localhost` redirect URIs for **Desktop app** type credentials.

---

## Testing

### Build and run

```bash
cargo build
./target/debug/claude-cli --help
```

```
A CLI tool with Google authentication

Usage: claude-cli <COMMAND>

Commands:
  login   Log in with your Google account
  me      Show your stored profile details
  logout  Log out and clear stored credentials
  help    Print this help message
```

### Test login

```bash
./target/debug/claude-cli login
# Opens browser → you log in → returns to terminal
# ✓ Logged in as Mani Kumar (mani@example.com)
```

### Test me

```bash
./target/debug/claude-cli me
# ─────────────────────────
#   Name:   Mani Kumar
#   Email:  mani@example.com
#   Avatar: https://lh3.googleusercontent.com/...
# ─────────────────────────
```

### Test logout

```bash
./target/debug/claude-cli logout
# ✓ Logged out successfully.

./target/debug/claude-cli me
# ✗ Not logged in. Run claude-cli login first.
```

### Verify config file

```bash
cat ~/.config/claude-cli/config.toml
```

```toml
[user]
name = "Mani Kumar"
email = "mani@example.com"
picture = "https://lh3.googleusercontent.com/..."
access_token = "ya29.a0AfH6SM..."
```

---

## What v0.2 adds

| Feature | Description |
|---------|-------------|
| `ask "question"` | Send a message to Claude API |
| Token refresh | Auto-refresh expired Google tokens |
| `--json` flag | Output `me` as JSON for scripting |
| Spinner | Loading animation while waiting for OAuth |
| Install script | `curl install.sh \| sh` one-liner |

---

## Key takeaway

```
login  →  browser OAuth  →  save profile to TOML
me     →  read TOML  →  display profile
logout →  delete TOML
```

The entire auth system is just reading and writing a TOML file.
The complexity is only in the OAuth redirect dance — once you have the token, everything else is simple.
