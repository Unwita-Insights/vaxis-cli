# CLAUDE.md

Guidance for Claude Code when working in the `vaxis-cli` repository.

## What this is

`vaxis-cli` is the Rust command-line client for **Vaxis**, a hosted diagram/architecture
design SaaS (Cloudflare Workers API + D1, default host `https://app.vaxis.dev`). The CLI
is an auth-aware HTTP client that also embeds version-matched agent instructions. It renders
nothing and runs no AI locally. Its purpose is to be **driven by an AI assistant**: the
assistant generates Mermaid, and Vaxis
persists it, auto-expands "drill" subsystems into a diagram tree, and returns a share link.

The authoritative behavioral contract lives in `skill-data/core/SKILL.md` and is embedded in
the binary for `vaxis skills get core`. The small `skills/vaxis/SKILL.md` discovery skill is
installed into supported agent hosts by `vaxis install --skills`. When you change command
flags, output shapes, or workflows, update the core skill too — it documents the exact
invocations and JSON schemas that assistants rely on.

### Why the installed skill loads `vaxis skills get core`

Keep `skills/vaxis/SKILL.md` as a small discovery skill; do not copy the full core contract
into it.

- The core contract must match the installed CLI's commands and JSON schemas. It is compiled
  into the official binary with `include_str!`, so `vaxis skills get core` reads local,
  version-locked package content and makes no network request.
- Publishing the full contract through skills.sh would create a second independently updated
  copy. A marketplace skill could then describe commands that an older installed binary does
  not support, or remain stale after a CLI release.
- The discovery file keeps agent startup context small. The larger contract is loaded only
  when Vaxis is actually used.
