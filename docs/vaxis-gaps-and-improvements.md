# Vaxis — Gaps & Improvement Plan

Identified by comparing Vaxis against excalidraw-mcp and excalidraw core.
These are capabilities Excalidraw MCP has that Vaxis currently lacks, plus improvements specific to Vaxis's architecture.

---

## Summary Table

| # | Gap | Priority | Area |
|---|-----|----------|------|
| G-01 | No edit feedback loop | High | API + Frontend |
| G-02 | No format spec command | Medium | CLI + API |
| G-03 | Whole-Mermaid rewrite risk | Medium | API |
| G-04 | Only last-turn undo | Low | API + DB |
| G-05 | No MCP server | High | New service |
| G-06 | No viewport control | Low | API + Frontend |
| G-07 | Share not in CLI | Low | CLI |

---

## G-01 — No edit feedback loop

**Priority: High**

### What happens today
User opens the Vaxis web app and manually moves nodes, adds labels, rearranges arrows on the canvas. Claude has zero awareness this happened. Next time Claude runs `vaxis diagrams generate`, it reads the last Mermaid from chat history — which is the old version before the user's manual edits. Claude then overwrites those edits without knowing they existed.

### The real problem
The web canvas (Excalidraw) and the AI chat history are completely disconnected. Chat history is Claude's source of truth, but it never reflects what the user does manually on the canvas. The user's work gets silently lost on the next AI generation.

### How excalidraw-mcp solves it
Every time the user edits the canvas, a debounced `updateModelContext()` call pushes a compact text diff into Claude's context:
> *"User manually edited diagram: moved 'Payment Service' node, added 'Webhook Handler' label, deleted 'Legacy API' node"*

Claude sees this before the next generation and knows exactly what to preserve.

### What to build
Every time the user saves/edits on the web canvas (debounced, ~2 seconds after last edit), the frontend fires a background API call:

**New endpoint:** `POST /api/diagrams/:id/context`
```json
{
  "diff": "User manually edited: moved 'Payment Service', added 'Webhook Handler', deleted 'Legacy API'"
}
```

This is stored as a special `system` role message in `chat_message`. Claude reads it via `diagrams show --json` and knows what the user has manually changed. It preserves those changes on the next generate call.

**Impact:** Eliminates the most common source of data loss in Vaxis — Claude overwriting user's manual canvas edits.

---

## G-02 — No format spec command

**Priority: Medium**

### What happens today
Claude has `SKILL.md` loaded at the start of a session. If Claude forgets what diagram types Vaxis supports, what Mermaid syntax to use, or how the generate API works — there is nothing to call to refresh that knowledge. Claude either guesses or produces bad Mermaid.

### The real problem
Excalidraw MCP solves this with the `read_me` tool — a dedicated command that returns the full format reference on demand. Claude calls it once before starting and the knowledge stays in context. Vaxis has no equivalent — SKILL.md is static and only loaded at session start.

### What to build
**New CLI command:** `vaxis diagrams format --json`

Returns a structured reference containing:
- All supported diagram types with names and when to use each (flowchart, ER, sequence, state machine, class, infrastructure, journey, etc.)
- A complete working Mermaid example for each type
- Rules the AI must follow: how to write node IDs, how drill annotations work (`%% vaxis:drill <nodeId>`), character limits, unsupported syntax
- Common mistakes and how to avoid them

Claude calls this once at the start of any design session and has the full spec in context for the whole conversation. If something goes wrong mid-session, Claude can call it again to reset its understanding.

**Example output (partial):**
```json
{
  "supported_types": [
    {
      "type": "flowchart",
      "description": "System architecture, service maps, general flows",
      "example": "graph TD\n    A[User] --> B[API Gateway]\n    B --> C[Auth Service]\n    B --> D[Payment Service]"
    },
    {
      "type": "er",
      "description": "Database schema, entity relationships",
      "example": "erDiagram\n    USER ||--o{ ORDER : places\n    ORDER ||--|{ LINE_ITEM : contains"
    }
  ],
  "drill_syntax": "%% vaxis:drill <nodeId>",
  "rules": [
    "Always use alphanumeric node IDs — no spaces",
    "Max 50 nodes per diagram",
    "Use %% vaxis:drill to mark nodes that need child diagrams"
  ]
}
```

