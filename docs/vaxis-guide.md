# Vaxis CLI — Team Setup & Usage Guide

> **Flow:** Install → Login → Install Skill → Use → Upgrade → Update Skill

---

## Prerequisites

- **Node.js 18 or later** — check with `node --version`
- **An AI agent** — Claude Code, Codex, or any Agent Skills-compatible host
- **A Vaxis account** — sign up at [app.vaxis.dev](https://app.vaxis.dev) before logging in

---

## Step 1 — Install Vaxis CLI

Run once on each machine:

```bash
npm install -g @unwita-insights/vaxis
```

Confirm the install:

```bash
vaxis --version
```

---

## Step 2 — Log in

```bash
vaxis login
```

This opens your browser and logs you in with your Google account. After completing the login,
confirm the CLI stored your session:

```bash
vaxis me --json
```

Expected output: `{ "name": "...", "email": "...", "token": "..." }`
If you see `{"error":"not_authenticated"}`, run `vaxis login` again.

---

## Step 3 — Install the Vaxis skill for your AI agent

The skill teaches your AI agent how to use Vaxis. Install it once per agent and scope.

```bash
npx skills add https://github.com/unwita-insights/vaxis-cli --skill vaxis
```

This command asks two questions:

1. **Which AI agent?** — choose the agent you use (Claude Code, Codex, etc.)
2. **Project or global?**
   - **Project** — installs the skill in the current project directory only (`.claude/skills/`
     or `.agents/skills/`). Use this when the project has its own AI config.
   - **Global** — installs the skill for your user account, active in all projects. Use this
     for a personal setup.

After install, **start a new session** in your AI agent for the skill to take effect.

> **Alternative:** If you prefer to install via the Vaxis CLI itself:
> ```bash
> vaxis install --skills
> ```
> This runs the same interactive prompts. Or install directly for Claude Code:
> ```bash
> vaxis install --skills --agent claude --project --yes
> ```

---

## Step 4 — Configure generation mode (optional)

This controls who writes the diagram. The default (your AI agent) is recommended for most
teams — no configuration needed to get started.

| Mode | Who generates | Best for |
|---|---|---|
| `mermaid` *(default)* | Your AI agent (Claude, Codex, GPT…) | Any model; supports deep drill-down; works offline |
| `prompt` | Vaxis server AI | Quick single-level overviews |

To switch to Vaxis server AI:

```bash
vaxis config set-mode prompt
```

To switch back:

```bash
vaxis config set-mode mermaid
```

Check what's currently set:

```bash
vaxis config show --json
```

---

## Step 5 — Use Vaxis in a project

Open your project in your AI agent and send this prompt to get started:

```
Use the installed Vaxis skill for this project.

First, explain briefly what Vaxis does and run `vaxis skills get core` to load its complete
instructions. Then inspect this codebase, identify its main components and relationships, and
create a clear root architecture diagram in Vaxis. Add useful drill-down diagrams for
important services or modules. Give me the Vaxis link when finished.
```

The agent will ask a brief setup question (working mode: interactive or hands-free), analyse
the project, create the diagrams, and give you a shareable link.

**Viewing diagrams:** Share the root diagram link with your team — no Vaxis account is needed
to view. One root link also unlocks every drill-down sub-diagram it links to.

---

## Step 6 — Keep diagrams in sync with code (recommended)

Add the following to your project's agent instruction file so diagrams stay up to date as the
code changes:

- Claude Code → `CLAUDE.md`
- Codex → `AGENTS.md`
- Other agents → equivalent project instruction file

```
## Vaxis architecture maintenance

Use Vaxis as the architecture source for this repository. Before creating,
reading, or updating diagrams, run `vaxis skills get core` and follow the
returned instructions.

Keep the codebase and Vaxis diagrams synchronised:

- Before implementing a change that affects architecture, inspect the existing
  Vaxis root diagram and relevant drill-down diagrams.
- If the architecture is designed in Vaxis first, implement the code to match
  the approved diagrams.
- If the code changes first, update the affected Vaxis diagrams in the same task.
- Add, update, or remove nodes, connections, and drill-down diagrams whenever
  services, modules, data stores, integrations, or major workflows change.
- Preserve accurate existing diagram content that is unrelated to the change.
- At the end of an architecture-impacting task, verify the diagram tree and
  provide the updated Vaxis link.
```

---

## Step 7 — Upgrade Vaxis CLI

Run this to get the latest version:

```bash
npm install -g @unwita-insights/vaxis@latest
```

Or use the built-in upgrade command:

```bash
vaxis upgrade
```

Check your current version:

```bash
vaxis --version
```

---

## Step 8 — Update the installed skill

After upgrading the CLI, update the skill to match:

```bash
npx skills update vaxis
```

This updates the discovery skill in place. After updating, **start a new session** in your
AI agent.

> **If you used `vaxis install --skills`** to install, update with the same command:
> ```bash
> vaxis install --skills --agent claude --project --yes
> ```
> Add `--force` if you have manually edited the skill file (backs it up first).

---

## Quick-reference cheat sheet

| Task | Command |
|---|---|
| Install CLI | `npm install -g @unwita-insights/vaxis` |
| Log in | `vaxis login` |
| Confirm login | `vaxis me --json` |
| Install skill | `npx skills add https://github.com/unwita-insights/vaxis-cli --skill vaxis` |
| Update skill | `npx skills update vaxis` |
| Upgrade CLI | `npm install -g @unwita-insights/vaxis@latest` |
| Quick upgrade | `vaxis upgrade` |
| Check version | `vaxis --version` |
| Check config | `vaxis config show --json` |
| Set generation mode | `vaxis config set-mode mermaid` |
| Log out | `vaxis logout` |
