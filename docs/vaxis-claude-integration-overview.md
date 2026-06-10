# Vaxis × Claude Integration — Strategy Overview

**Purpose:** Explain why two separate integration approaches are needed, what each covers, and how they complement each other  


---

## The Problem

Vaxis is a diagram and architecture tool. Our users increasingly work inside AI coding assistants — primarily Claude. If Claude cannot access Vaxis natively, users have to manually copy-paste IDs, commands, and outputs between Claude and the CLI. This is friction that kills adoption.

The goal is: **Claude should be able to use Vaxis as a first-class tool** — creating apps, listing diagrams, generating architecture, drilling into subsystems — just by being asked in natural language.

---

## Why One Approach Is Not Enough

There are two fundamentally different ways Claude can call an external tool, and they serve different audiences:

| | Approach 1: SKILL.md + Direct CLI | Approach 2: Remote MCP |
|---|---|---|
| **How Claude calls vaxis** | Runs `vaxis diagrams list` as a shell command | Calls structured API tools over HTTP |
| **Who this works for** | Claude Code users (developers in terminal) | Claude Desktop, Claude.ai, any Claude client |
| **User needs to install** | vaxis binary | Nothing — just a URL |
| **Implementation cost** | Zero — already done | Medium — new `/mcp` route |
| **When to ship** | Now (Phase 1) | Next sprint (Phase 2) |

Neither approach alone covers the full user base:
- **SKILL.md only** → non-developer users on Claude.ai or Claude Desktop are locked out
- **Remote MCP only** → developers using Claude Code lose the richest, fastest integration (direct CLI invocation with full shell context)

Doing both phases means every Claude user — from a developer in their terminal to a product manager on Claude.ai — can use Vaxis.

---

## Approach 1 Summary: SKILL.md + Direct CLI

**Status: Implemented.** The `skills/SKILL.md` file in this repo is the active Claude Code skill.

Claude Code reads the SKILL.md and then executes vaxis CLI commands directly:
```
Claude → reads skills/SKILL.md → runs `vaxis diagrams generate "$ID" --mermaid ...`
```

This works because Claude Code has native shell access. It is the simplest possible integration — no protocol, no server, no infrastructure change.

**Full analysis:** `docs/vaxis-approach-1-skillmd.md`

---

## Approach 2 Summary: Remote MCP on Cloudflare

**Status: Planned for next sprint.**

Add an `/mcp` route to the existing `apps/api` Cloudflare Worker. Claude connects via the MCP standard (Streamable HTTP transport). Users paste one URL in their Claude settings — done. No binary install required.

```
Claude Desktop/Claude.ai → HTTPS POST to api.vaxis.app/mcp → Cloudflare Worker → D1 database
```

This works because vaxis already has a running Cloudflare Worker backend. Adding the MCP route is extending what already exists — not new infrastructure.

**Full analysis:** `docs/vaxis-approach-2-remote-mcp.md`

---

## Industry Reference: Vercel agent-browser

Vercel built a CLI tool called `agent-browser` (browser automation for AI agents). It is a directly comparable project — a Rust CLI that AI agents use as a tool. Studying their approach gives us real-world validation.

### What they chose

Vercel chose **SKILL.md only**. No MCP.

They distribute via npm (`npm install -g agent-browser`). The tool ships with a SKILL.md stub that tells Claude Code to run `agent-browser skills get core` to fetch the full instructions. Claude then invokes the CLI as shell commands.

### Did they achieve their goal?

**Partially — for their specific use case.**

| | agent-browser outcome |
|---|---|
| Claude Code integration | ✅ Works well — developers use it directly in their coding sessions |
| Claude Desktop | ❌ No integration — requires shell access agent-browser doesn't have there |
| Claude.ai (web) | ❌ No integration |
| Docs always up-to-date | ✅ Dynamic `skills get core` fetch matches installed version |
| Zero-install for user | ❌ Must install the npm package first |

### Why their choice was correct for them — but not for us

`agent-browser` is a **developer tool** — it automates browsers inside a coding workflow. Its users are always developers, always in a terminal, always in Claude Code. The SKILL.md approach is a perfect fit.

Vaxis is different. It is a **SaaS product** with:
- A running backend API (Cloudflare Workers)
- Non-developer users (product managers, designers, architects) who use Claude.ai or Claude Desktop
- A team collaboration model where not everyone has the CLI installed

For Vercel, SKILL.md-only was sufficient. For vaxis, it is the right starting point but not the end state. Remote MCP is what gets us to full coverage.

---

## Decision Summary

| Phase | Approach | Status | Unlocks |
|---|---|---|---|
| **Phase 1** | SKILL.md + direct CLI | Done | Claude Code users (developers) |
| **Phase 2** | Remote MCP on Cloudflare | Next sprint | Claude Desktop + Claude.ai + Claude API (everyone) |

Phase 1 costs nothing and is already working. Phase 2 is one new TypeScript file and a dependency addition — it does not require new infrastructure because vaxis already runs on Cloudflare.

---

## Risk & Tradeoffs

| Risk | Mitigaton |
|---|---|
| MCP spec is relatively new (2024) | Anthropic owns the spec — it is stable and actively developed |
| `/mcp` endpoint is public | Auth via `Authorization: Bearer <token>` — same token the CLI uses |
| Agent-browser proves SKILL.md is enough short-term | It is — Phase 1 is live. Phase 2 expands reach, not replaces Phase 1 |
| Remote MCP requires ongoing maintenance | The route wraps existing API logic — if the API changes, MCP follows naturally |
