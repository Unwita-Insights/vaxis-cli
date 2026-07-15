# CLAUDE.md

Guidance for Claude Code when working in the `vaxis-cli` repository.

## What this is

`vaxis-cli` is the Rust command-line client for **Vaxis**, a hosted diagram/architecture
design SaaS (Cloudflare Workers API + D1, default host `https://beta.vaxis.dev`). The CLI
is a thin, auth-aware HTTP client — it renders nothing and runs no AI locally. Its purpose
is to be **driven by an AI assistant (Claude)**: Claude generates Mermaid, and Vaxis
persists it, auto-expands "drill" subsystems into a diagram tree, and returns a share link.

The behavioral contract that tells Claude *how* to use the CLI lives in `skills/SKILL.md`.
When you change a command's flags, output shape, or workflow, update `skills/SKILL.md` too —
it documents exact command invocations and JSON output schemas that the assistant relies on.

## Build / run / release

```bash
cargo build --release          # binary at target/release/vaxis
cargo run -- <args>            # e.g. cargo run -- apps list --json
cargo run -- --help
```

There is no test suite. `docs/test-commands.sh` and `docs/vaxis-cli-test-guide.md` contain
manual end-to-end checks against a running backend.

**Releasing** (see `docs/vaxis-release-guide.md`): bump the version in **both**
`Cargo.toml` and `npm/package.json` (kept in lockstep), then tag `vX.Y.Z` and push.
CI (`.github/workflows/npm-publish.yml`) cross-compiles 6 targets to a GitHub Release and
publishes `@unwita-insights/vaxis` to npm.

> Note: `src/cli.rs` hardcodes `#[command(version = "0.1.0")]`, which is stale and does not
> track the real version. Update it when touching versioning.

## Architecture

```
src/
├── main.rs            # #[tokio::main]; parses Cli, matches Commands, dispatches to a module
├── cli.rs             # ALL clap derive definitions (commands, subcommands, args)
├── config.rs          # TOML config load/save/clear + base_url() resolution
└── commands/
    ├── login.rs       # browser device-flow login (start → open browser → poll)
    ├── me.rs / logout.rs
    ├── config.rs      # config set-url / show
    ├── apps.rs        # applications: list/create/update/delete/share
    └── diagrams.rs    # diagrams: list/create/generate/show/tree/undo/rename/delete/patch/import/format
```

- **Flat command pattern.** `main.rs` matches the `Commands` enum and calls one `run()` per
  module. No central router, service layer, or DI.
- **`--json` is a global flag** (`cli.rs`), threaded as a `bool` into every command. Each
  command has two output modes: colored human text, or machine JSON. The assistant always
  uses `--json`. Any new command MUST honor `--json`.
- **No shared HTTP/error abstraction — this is intentional and repo-wide.** Every command
  inlines its own `reqwest::Client::new()`, builds its own URL from `config::base_url()`,
  attaches `Authorization: Bearer <token>`, and hand-matches status codes. When adding a
  command, copy the existing shape rather than inventing an abstraction, unless you are
  deliberately refactoring all of them.
- **Errors print and exit**, they don't propagate. The convention is
  `eprintln!("{} …", "✗".red()); std::process::exit(1);`. Status handling is uniform:
  `401` → "Session expired", `404` → "not found", `200`/`201` → success, other → "Unexpected
  status". In `--json` mode, machine-readable error objects are printed instead where relevant
  (e.g. `{"error":"not_authenticated"}`).

## Key conventions

- **Auth token** comes from `config::load().user.map(|u| u.token)`. `apps.rs` and `diagrams.rs`
  each define a local `auth_token()` and guard at the top of `run()`, emitting
  `{"error":"not_authenticated"}` (JSON) or a login hint (human) before exiting.
- **Config file**: `<OS config dir>/vaxis/config.toml` via the `dirs` crate. Holds `auth_url`
  and `user { name, email, token }`. `load()` never panics — malformed/missing → default.
- **Base URL precedence**: `VAXIS_AUTH_URL` env → `auth_url` in config → `https://beta.vaxis.dev`.
- **Interactive pickers**: `apps`/`diagrams` delete/update accept an optional id; when omitted,
  a `dialoguer::Select`/`Input`/`Confirm` prompt appears. A cancelled prompt exits cleanly (0).
  Guard interactive prompts so they don't fire in `--json`/scripting mode (see `diagrams delete`).
- **Human output uses the `colored` crate** (`.green()`, `.dimmed()`, `✓`/`✗`/`⚠` glyphs).
  Keep new human output consistent with the existing style.

## Domain concepts you must preserve

- **Two-mode generation** (`diagrams generate`): `--prompt` (server AI generates) vs
  `--mermaid` (Claude supplies Mermaid, server just stores + processes drills). They are
  `conflicts_with` each other. The `--mermaid` path is the primary product flow.
- **Drill auto-expansion**: after `generate`, the CLI iterates the server's `drills[]` and
  makes a follow-up `POST /api/diagrams/{id}/children` per node, materializing child diagrams.
  A single `generate` can create a parent + many children. Don't break this loop.
- **`%% vaxis:drill <nodeId>`** Mermaid comments mark nodes that become child diagrams.
- **`show` enrichment**: `show` makes a second `GET …/chat` call, finds the last `assistant`
  message, and surfaces it as a synthetic `current_mermaid` field; it also strips `scene_json`
  (Excalidraw noise) from JSON output. Preserve both behaviors — the assistant depends on
  `current_mermaid`.
- **`format`** makes no network call — it returns a hardcoded Mermaid reference spec.

## Backend API surface the CLI depends on

`POST /api/cli/start`, `GET /api/cli/poll?state=`,
`GET|POST /api/applications`, `GET|PUT|DELETE /api/applications/{id}`,
`POST /api/applications/{id}/share`,
`GET|POST /api/diagrams`, `GET|DELETE /api/diagrams/{id}`,
`POST /api/diagrams/{id}/generate|children|patch|import`,
`GET /api/diagrams/{id}/chat|tree`, `DELETE /api/diagrams/{id}/chat/last`,
`PATCH /api/diagrams/{id}/meta`.

## When you change things

- Adding/altering a command → update `src/cli.rs`, the module in `src/commands/`, **and**
  `skills/SKILL.md` (command reference + JSON output schema + relevant workflow).
- Changing JSON output shape → grep `skills/SKILL.md` and `docs/` for the affected fields.
- The npm `package.json` repository URL currently reads `github.com/unwita/vaxis-cli`; the
  real remote is `github.com/Unwita-Insights/vaxis-cli`.