---

## G-03 — Whole-Mermaid rewrite risk

**Priority: Medium**

### What happens today
User says *"add a cache layer between the API and the database."* Claude reads the current Mermaid (which might have 50 nodes and 40 edges), rewrites the entire string with the cache added, and sends the full new Mermaid to `generate`. If Claude makes one syntax mistake anywhere — the whole diagram breaks. If Claude misremembers a node label — that node gets silently renamed.

### The real problem
Vaxis stores Mermaid as a full string. Every `generate` call replaces the entire string. There is no way to say "add only this node and this edge — leave everything else exactly as it is."

As diagrams grow larger, the rewrite risk compounds. A 10-node diagram is safe to rewrite. A 60-node diagram with complex relationships is very risky.

### How excalidraw-mcp solves it
The `create_view` tool supports a `restoreCheckpoint` pseudo-element at the start of the elements array. Claude sends only the new elements — the existing ones are loaded from the checkpoint and merged. Claude never needs to reproduce elements it didn't change.

### What to build
**New endpoint:** `POST /api/diagrams/:id/patch`

Accepts a structured diff instead of a full Mermaid replacement:
```json
{
  "add_nodes": [
    { "id": "cache", "label": "Redis Cache", "shape": "cylinder" }
  ],
  "add_edges": [
    { "from": "api", "to": "cache", "label": "read" },
    { "from": "cache", "to": "db", "label": "miss" }
  ],
  "remove_nodes": ["legacy_api"],
  "remove_edges": [],
  "update_labels": [
    { "id": "db", "label": "PostgreSQL" }
  ]
}
```

The server applies this diff to the existing Mermaid and returns the updated full Mermaid. Claude only sends what changed — the existing 50 nodes are untouched.

**New CLI command:** `vaxis diagrams patch <id> --json`

Claude uses `generate` for new diagrams and large changes, and `patch` for targeted additions or removals. This eliminates the rewrite risk for iterative updates.

---

## G-04 — Only last-turn undo

**Priority: Low**

### What happens today
`vaxis diagrams undo` removes only the last user+assistant message pair from chat history. If Claude generated 3 bad turns in a row, you can only undo the last one. There is no way to go back to the state from 2 or 3 turns ago.

### The real problem
Excalidraw MCP creates a UUID checkpoint after every `create_view` call. The AI can reference any past checkpoint by ID to restore exactly that state. Vaxis's undo is always exactly one step.

### What to build
**New DB table:** `diagram_checkpoint`
```
id (UUID), diagram_id (FK), mermaid (text), prompt (text), created_at
```

Every time `POST /api/diagrams/:id/generate` is called successfully, save a snapshot of the Mermaid before and after generation.

**New endpoints:**
- `GET /api/diagrams/:id/history` — returns list of checkpoints: `[{ id, prompt, created_at }]`
- `POST /api/diagrams/:id/restore` with `{ checkpointId }` — restores the diagram Mermaid to that checkpoint's state

**New CLI commands:**
- `vaxis diagrams history <id> --json` — lists all checkpoints for a diagram
- `vaxis diagrams restore <id> --checkpoint <checkpointId>` — restores to any past state

Claude can say: *"I'll roll back to 3 turns ago before things went wrong"* — and do it precisely instead of only being able to remove the last step.

---

## G-05 — No MCP server

**Priority: High**

### What happens today
To use Vaxis with Claude, a user must: install Rust, build the CLI, run `vaxis config set-url`, run `vaxis login`, and configure Claude to use it. This is a significant setup barrier. Most users trying Vaxis for the first time will drop off before completing it.

### The real problem
MCP (Model Context Protocol) is now the standard way AI assistants integrate with external tools. Claude Desktop, Cursor, VS Code, Windsurf, Goose — all support MCP servers natively. Excalidraw, GitHub, Linear, Notion, Figma all have MCP servers. Vaxis has none.

Without an MCP server, Vaxis is invisible to the entire MCP ecosystem.

### What to build
A Vaxis MCP server — a TypeScript/Node.js service (deployable to Cloudflare Workers or Vercel) that exposes Vaxis capabilities as MCP tools:

| MCP Tool | Maps to |
|----------|---------|
| `vaxis_apps_list` | `GET /api/applications` |
| `vaxis_apps_create` | `POST /api/applications` |
| `vaxis_diagrams_list` | `GET /api/diagrams?applicationId=X` |
| `vaxis_diagrams_create` | `POST /api/diagrams` |
| `vaxis_diagrams_generate` | `POST /api/diagrams/:id/generate` |
| `vaxis_diagrams_show` | `GET /api/diagrams/:id` + chat |
| `vaxis_diagrams_tree` | `GET /api/diagrams/:id/tree` |
| `vaxis_diagrams_undo` | `DELETE /api/diagrams/:id/chat/last` |
| `vaxis_diagrams_format` | Returns Mermaid format spec (G-02) |

**User setup (one-time):**
```json
{
  "mcpServers": {
    "vaxis": {
      "url": "https://mcp.vaxis.dev",
      "headers": { "Authorization": "Bearer <token>" }
    }
  }
}
```

No CLI install. No Rust. No config file. Claude Desktop, Cursor, and VS Code all get full Vaxis capabilities immediately. The MCP server authenticates via the same Bearer tokens already used by the CLI.

**This is the biggest unlock for adoption** — it removes every setup barrier and makes Vaxis a first-class citizen in the AI tool ecosystem.

---

## G-06 — No viewport control

**Priority: Low**

### What happens today
Claude generates a diagram with 6 subsystems. The web canvas zooms out to show the full diagram. If the most important new content is the Payment Service in the bottom-right corner, the user has to manually scroll and zoom to find it. Claude knows exactly what changed — but can't tell the canvas where to focus.

### The real problem
After generation, the web UI has no signal about which part of the diagram is the most relevant to show. The AI that just created the content knows this — but there's no channel to communicate it to the frontend.

### What to build
Add an optional `focus_node` field to the generate API response:
```json
{
  "mermaid": "...",
  "drills": [...],
  "focus_node": "payment_service"
}
```

The frontend reads `focus_node` after rendering and automatically pans and zooms the Excalidraw canvas to center on that node.

Claude sets `focus_node` to the most recently added or most important changed node. The user's view automatically lands on the new content without any manual navigation.

**Small change, meaningful UX improvement** — especially for large diagrams where new content is added far from the current viewport center.

---

## G-07 — Share not in CLI

**Priority: Low**

### What happens today
Claude finishes designing a full system. The user wants to share it with their team. They must: open the Vaxis web app, find the project, click the share button, copy the link — all manually. Claude cannot give them the link directly at the end of the session.

### The real problem
The share API exists (`POST /api/applications/:id/share` returns a token) but it is not exposed in the CLI. Claude cannot fulfill SKILL.md Pattern 5 (*"Every design session ends with a shareable link offered to the user"*) because the command to generate that link doesn't exist in the CLI.

### What to build
Two new CLI commands:

**`vaxis apps share <id> --json`**
Calls `POST /api/applications/:id/share` (creates or rotates the token) and returns the full shareable URL:
```json
{
  "url": "https://vaxis.dev/view/abc123xyz",
  "token": "abc123xyz",
  "created_at": "2026-06-05T10:00:00Z"
}
```

**`vaxis apps share revoke <id>`**
Calls `DELETE /api/applications/:id/share` to invalidate the current token.

At the end of every design session, Claude calls `vaxis apps share <appId> --json` and gives the user the link directly in the chat:
> *"Here's your shareable link: https://vaxis.dev/view/abc123xyz — anyone with this link can view the full architecture"*

No manual steps needed. This closes the loop on every design session.

---

## Implementation Priority Order

| Order | Gap | Reason |
|-------|-----|--------|
| 1 | G-05 MCP server | Biggest adoption unlock — removes all setup barriers |
| 2 | G-01 Edit feedback loop | Prevents data loss — highest real-world impact |
| 3 | G-02 Format spec command | Makes Claude reliably produce correct Mermaid |
| 4 | G-03 Partial Mermaid patch | Protects large diagrams from rewrite errors |
| 5 | G-07 Share in CLI | Completes the design session flow |
| 6 | G-04 Checkpoint history | Nice to have — current one-step undo is usually enough |
| 7 | G-06 Viewport control | Polish — low effort, low impact |
