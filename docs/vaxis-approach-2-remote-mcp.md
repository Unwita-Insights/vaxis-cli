# Approach 2: Remote MCP on Cloudflare Workers

**Status: Planned — next sprint**  
**Target:** Claude Desktop, Claude.ai, Claude API, and all other Claude clients  
**Implementation cost:** Medium — one new TypeScript file, one dependency, one route registration

---

## Why

Approach 1 (SKILL.md + direct CLI) covers Claude Code users — developers in a terminal. But there is a large segment of Claude users who do not use Claude Code:

- **Claude Desktop** — product managers, architects, designers who use Claude as a desktop app
- **Claude.ai** — anyone using Claude through the web browser
- **Claude API** — developers building applications on top of Claude

None of these can run shell commands. None of them can call `vaxis diagrams list` directly. For these users, vaxis is completely invisible without a second integration layer.

**The solution is MCP — Model Context Protocol.**

MCP is Anthropic's open standard for connecting Claude to external tools via HTTP. A remote MCP server is a public HTTPS endpoint that Claude calls with structured JSON requests. No binary install. No shell access required. Just a URL.

Crucially: **vaxis already runs on Cloudflare Workers.** The API backend (`apps/api`) is live at `api.vaxis.app`. Adding MCP is not new infrastructure — it is one new route on the existing Worker.

---

## What

### Model Context Protocol (MCP)

MCP (created by Anthropic 2024, now an open standard under the Linux Foundation) defines how AI models connect to external tools. Think of it as a USB-C port for AI integrations — one standard that works with any compatible client.

**Core concepts:**
- **Tools** — functions Claude can call (e.g., `list_diagrams`, `generate_diagram`)
- **Streamable HTTP transport** — the current standard; Claude sends HTTP POST/GET to your server
- **Tool schemas** — JSON Schema definitions for each tool's parameters; Claude uses these to form correct calls

### What gets built

A new route `/mcp` on the existing `apps/api` Cloudflare Worker.

**Architecture:**
```
Claude Desktop / Claude.ai / Claude API
        │
        │  HTTPS POST https://api.vaxis.app/mcp
        │  Authorization: Bearer <token>
        ▼
┌─────────────────────────────────────┐
│   Cloudflare Worker (apps/api)      │
│   ┌─────────────────────────────┐   │
│   │  /mcp route (new)           │   │
│   │  MCP tool handlers          │   │
│   └──────────┬──────────────────┘   │
│              │ reuses existing       │
│              ▼ service logic         │
│   ┌─────────────────────────────┐   │
│   │  D1 database                │   │
│   │  apps / diagrams / etc.     │   │
│   └─────────────────────────────┘   │
└─────────────────────────────────────┘
```

The MCP route does not duplicate business logic — it calls the same service functions that the REST API already uses.

### Tools to expose

| MCP Tool | Maps to | Description |
|---|---|---|
| `list_apps` | `GET /api/apps` | List all apps for the authenticated user |
| `list_diagrams` | `GET /api/apps/:id/diagrams` | List diagrams in an app |
| `show_diagram` | `GET /api/diagrams/:id` + `/tree` | Get diagram details and child node tree |
| `generate_diagram` | `POST /api/diagrams/:id/generate` | Generate Mermaid diagram + create drill children |
| `create_diagram` | `POST /api/apps/:id/diagrams` | Create a new blank diagram |
| `get_share_link` | `GET /api/diagrams/:id/share` | Get a shareable link for a diagram |

---

## How

### Step 1: Add the MCP SDK dependency

```bash
cd vaxis-web/vaxis/apps/api
npm install @modelcontextprotocol/sdk
```

### Step 2: Create `src/routes/mcp.ts`

```typescript
import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import { StreamableHTTPServerTransport } from '@modelcontextprotocol/sdk/server/streamableHttp.js';
import { z } from 'zod';

export function createMcpServer(env: Env) {
  const server = new McpServer({
    name: 'vaxis',
    version: '1.0.0',
  });

  server.tool(
    'list_apps',
    'List all apps for the authenticated user',
    {},
    async () => {
      const apps = await listApps(env.DB);  // reuse existing service logic
      return { content: [{ type: 'text', text: JSON.stringify(apps) }] };
    }
  );

  server.tool(
    'list_diagrams',
    'List all diagrams in an app',
    { app_id: z.string().describe('The app ID') },
    async ({ app_id }) => {
      const diagrams = await listDiagrams(env.DB, app_id);
      return { content: [{ type: 'text', text: JSON.stringify(diagrams) }] };
    }
  );

  server.tool(
    'generate_diagram',
    'Generate a Mermaid diagram and create drill-down children for annotated nodes',
    {
      diagram_id: z.string().describe('The diagram ID to generate into'),
      mermaid: z.string().describe('The Mermaid diagram source code'),
    },
    async ({ diagram_id, mermaid }) => {
      const result = await generateDiagram(env.DB, diagram_id, mermaid);
      return { content: [{ type: 'text', text: JSON.stringify(result) }] };
    }
  );

  // ... additional tools

  return server;
}

export async function handleMcpRequest(request: Request, env: Env): Promise<Response> {
  const token = request.headers.get('Authorization')?.replace('Bearer ', '');
  if (!token || !(await validateToken(env.DB, token))) {
    return new Response('Unauthorized', { status: 401 });
  }

  const server = createMcpServer(env);
  const transport = new StreamableHTTPServerTransport({ sessionIdGenerator: undefined });
  await server.connect(transport);
  return transport.handleRequest(request);
}
```

### Step 3: Register the route in `src/index.ts`