- This is an established pattern for versioned CLI tools. For example,
  [`agent-browser`](https://www.skills.sh/vercel-labs/agent-browser/agent-browser) publishes a
  discovery stub that loads its version-matched core with `agent-browser skills get core`.
  Static skills such as [`gh-skill`](https://www.skills.sh/cli/cli/gh-skill), whose contract is
  the skill file itself, publish their complete instructions directly.

Security scanners may conservatively flag the discovery command as indirect prompt loading.
The trust boundary is the locally installed Vaxis binary, not remote task-time content. Users
should install Vaxis only from the official npm/GitHub release. Any change that makes
`skills get core` fetch instructions from the network requires a fresh security review.

The public listing is
[`unwita-insights/vaxis-cli/vaxis`](https://www.skills.sh/unwita-insights/vaxis-cli/vaxis).
Update `skills/vaxis/SKILL.md` for discovery/triggering changes; update
`skill-data/core/SKILL.md` and publish a new CLI version for behavioral changes.

## Build / run / release

```bash
cargo build --release          # binary at target/release/vaxis
cargo run -- <args>            # e.g. cargo run -- apps list --json
cargo run -- --help
cargo test
```

The Rust test suite covers CLI contracts, skill packaging and installation behavior, Mermaid
linting, and parity evaluation. `docs/test-commands.sh` and `docs/vaxis-cli-test-guide.md`
also contain manual end-to-end checks against a running backend.

**Releasing** (see `docs/vaxis-release-guide.md`): bump the version in **both**
`Cargo.toml` and `npm/package.json` (kept in lockstep), then tag `vX.Y.Z` and push.
CI (`.github/workflows/npm-publish.yml`) cross-compiles 6 targets to a GitHub Release and
publishes `@unwita-insights/vaxis` to npm.

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
    ├── apps.rs        # applications: list/create/update/delete
    ├── diagrams.rs    # diagrams: list/create/generate/ask/sessions/share/show/tree/undo/rename/delete/import/format
    └── skills.rs      # bundled skill inspection + discovery-skill installation

skills/vaxis/SKILL.md          # small discovery skill installed into agent hosts
skill-data/core/SKILL.md       # authoritative instructions embedded in the binary
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
- **Base URL precedence**: `VAXIS_AUTH_URL` env → `auth_url` in config → `https://app.vaxis.dev`.
- **Interactive pickers**: `apps`/`diagrams` delete/update accept an optional id; when omitted,
  a `dialoguer::Select`/`Input`/`Confirm` prompt appears. A cancelled prompt exits cleanly (0).
  Guard interactive prompts so they don't fire in `--json`/scripting mode (see `diagrams delete`).
- **Human output uses the `colored` crate** (`.green()`, `.dimmed()`, `✓`/`✗`/`⚠` glyphs).
  Keep new human output consistent with the existing style.

## Domain concepts you must preserve

- **Two-mode generation** (`diagrams generate`): `--prompt` (server AI generates) vs
  `--mermaid` (Claude supplies Mermaid, server just stores + processes drills). They are
  `conflicts_with` each other. The `--mermaid` path is the primary product flow.
- **`generate` is not always an edit.** The server routes a `--prompt` turn to Ask when
  `intent:"ask"` OR when intent is `auto` (the DEFAULT) and the prompt parses as a question
  — returning `{unchanged:true, answer, drills:[], mermaid:<current content echoed back>}`.
  `notice` and `mode_mismatch` are the other no-edit turns. `generate` MUST check
  `unchanged`/`answer` before reporting success: printing "Generated" over the echoed-back
  Mermaid claims an edit that never happened and throws the answer away. The `--mermaid`
  path never routes to Ask.
- **Sharing is per-diagram, not per-app.** One diagram link also unlocks the sub-diagrams
  it drills into, so share the ROOT diagram. App-wide sharing is retired because a single
  app link exposes every diagram in the app — the CLI exposes no app-share command at all
  and must not call `/api/applications/{id}/share`.
- **Drill auto-expansion**: after `generate`, the CLI iterates the server's `drills[]` and
  makes a follow-up `POST /api/diagrams/{id}/children` per node, materializing child diagrams.
  A single `generate` can create a parent + many children. Don't break this loop.
- **`%% vaxis:drill <nodeId>`** Mermaid comments mark nodes that become child diagrams.
- **`show` enrichment**: `show` makes a second `GET …/chat` call, finds the last `assistant`
  message, and surfaces it as a synthetic `current_mermaid` field; it also strips `scene_json`
  (Excalidraw noise) from JSON output. Preserve both behaviors — the assistant depends on
  `current_mermaid`.
- **`format`** makes no network call — it returns the embedded Mermaid reference spec.
- **Skill distribution** also makes no network call. `vaxis skills get core` prints the
  embedded authoritative skill exactly; `vaxis install --skills` installs the small discovery
  skill using canonical host path mappings, checksum-managed upgrades, and backup-on-force.

## Backend API surface the CLI depends on (the contract)

**🔴 STRONG RULE — this CLI is a hard-coded HTTP client of the `vaxis` backend (`apps/api` in
the separate repo [`vaxis`](https://github.com/Unwita-Insights/vaxis)). Every URL and JSON
field name is a literal string in the Rust source; there is NO shared schema.** This creates a
two-way contract that MUST stay in sync:

- **If the backend changes a consumed endpoint** (rename/remove a route, change a method/path,
  rename a request/response field) → the CLI breaks silently. The mirror of this rule lives in
  `vaxis`'s `CLAUDE.md` (API routes section) and lists the same endpoints.
- **If the CLI starts consuming a NEW backend endpoint, or stops using one** → update the list
  below AND the corresponding list in `vaxis`'s `CLAUDE.md` in the same change, so both sides
  agree on exactly which routes are coupled. Do not add a new backend call without recording it
  in both places.

Endpoints the CLI is coupled to (backend's CURRENT paths):

- **Auth:** `POST /api/cli/start`, `GET /api/cli/poll?state=`, `POST /api/cli/complete`,
  `Bearer <cli_token>` auth.
- **Apps:** `GET|POST /api/applications`, `GET|PUT|DELETE /api/applications/{id}`,
  `GET /api/applications/{id}/diagrams` (list diagrams).
- **Diagrams:** `POST /api/diagrams`, `GET|DELETE /api/diagrams/{id}`,
  `PATCH /api/diagrams/{id}` (rename),
  `GET|POST|DELETE /api/diagrams/{id}/share` (per-diagram sharing — the CURRENT share
  model; returns `{token, edit_token}` and the CLI builds `/view/{token}` +
  `/collab/{edit_token}`. `POST` is create-OR-**ROTATE**: it mints a new token pair every
  call, so `diagrams share` reads via `GET` first and only `POST`s when unshared),
  `GET /api/diagrams/rules` (authenticated canonical diagram-authoring contract used by
  `diagrams rules-check` to detect cross-repository rule drift),
  `POST /api/diagrams/{id}/generate`
  (request may send `prompt` + `intent` + `chat_session_id`, or `mermaid` for the direct
  path; direct Mermaid requests may also include optional `direction_context` with
  `policy`, `explicit`, `is_fresh_generation`, and `viewport`; `intent:"ask"` powers
  `diagrams ask` and returns an `answer` field),
  `POST /api/diagrams/{id}/children`, `POST /api/diagrams/{id}/import`,
  `GET /api/diagrams/{id}/tree`, `GET /api/diagrams/{id}/chat`,
  `GET|POST /api/diagrams/{id}/chat/sessions` (sessions list/create),
  `PATCH /api/diagrams/{id}/chat/sessions/{sid}` (session rename),
  `DELETE /api/diagrams/{id}/chat/messages/last` (undo).

**🔴 STRONG RULE — `skill-data/core/SKILL.md`'s diagram style-rules section (shape mapping, subgraph
coloring, fan-out cap, auto-drill threshold) is a condensed mirror of the system prompt the
`vaxis` backend gives its own server-side AI**, so that any driving assistant (via `diagrams
generate --mermaid`) produces visually consistent diagrams with the server-AI path (`generate
--prompt`). The mirrored source, in the `vaxis` repo:
- `S_FLOWCHART_SHAPES`, `S_COLOR`, `S_OUTPUT_FLOWCHART` (fan-out cap, layout cleanliness,
  subgraph grouping syntax), `S_DRILL` in `apps/api/src/prompts.ts`.
- `STORAGE_KEYWORD_TOKENS` / `STORAGE_KEYWORDS_PLAIN` in `packages/scene-serializer/src/shapeRules.ts`.

This is a **prose mirror, not shared code** — the CLI is Rust and cannot import a TS module, so
some drift over time is expected, not a bug. If you're touching `vaxis`'s prompt/shape rules,
flag it and update `skill-data/core/SKILL.md` here in the same change (or open a `vaxis-cli` issue). The
mirror note on the `vaxis` side lives next to its own STRONG RULE for the endpoint contract.

## When you change things

- Adding/altering a command → update `src/cli.rs`, the module in `src/commands/`, **and**
  `skill-data/core/SKILL.md` (agent workflow and JSON contract) **and**
  `docs/vaxis-cli-commands.md` (complete user-facing command index and short descriptions).
- Keep `vaxis --help` concise and useful for everyday users. New top-level commands need a
  clear one-line description in `src/cli.rs`; put detailed flags and advanced options in the
  command's nested `vaxis <command> --help` output instead of expanding the root help.
- Changing skill discovery or installation behavior → update `src/commands/skills.rs`,
  `skills/vaxis/SKILL.md`, and `docs/vaxis-skill-distribution-plan.md`.
- Adding/removing a consumed backend endpoint → update the contract list above **and** the
  mirror list in `vaxis`'s `CLAUDE.md` (see the STRONG RULE).
- Changing JSON output shape → grep `skill-data/core/SKILL.md` and `docs/` for the affected fields.
- The npm `package.json` repository URL currently reads `github.com/unwita/vaxis-cli`; the
  real remote is `github.com/Unwita-Insights/vaxis-cli`.

## Workflow preference (owner: Kaviya)

- **Never commit contract/CLAUDE.md/cross-repo changes straight to `main` or an unrelated
  branch.** Make them on a dedicated branch and open a PR. This applies in BOTH repos —
  `vaxis-cli` and `vaxis`.
- **Ask before pushing or opening a PR**, and confirm the base branch first (changes here start
  from `main` unless told otherwise).
