# Vaxis CLI

**Diagram and architecture design, driven by your AI assistant.**

`vaxis` is a command-line client for [Vaxis](https://beta.vaxis.dev) — a hosted tool for
designing systems as Mermaid diagrams. It's built to be used *by an AI assistant* (like
Claude): the assistant generates the diagram, and Vaxis stores it, breaks large systems into
a navigable tree of sub-diagrams, and gives you a shareable link. You can also use it
directly from your terminal.

- 🤖 **AI-first** — every command speaks `--json` so an assistant can read output and decide what to do next
- 🧩 **Drill-down hierarchy** — mark a node with `%% vaxis:drill <id>` and Vaxis auto-creates a child diagram for it
- 🔗 **Shareable** — publish any project to a public view link
- 🦀 **Single static binary** — written in Rust, distributed via npm or `cargo`

---

## Installation

### npm (recommended)

```bash
npm install -g @unwita-insights/vaxis
```

The install downloads a prebuilt native binary for your platform (macOS arm64/x64,
Linux x64/arm64/musl-x64, Windows x64). If no binary is available for your platform, the
install still succeeds — build from source instead.

### From source

```bash
git clone https://github.com/Unwita-Insights/vaxis-cli.git
cd vaxis-cli
cargo build --release
# binary is at target/release/vaxis
```

---

## Getting started

```bash
# 1. Log in (opens your browser)
vaxis login

# 2. Create a project ("application")
vaxis apps create "Payment System" --description "Stripe-backed checkout"

# 3. Create a diagram in it
vaxis diagrams create <appId> "Root Architecture"

# 4. Save a Mermaid diagram to it
vaxis diagrams generate <diagramId> --mermaid "graph TD
    ui[Web App] -->|HTTPS| api[API Gateway]
    api -->|validates| auth[Auth Service]
    api -->|charges| pay[Payment Service]
    %% vaxis:drill pay
    pay --> db[(PostgreSQL)]"

# 5. Get a shareable link
vaxis apps share <appId>
```

Because `pay` is annotated with `%% vaxis:drill pay`, Vaxis automatically creates a child
diagram for the Payment Service that you can drill into and expand later.

---

## Commands

Every command accepts a global `--json` flag for machine-readable output.

### Authentication

| Command | Description |
|---|---|
| `vaxis login` | Log in via your browser |
| `vaxis me` | Show the stored profile |
| `vaxis logout` | Clear stored credentials |

### Configuration

| Command | Description |
|---|---|
| `vaxis config set-url <url>` | Point the CLI at a different Vaxis server |
| `vaxis config show` | Show the current server URL |

### Applications (projects)

| Command | Description |
|---|---|
| `vaxis apps list` | List your applications |
| `vaxis apps create <name> [--description <text>]` | Create an application |
| `vaxis apps update [id] [--name <n>] [--description <d>]` | Update name/description (interactive if `id` omitted) |
| `vaxis apps delete [id] [--force]` | Delete an application (interactive if `id` omitted) |
| `vaxis apps share <id>` | Get or create the public share link |

### Diagrams

| Command | Description |
|---|---|
| `vaxis diagrams list <appId>` | List diagrams in an application |
| `vaxis diagrams create <appId> <name>` | Create a diagram |
| `vaxis diagrams generate <id> --mermaid <str>` | Save Mermaid directly (processes drill annotations) |
| `vaxis diagrams generate <id> --prompt <str>` | Let the server AI generate the diagram |
| `vaxis diagrams show <id>` | Show a diagram's current Mermaid and child nodes |
| `vaxis diagrams tree <id>` | Print the full diagram tree for the application |
| `vaxis diagrams undo <id>` | Remove the last generation turn (safe undo before retry) |
| `vaxis diagrams rename <id> <name>` | Rename a diagram |
| `vaxis diagrams delete [id] [--app-id <appId>] [--force]` | Delete a diagram and its children |
| `vaxis diagrams patch <id> --diff <json>` | Apply a targeted node/edge diff without rewriting the whole diagram |
| `vaxis diagrams import <id> --mermaid <str>` | Save raw Mermaid directly, no AI |
| `vaxis diagrams format` | Print the Mermaid format reference (types, drill syntax, limits) |

> `--mermaid` and `--prompt` are mutually exclusive on `generate`. Use `--mermaid` when *you*
> (or your assistant) produce the diagram; use `--prompt` only to exercise the server's own AI.

---

## Configuration & data

- **Config file:** `~/.config/vaxis/config.toml` (Linux/macOS) or `%APPDATA%\vaxis\config.toml`
  (Windows). Stores the server URL and your login token.
- **Server URL precedence:** `VAXIS_AUTH_URL` environment variable →
  `auth_url` in the config file → default `https://beta.vaxis.dev`.

```bash
# Point at a local backend during development
vaxis config set-url http://localhost:3000
# or, one-off:
VAXIS_AUTH_URL=http://localhost:3000 vaxis apps list --json
```

---

## Using Vaxis with Claude

Vaxis ships a Claude skill at [`skills/SKILL.md`](skills/SKILL.md). It teaches an assistant
when to reach for Vaxis and how to sequence commands — check before creating, read before
overwriting, undo before retrying, and end each session with a share link. Point your
assistant at that file (or install it as a skill) to let it design systems on your behalf and
hand you a link at the end.

### Mermaid basics

Vaxis supports flowcharts (`graph TD` / `graph LR`), ER diagrams (`erDiagram`), sequence
diagrams (`sequenceDiagram`), state machines (`stateDiagram-v2`), class diagrams
(`classDiagram`), and user journeys (`journey`). Mark any node for drill-down with a comment
on the following line:

```
graph TD
    api[API Gateway]
    pay[Payment Service]
    %% vaxis:drill pay
```

Run `vaxis diagrams format --json` for the full reference (limits: 50 nodes / 60 edges per
diagram).

---

## Development

```bash
cargo build --release
cargo run -- apps list --json
```

The project is a single Rust binary. Command definitions live in `src/cli.rs`; each command
is implemented in `src/commands/`. See [`CLAUDE.md`](CLAUDE.md) for architecture notes and
conventions. Manual test recipes are in [`docs/`](docs/).

### Releasing

Bump the version in **both** `Cargo.toml` and `npm/package.json`, then tag and push:

```bash
git tag v0.1.9
git push origin v0.1.9
```

CI cross-compiles all targets, publishes a GitHub Release, and pushes the package to npm.

---

## License

MIT