```typescript
import { handleMcpRequest } from './routes/mcp.js';

// Add alongside existing routes:
app.all('/mcp', (c) => handleMcpRequest(c.req.raw, c.env));
app.all('/mcp/*', (c) => handleMcpRequest(c.req.raw, c.env));
```

### Step 4: Deploy

```bash
wrangler deploy
```

No new Cloudflare resources. No new environment variables. The same Worker, now with an additional route.

---

## How Users Connect

### Claude Desktop
Go to **Settings → Integrations → + Add MCP Server**  
Enter: `https://api.vaxis.app/mcp`  
Authentication: Claude prompts for the Bearer token (same token from `vaxis auth login`)

### Claude.ai (web)
Go to **Settings → Integrations → + Add**  
Enter: `https://api.vaxis.app/mcp`

### Claude Code (project-wide)
Add to `.mcp.json` in the project root (committed to the repo — shared with the whole team):
```json
{
  "mcpServers": {
    "vaxis": {
      "type": "http",
      "url": "https://api.vaxis.app/mcp",
      "headers": {
        "Authorization": "Bearer ${VAXIS_TOKEN}"
      }
    }
  }
}
```

### Claude API (programmatic)
```bash
curl https://api.anthropic.com/v1/messages \
  -H "anthropic-beta: mcp-client-2025-11-20" \
  -d '{
    "model": "claude-opus-4-7",
    "mcp_servers": [{"type": "url", "url": "https://api.vaxis.app/mcp", "name": "vaxis"}],
    "tools": [{"type": "mcp_toolset", "mcp_server_name": "vaxis"}],
    "messages": [{"role": "user", "content": "List my diagrams for app abc123"}]
  }'
```

---

## Why Not Local stdio MCP Instead?

An alternative is adding an MCP server mode to the Rust binary itself (`vaxis mcp-server`), which speaks the MCP stdio protocol. Claude Desktop spawns it as a child process.

This was considered and deprioritized for the following reasons:

| | Local stdio MCP | Remote MCP (our choice) |
|---|---|---|
| User must install binary | Yes | **No** |
| Works on Claude.ai | No | **Yes** |
| Extra infrastructure | No | **No** — we already run on Cloudflare |
| Auth complexity | Token in env var (manual) | Bearer header (same as CLI) |
| Fits vaxis as a SaaS | Poorly | **Well** |
| Implementation location | Rust binary | TypeScript (existing stack) |

Local stdio MCP is the right choice for developer tools that need local file access (like `agent-browser`). Vaxis is an API-backed SaaS — the natural home for its MCP server is the existing API, not a local binary.

Local stdio MCP may be added later as a Phase 3 enhancement for users who want offline or private-network access.

---

## Why Cloudflare Is the Right Host

- **Already our stack** — `apps/api` is already deployed as a Cloudflare Worker; no new infrastructure to provision
- **Edge performance** — Cloudflare Workers run at the network edge globally; MCP calls from any Claude client get low latency
- **Zero cold starts** — Workers are always warm; no Lambda-style cold start delay on Claude tool calls
- **Same auth** — the existing Bearer token validation logic in the API reuses directly
- **Free tier covers it** — MCP requests are small JSON payloads; the Cloudflare free tier (100k requests/day) covers all but the largest teams

---

## Comparison: agent-browser vs. vaxis for Remote MCP

Vercel's `agent-browser` does not have a remote MCP server. Here is why the choice that was correct for them is incorrect for vaxis:

| | agent-browser | vaxis |
|---|---|---|
| Product type | Developer CLI tool | SaaS with web UI + API |
| Primary users | Developers in terminal | Developers + non-developers |
| Backend API | None | Cloudflare Workers (already running) |
| Claude Desktop users | Not the target | Yes — architects, PMs, designers |
| Claude.ai users | Not the target | Yes — team collaboration use case |
| Right choice | SKILL.md only | SKILL.md + Remote MCP |

Vercel made the right call for a developer-only CLI tool. Vaxis has a broader user base and an existing API backend that makes remote MCP a natural fit.

---

## Risks & Mitigations

| Risk | Mitigation |
|---|---|
| `/mcp` endpoint exposed publicly | Auth via `Authorization: Bearer <token>` — same validation as all other API routes |
| MCP spec evolves | Anthropic owns the spec; changes are versioned. `@modelcontextprotocol/sdk` handles protocol negotiation |
| Tool definitions drift from API | MCP tools are thin wrappers — if the underlying API function changes, the MCP tool updates in the same PR |
| Cloudflare Worker CPU limits | MCP requests are lightweight JSON operations; D1 reads are fast. No risk of hitting 50ms CPU limit |

---

## Files to Create/Edit

| File | Change |
|---|---|
| `apps/api/src/routes/mcp.ts` | New file — MCP server + tool definitions |
| `apps/api/src/index.ts` | Add `app.all('/mcp', ...)` and `app.all('/mcp/*', ...)` route |
| `apps/api/package.json` | Add `@modelcontextprotocol/sdk` dependency |

---

## See Also

- `docs/vaxis-approach-1-skillmd.md` — Approach 1: SKILL.md for Claude Code users
- `docs/vaxis-claude-integration-overview.md` — Why both approaches are needed
- [MCP Specification — modelcontextprotocol.io](https://modelcontextprotocol.io)
- [Cloudflare Agents: Build a Remote MCP Server](https://developers.cloudflare.com/agents/guides/remote-mcp-server/)
- [@modelcontextprotocol/sdk on npm](https://www.npmjs.com/package/@modelcontextprotocol/sdk)
