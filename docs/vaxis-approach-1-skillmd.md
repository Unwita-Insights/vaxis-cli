# Approach 1: SKILL.md + Direct CLI Integration

**Status: Implemented**  
**Target:** Claude Code users (developers using Claude in a terminal/IDE context)  
**Implementation cost:** Zero — already shipped in `skills/SKILL.md`

---

## Why

Claude Code has native shell access. It can run any CLI command on the user's machine — `git`, `npm`, `curl`, and crucially, `vaxis`.

This means we do not need any protocol layer, server, or API change. We only need to tell Claude **what vaxis does** and **how to call it**. That is exactly what a SKILL.md file provides.

The key insight: **for developers already using the CLI, the fastest Claude integration is zero infrastructure — just documentation that Claude reads.**

When a developer asks Claude to "generate a payment service architecture diagram", Claude reads the SKILL.md, understands what commands are available, and executes:
```bash
vaxis diagrams generate "$DIAGRAM_ID" --mermaid "$(cat payment.mmd)"
```

No protocol handshake. No API token exchange. Just a shell command — the same command the developer would type themselves.

---

## What

A `SKILL.md` file is a structured documentation file that Claude Code reads to understand a CLI tool. It is the standard integration pattern for Claude Code skills.

**Location in this repo:** `skills/SKILL.md`

The file contains:
- When Claude should use this tool (activation triggers)
- The full command reference with parameter formats
- Standard workflows (8 common patterns: create diagram, drill into subsystem, undo, share, etc.)
- Mermaid format rules and constraints
- JSON output schemas for parsing CLI responses
- Error handling scenarios

Claude reads this file once at the start of a session (or whenever it loads the skill), then calls CLI commands directly based on those instructions.

**How a user adds the skill to their project:**
```bash
# Option A — project-scoped (committed to repo, shared with the team)
npx skills add your-org/vaxis

# Option B — global (available in all Claude Code sessions)
# Add skills/SKILL.md path to Claude Code user settings
```

This creates `.claude/skills/vaxis/SKILL.md` in the project — a minimal stub pointing to the full runtime instructions.

---

## How

### The execution flow

```
1. Developer asks Claude: "Create an auth service diagram for this app"
2. Claude reads skills/SKILL.md (loaded at session start)
3. Claude knows: "I need to call vaxis diagrams create, then vaxis diagrams generate"
4. Claude runs:
     vaxis apps list --json
     vaxis diagrams create "$APP_ID" --name "Auth Service" --json
     vaxis diagrams generate "$DIAGRAM_ID" --mermaid "graph TD\n  ..."
5. Claude parses JSON output, reports back to developer
```

### Dynamic skill loading (the agent-browser pattern)

Rather than bundling a fixed SKILL.md, the recommended pattern is a minimal stub that fetches the live documentation at runtime:

**`skills/vaxis/SKILL.md` stub:**
```markdown
# Vaxis Skill

To get the full workflow instructions for the currently-installed version, run:
    vaxis skills get core

Then follow those instructions.
```

**Why this matters:** When vaxis adds new commands or changes workflows, the stub automatically points Claude to the updated docs without requiring users to update any config files.

### What Claude can do with this integration

With the SKILL.md loaded, Claude can handle requests like:
- *"List all my apps"* → `vaxis apps list --json`
- *"Create a new architecture diagram called 'Payment Service'"* → `vaxis diagrams create ...`
- *"Generate the Mermaid I gave you into this diagram"* → `vaxis diagrams generate ...`
- *"Drill into the auth node"* → `vaxis diagrams show --tree --json` + analyze `child_nodes`
- *"Undo the last change"* → `vaxis diagrams undo "$ID"`
- *"Show me the full tree of this architecture"* → `vaxis diagrams tree "$ID" --json`

---

## Vercel agent-browser: Case Study

[GitHub: vercel-labs/agent-browser](https://github.com/vercel-labs/agent-browser)

Vercel's `agent-browser` is a Rust CLI for browser automation used by AI agents. It is the closest public reference to what vaxis is doing. Their integration approach is **SKILL.md only** — no MCP, no protocol layer.

### What they built

- Distributed via `npm install -g agent-browser` (postinstall places the Rust binary)
- Ships with a SKILL.md stub at `.claude/skills/agent-browser/SKILL.md`
- Stub tells Claude to run `agent-browser skills get core` for the full docs
- Claude then invokes the CLI directly: `agent-browser open <url>`, `agent-browser snapshot -i`, `agent-browser click @e1`
- The "ref" pattern (`@e1`, `@e2`) is designed specifically for LLM workflows — Claude parses snapshot output and uses stable element refs rather than re-querying the DOM

### What they achieved

| Outcome | Result |
|---|---|
| Claude Code integration | ✅ Works well — developers use it seamlessly in coding sessions |
| Docs always match installed version | ✅ Dynamic `skills get core` fetch is a clean solution |
| Zero protocol complexity | ✅ No MCP overhead, no server to maintain |
| Works on Claude Desktop | ❌ Claude Desktop has no native shell access without MCP |
| Works on Claude.ai web app | ❌ No integration — web Claude cannot run local binaries |
| Zero-install path | ❌ User must `npm install -g agent-browser` first |

### Verdict on their approach

**It works for their use case.** `agent-browser` is a developer tool used inside coding workflows. Its users are always developers, always in a terminal, always in Claude Code. SKILL.md is the perfect fit — simple, fast, and the docs always stay current.

The approach **does not scale to a SaaS product** with non-developer users who access Claude through Claude Desktop or Claude.ai. For those use cases, a remote MCP server is required (see Approach 2).

### Key lessons from agent-browser

1. **SKILL.md is production-proven** — Vercel shipped this and it works
2. **Dynamic skill loading is the right pattern** — stub + `skills get core` keeps docs current
3. **Design for LLM consumption** — structured output formats (JSON, refs) make Claude's job easier
4. **This approach has a ceiling** — it covers developers, not the broader Claude user base

---

## Limitations

| Limitation | Impact |
|---|---|
| Requires vaxis binary installed on the user's machine | Developer must install vaxis before Claude can use it |
| Works in Claude Code only | Claude Desktop and Claude.ai cannot run shell commands |
| Shell environment must have the binary in PATH | May fail in some CI or containerized environments |
| No structured parameter validation | Claude infers parameters from docs — occasional hallucinations possible |

---

## Current Status

The `skills/SKILL.md` in this repo is the active implementation. It covers:
- All vaxis CLI commands
- 8 standard workflows
- Mermaid format rules
- JSON output schemas
- Error handling patterns

**This integration is live and working today.** No further changes are required to enable Approach 1.

---

## See Also

- `skills/SKILL.md` — the active Claude Code skill file
- `docs/vaxis-approach-2-remote-mcp.md` — Approach 2: Remote MCP for broader Claude coverage
- `docs/vaxis-claude-integration-overview.md` — Why both approaches are needed
