# Vaxis CLI — Guide & Cheatsheet

A simple guide for the team, plus the full command reference.

## What is the Vaxis CLI?

It's a small tool you run in your terminal that connects to Vaxis — the same
Vaxis you already use in the browser.

The big idea: **you don't draw anything yourself, and you don't type commands
yourself.** You talk to your AI assistant (Claude, Codex, or similar) in plain
English, the AI turns your words into a diagram and saves it to Vaxis, and you
get back a link to view and share it.

Think of it as: *you describe the system → the AI draws it → Vaxis keeps it.*

---

## Get set up (one time)

1. **Install it.** In your terminal, run:
   ```
   npm install -g @unwita-insights/vaxis
   ```
   This installs the `vaxis` binary.
2. **Sign in.** Run `vaxis login` — it opens your browser to sign in, just like
   the website.
3. That's it. Run `vaxis me` and you should see your name.

If any step doesn't work, ask your team's Vaxis admin — it's usually a quick fix.

> Tip: add `--json` to any command for machine-readable output — this is what
> AI assistants use, so you'll see it in examples below.

---

## The main way to use it: just ask your AI

Tell your AI assistant, once, that it can use Vaxis. The easiest way:

- **Claude Code** — add this line to your project's `CLAUDE.md`:
  *"When I ask for diagrams or system designs, use the Vaxis CLI (see skills/SKILL.md)."*
- **Codex or other tools** — add the same line to your `AGENTS.md` or the tool's
  instructions.
- **Any AI chat** — paste in the file `skills/SKILL.md`.

Don't worry about the details — that file tells the AI everything it needs.

Then just talk to it normally:

> "Design a food-delivery app: a customer mobile app, a restaurant dashboard, an
> orders service, a payments service, and a database. Show payments in detail."

> "Add a notifications service to the payment system we designed earlier."

> "Show me the diagram we made yesterday and explain how login works."

The AI does the rest and gives you a link.

---

## Full command reference

You mostly won't need these — the AI runs them for you — but here's everything
the CLI can do, if you want to try it by hand.

(Wherever you see `<...-id>`, that's an ID an earlier command showed you — copy
and paste it in.)

### Setup

| Command | What it does |
|---|---|
| `vaxis login` | Browser login. Saves token to `<config dir>/vaxis/config.toml` |
| `vaxis me` | Show who you're logged in as |
| `vaxis logout` | Clear stored credentials |
| `vaxis config set-url <url>` | Point at a different server (default `https://app.vaxis.dev`) |
| `vaxis config show` | Show current server + user |

### Apps (projects)

| Command | What it does |
|---|---|
| `vaxis apps list` | List your projects |
| `vaxis apps create <name> [-d <desc>]` | Create a project |
| `vaxis apps update [id] [--name] [--description]` | Edit one (interactive if no id) |
| `vaxis apps delete [id] [--force]` | Delete one (interactive if no id) |
| `vaxis apps share <id> [--revoke]` | **Legacy only** — app-wide sharing is retired. Reads/revokes an old link. Use `diagrams share` instead |

### Diagrams

| Command | What it does |
|---|---|
| `vaxis diagrams list <appId>` | List diagrams in a project |
| `vaxis diagrams create <appId> <name>` | Create an empty diagram |
| `vaxis diagrams generate <id> --mermaid "..."` | **Main flow** — save Mermaid your AI wrote. No server AI, no quota |
| `vaxis diagrams generate <id> --prompt "..."` | Let the *server's* AI write it. `--intent auto\|edit\|replace\|drill\|detail\|simplify\|ask`, `--session <id>` |
| `vaxis diagrams ask <id> --prompt "..."` | Ask about a diagram — prose answer, no edit |
| `vaxis diagrams show <id>` | Metadata + current Mermaid (`current_mermaid`) + children |
| `vaxis diagrams tree <id>` | Full parent→child hierarchy |
| `vaxis diagrams share <id> [--rotate] [--revoke]` | Get/create the public link. Covers this diagram + everything it drills into |
| `vaxis diagrams import <id> --mermaid "..."` | Save raw Mermaid, bypass AI |
| `vaxis diagrams undo <id>` | Drop the last AI turn before retrying |
| `vaxis diagrams rename <id> <name>` | Rename |
| `vaxis diagrams delete [id] [--force]` | Delete it and its children |
| `vaxis diagrams format` | Mermaid reference — types, syntax, limits. No login needed |
| `vaxis diagrams sessions list\|create\|rename <id>` | Manage server-AI chat sessions |
| `vaxis --help` | The full list of everything the tool can do |

**Drill-down:** mark a node with `%% vaxis:drill <nodeId>` on the next line and `generate`
auto-creates a child diagram for it.

**Two gotchas:**
- `diagrams share` is safe to re-run — it returns the existing link. `--rotate` mints a new
  one and **breaks the old link**.
- `generate --prompt` doesn't always edit. The server answers instead when the prompt reads
  as a question (default intent is `auto`), replying `unchanged: true` + `answer`. Use
  `--intent edit` to force a change, or `--mermaid` to bypass the server AI entirely.

Sharing tip: share the **main** diagram of a project. Its link also opens all the
smaller diagrams inside it, so one link is usually all you need.

---

## Connecting it to an AI assistant

Vaxis is designed to be *driven by* your assistant: it writes the Mermaid, Vaxis stores it,
builds the diagram tree, and returns a share link. The behavior contract lives in
[`skills/SKILL.md`](../skills/SKILL.md) — it tells the assistant when to use Vaxis and how to
sequence commands (check before creating, read before overwriting, undo before retrying, end
with a share link).

The npm package ships **only the binary**, not `SKILL.md` — grab that from the repo.

**Claude Code** — install it as a skill:
```bash
mkdir -p ~/.claude/skills/vaxis
curl -o ~/.claude/skills/vaxis/SKILL.md \
  https://raw.githubusercontent.com/Unwita-Insights/vaxis-cli/main/skills/SKILL.md
```
Use `.claude/skills/vaxis/` inside a project instead to scope it to that repo.
Claude Code requires YAML frontmatter (`name`, `description`) at the top of `SKILL.md` for
auto-loading; without it, just point Claude at the file directly.

**Codex / agents that read `AGENTS.md`** — add a pointer:
```markdown
## Diagrams
Use the `vaxis` CLI for any architecture/diagram work.
Read ./skills/SKILL.md for the command contract. Always pass --json.
```

**Cursor, opencode, Copilot, any other agent** — same idea: put `SKILL.md` where the tool
reads project instructions (`.cursor/rules/`, `AGENTS.md`, `CONVENTIONS.md`), or paste it in.
It's plain Markdown with no tool-specific syntax.

**Anything else / no skill support** — tell the assistant:
> Use the `vaxis` CLI for diagrams. Run `vaxis --help` and `vaxis diagrams format` first,
> and pass `--json` to every command.

Whichever route: run `vaxis login` yourself once first — the assistant can't do the browser
login for you.

---

## Keeping it up to date

Every so often, run this to get the newest version:

```
npm install -g @unwita-insights/vaxis@latest
```

---

## If something looks wrong

- **It asks you to log in again** — just run `vaxis login`.
- **It can't connect, or your projects are missing** — you might be pointed at
  the wrong server. Ask your admin which one your team uses.
- **The command isn't found** — try the install step again.

That's everything. Have fun — describe a system to your AI and watch it appear in
Vaxis.
