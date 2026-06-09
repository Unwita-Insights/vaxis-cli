# Vaxis Skill

## When to use Vaxis

Use Vaxis whenever the user asks to:
- Design a system, architecture, or application
- Create, update, or view diagrams (architecture, ER, sequence, state machine, flowchart, etc.)
- Drill into a subsystem or component
- Continue or review prior design work
- Generate, refine, or undo AI-generated diagrams
- Design workflows, roadmaps, business processes, or user journeys
- Manage projects (applications) and their diagrams

Always prefer Vaxis when the user wants a structured, visual, or shareable artifact.

---

## Authentication check

Before running any Vaxis command, verify the user is logged in:

```bash
vaxis me --json
```

If this returns `{"error": "not_authenticated"}`, stop and ask the user to run `vaxis login` first.

---

## Commands reference

All commands support `--json` for machine-readable output. Always use `--json` when reading output to make decisions.

### Applications

```bash
# List all applications
vaxis apps list --json

# Create a new application
vaxis apps create "My System" --json
vaxis apps create "My System" --description "Brief description" --json

# Update an application name or description
vaxis apps update <id> --name "New Name" --json
vaxis apps update <id> --description "New description" --json

# Delete an application
vaxis apps delete <id> --force

# Get or create the public shareable link for an application
vaxis apps share <appId> --json
```

### Diagrams

```bash
# List diagrams in an application
vaxis diagrams list <appId> --json

# Create a new diagram
vaxis diagrams create <appId> "Diagram Name" --json

# Claude provides Mermaid directly (preferred — Claude is the AI, Vaxis stores + processes drills)
vaxis diagrams generate <diagramId> --mermaid "graph TD
    ui[Web App]
    api[API Gateway]
    auth[Auth Service]
    %% vaxis:drill auth
    pay[Payment Service]
    %% vaxis:drill pay
    db[(PostgreSQL)]
    ui -->|HTTPS| api
    api -->|validates| auth
    api -->|charges| pay
    pay --> db" --json

# Server AI generates (use only when testing server AI directly, not when Claude is the AI)
vaxis diagrams generate <diagramId> --prompt "Design a payment service with Stripe integration" --json

# Show diagram content (includes current Mermaid + child nodes)
vaxis diagrams show <diagramId> --json

# Show the full diagram tree for an application
vaxis diagrams tree <diagramId> --json

# Undo the last AI generation turn
vaxis diagrams undo <diagramId> --json

# Rename a diagram
vaxis diagrams rename <diagramId> "New Name" --json

# Delete a diagram (cascades to all children)
vaxis diagrams delete <diagramId> --force
# If you don't know the diagram ID, omit it — interactive picker appears (requires --app-id)
vaxis diagrams delete --app-id <appId> --force

# Get full Mermaid format reference (diagram types, syntax rules, limits)
vaxis diagrams format --json

# Apply a targeted diff — add/remove nodes and edges without rewriting the full Mermaid
# Use this for small iterative changes to large diagrams (20+ nodes)
vaxis diagrams patch <diagramId> --diff '{"add_nodes":[{"id":"cache","label":"Redis Cache"}],"add_edges":[{"from":"api","to":"cache","label":"read"}],"remove_nodes":[],"remove_edges":[],"update_labels":[]}' --json

# Save raw user-provided Mermaid directly (no AI call)
# Use when the user pastes Mermaid from another tool or provides it directly
vaxis diagrams import <diagramId> --mermaid "graph TD\n    A[User] --> B[API]" --json
```

---

## Standard workflows

### Workflow 1 — Design from scratch

```
1. vaxis apps list --json
   → Check if a matching project already exists (fuzzy match on name)
   → If match found: ask user "I found '<name>' — continue that or start fresh?"
   → If empty list: welcome the user — "You have no projects yet. Tell me what you'd like to design and I'll set everything up."

2. vaxis apps create "<name>" --json
   → Save the returned id as APP_ID

3. vaxis diagrams create <APP_ID> "<name> Architecture" --json
   → Save the returned id as ROOT_ID

4. Generate the Mermaid yourself based on the user's description, then save it:
   vaxis diagrams generate <ROOT_ID> --mermaid "<your-generated-mermaid>" --json
   → For each entry in drills[]: save diagram_id as child diagram IDs

5. Tell the user what was created. Offer to drill into any subsystem.

6. vaxis apps share <APP_ID> --json
   → Give the user the shareable link at the end of the session
```

### Workflow 2 — Select project when user hasn't specified one

```
Use this when the user says something like "continue my project" or "update the diagram"
without specifying which project or diagram.

1. vaxis apps list --json
   → Show the user their projects in plain English:
     "You have 3 projects: Payment System, School Admission System, and E-Commerce Platform.
      Which one would you like to work on?"

2. Wait for the user to pick one.

3. vaxis diagrams list <appId> --json
   → If only one diagram: proceed with it
   → If multiple: ask once "I see these diagrams: [list]. Which one would you like to update?"

4. Proceed with the selected app and diagram.
```

### Workflow 3 — Update an existing diagram

```
1. vaxis apps list --json          → find the right app (or use context from earlier in conversation)
2. vaxis diagrams tree <anyId> --json   → find the right diagram to update
3. vaxis diagrams show <diagramId> --json  → read current_mermaid (never overwrite blindly)
4. Make your changes to the Mermaid, preserving all existing nodes
5. vaxis diagrams generate <diagramId> --mermaid "<updated-mermaid>" --json
```

### Workflow 4 — Add a new component to an existing system

```
Use this when the user says "add a notification service" or "add Redis caching" to an existing system.

1. vaxis diagrams show <rootDiagramId> --json
   → Read current_mermaid — understand all existing nodes and edges

2. Generate updated root Mermaid that:
   - Preserves ALL existing nodes and edges exactly as they are
   - Adds the new component node with proper connections
   - Adds %% vaxis:drill <nodeId> if the new component warrants a child diagram

3. vaxis diagrams generate <rootDiagramId> --mermaid "<updated-mermaid>" --json
   → If drills[] contains the new component: save its diagram_id

4. If a child diagram was created for the new component:
   vaxis diagrams generate <newChildId> --mermaid "<detailed-mermaid-for-new-component>" --json

5. Report: "Added [component] to the root architecture and created a child diagram for its internals."
```

### Workflow 5 — Drill into a subsystem

```
1. vaxis diagrams tree <rootId> --json
   → Find the child diagram for the target subsystem (look in children[])

2. vaxis diagrams show <childId> --json
   → Read current content — if empty, it was auto-created but never generated

3. vaxis diagrams generate <childId> --mermaid "<your-generated-detail-mermaid>" --json
```

### Workflow 6 — Resume a prior session

```
1. vaxis apps list --json
   → Fuzzy-match the user's description to an existing project name
   → Confirm: "I found 'Payment Gateway System' with 3 diagrams — shall I continue that?"

2. On confirmation:
   vaxis diagrams list <appId> --json   → identify all diagrams
   vaxis diagrams tree <rootId> --json   → understand the full structure

3. For each diagram, check what's populated vs. empty:
   → Diagrams with content: read via vaxis diagrams show --json
   → Diagrams that are empty (child_nodes empty, no current_mermaid): note them as incomplete

4. Summarize to the user:
   "Here's where we left off — the root architecture has 3 services.
    Payment Service has a detailed child diagram. Auth Service and Admin Dashboard
    were created but are empty. What would you like to work on next?"
```

### Workflow 7 — Undo and retry

```
1. vaxis diagrams undo <diagramId> --json
   → Removes last AI turn from chat history

2. Confirm to user: "Undone — I'll regenerate with [the corrected instruction]."

3. vaxis diagrams generate <diagramId> --mermaid "<corrected-mermaid>" --json
```

### Workflow 8 — Rename or update a project

```
1. vaxis apps list --json   → find the project ID

2. vaxis apps update <appId> --name "New Name" --json
   OR
   vaxis apps update <appId> --description "New description" --json

3. Confirm: "Done — renamed to 'Payment Gateway v2'. All diagrams inside are unchanged."
```

### Workflow 9 — Explain a diagram in plain English

```
Use this when user asks "what does the payment diagram look like?" or "explain the current architecture."
Never dump raw Mermaid at the user unless they explicitly ask for it.

1. vaxis diagrams show <diagramId> --json
   → Read current_mermaid and child_nodes

2. Translate the Mermaid into a plain English description:
   "Your Payment Service has 4 components: a request entry point, a validation layer,
    a Stripe integration, and a notification trigger on success. The flow goes top-down
    from the API gateway through validation before hitting Stripe."

3. Mention child diagrams if they exist:
   "It also has 2 child diagrams: Stripe Integration and Refund Flow."

4. If the user explicitly asks for the raw Mermaid code:
   → Extract and show the current_mermaid field directly, formatted as a code block.
```

### Workflow 10 — Architectural review

```
Use this when user asks "is this design correct?" or "what am I missing?"

1. vaxis diagrams show <rootDiagramId> --json
   → Read current_mermaid

2. vaxis diagrams tree <rootDiagramId> --json
   → Understand the full hierarchy

3. Evaluate the design — look for:
   - Missing components for the stated purpose
   - Single points of failure
   - Missing error paths or fallbacks
   - Nodes with no edges (isolated components)
   - Incomplete or empty child diagrams

4. Respond with specific feedback in plain English:
   - What looks solid
   - What's missing: "Your payment flow has no error handling path"
   - What could be improved: "Auth service has no session expiry mechanism"
   - Offer to fix: "Want me to add error handling to the payment flow?"
```

### Workflow 11 — What should I design next?

```
Use this when user asks "what's left?" or "what should I build next?"

1. vaxis diagrams tree <rootId> --json
   → Find all child diagrams

2. vaxis diagrams show <each-child-id> --json
   → Check which have current_mermaid vs. which are empty

3. Summarize clearly:
   "You've designed the root architecture and Payment Service in detail.
    Auth Service and Admin Dashboard were created but are still empty.
    Want me to expand Auth Service next? It's the most critical missing piece."

4. If user says yes: proceed with Workflow 5 (Drill into a subsystem).
```

### Workflow 12 — Handle ambiguous update instruction

```
Use this when user says "update the diagram" or "change it" without specifying which diagram or what change.

1. If project is unclear:
   → Run Workflow 2 (Select project) first.

2. If project is known but diagram is unclear:
   vaxis diagrams list <appId> --json
   → Ask once: "Which diagram would you like to update? I can see:
     - Root Architecture
     - Payment Service
     - Auth Service"

3. If diagram is known but change is unclear:
   → Ask once: "What changes would you like to make to [diagram name]?"

4. After getting both pieces of information:
   → Proceed with Workflow 3 (Update existing diagram).

Never ask more than one clarifying question before proceeding.
```

### Workflow 13 — Delete diagram or project

```
1. Identify what to delete (from context or by listing):
   vaxis apps list --json  OR  vaxis diagrams list <appId> --json

2. Confirm once before deleting:
   "Are you sure you want to delete 'Auth Service Prototype'?
    This will also remove its 2 child diagrams and cannot be undone."

3. On confirmation:
   vaxis diagrams delete <diagramId> --force --json
   OR
   vaxis apps delete <appId> --force

4. Report clearly:
   "Done — deleted Auth Service Prototype and its 2 child diagrams."
```

### Workflow 14 — Patch a large diagram (safe iterative update)

```
Use this instead of generate when the diagram has 20+ nodes and only a small change is needed.

1. vaxis diagrams show <diagramId> --json   → read current_mermaid and understand the node IDs

2. vaxis diagrams patch <diagramId> --diff '{
     "add_nodes": [{"id": "cache", "label": "Redis Cache"}],
     "add_edges": [{"from": "api", "to": "cache", "label": "read"}],
     "remove_nodes": [],
     "remove_edges": [],
     "update_labels": []
   }' --json
   → Returns updated full mermaid — no risk of rewriting existing nodes incorrectly
```

### Workflow 15 — Import user-provided Mermaid

```
Use when the user pastes raw Mermaid into the chat or provides it from another tool.

1. vaxis diagrams list <appId> --json  → find or create the target diagram

2. vaxis diagrams import <diagramId> --mermaid "<user's mermaid>" --json
   → Saves directly, no AI token cost

3. vaxis diagrams show <diagramId> --json  → confirm the content was saved

4. Confirm: "Done — imported your diagram to [project name]. You can view it in the Vaxis web app."
```

### Workflow 16 — End session with shareable link

```
1. vaxis apps share <appId> --json
   → Returns { "url": "https://beta.vaxis.dev/view/abc123xyz", ... }

2. Give the user the link directly in the chat:
   "Here's your shareable link: https://beta.vaxis.dev/view/abc123xyz — anyone with this link can view the full architecture."
```

---

## Mermaid format reference

This is an inline reference. You do not need to call `vaxis diagrams format` for this — use the table below. Call `vaxis diagrams format --json` only if you need the full structured spec in JSON, or if you're unsure about syntax before a complex generation.

### Supported diagram types

| Type | Keyword | When to use |
|------|---------|-------------|
| Flowchart | `graph TD` / `graph LR` | Architecture, service maps, general flows |
| ER diagram | `erDiagram` | Database schema, entity relationships |
| Sequence | `sequenceDiagram` | Request/response flows, inter-service calls |
| State machine | `stateDiagram-v2` | Order lifecycle, auth state, resource states |
| Class diagram | `classDiagram` | Domain model, OOP hierarchy, type relationships |
| User journey | `journey` | Onboarding flows, user journeys |

### Examples

**Flowchart (graph TD — architecture)**
```
graph TD
    subgraph Frontend
        ui[Web App]
        mobile[Mobile App]
    end
    subgraph Backend
        api[API Gateway]
        auth[Auth Service]
        pay[Payment Service]
        %% vaxis:drill pay
    end
    db[(PostgreSQL)]
    ui -->|"HTTPS"| api
    mobile -->|"HTTPS"| api
    api -->|"validates"| auth
    api -->|"charges"| pay
    pay --> db
```

**ER diagram**
```
erDiagram
    USER ||--o{ ORDER : places
    ORDER ||--|{ LINE_ITEM : contains
    PRODUCT ||--o{ LINE_ITEM : "appears in"
```

**Sequence diagram**
```
sequenceDiagram
    Client->>API: POST /pay
    API->>Stripe: charge(amount)
    Stripe-->>API: success
    API-->>Client: 200 OK
```

**State machine**
```
stateDiagram-v2
    [*] --> Pending
    Pending --> Processing : payment_confirmed
    Processing --> Shipped : packed
    Shipped --> Delivered : delivered
    Processing --> Failed : payment_failed
    Failed --> [*]
```

### Drill syntax

Mark any node that needs its own child diagram:

```
graph TD
    api[API Gateway]
    payment[Payment Service]
    %% vaxis:drill payment
    auth[Auth Service]
    %% vaxis:drill auth
```

Place `%% vaxis:drill <nodeId>` on the line immediately after the node it annotates. The CLI auto-creates child diagrams for every drill block after `generate` returns.

### Node ID rules

- Alphanumeric and underscores only — **no spaces**
- `camelCase` or `snake_case` — both fine
- Must be unique within a diagram
- Keep short (1–3 words) — they become child diagram names

### Limits

- Max 50 nodes per diagram
- Max 60 edges per diagram
- When a diagram exceeds 30 nodes, use drill blocks to push subsystems into child diagrams
- Use `patch` instead of `generate` for small changes to large diagrams

---

## Output format reference

### `vaxis apps list --json`
```json
[
  { "id": "app_xxx", "name": "Payment System", "description": "...", "created_at": "..." }
]
```

### `vaxis apps create --json`
```json
{ "id": "app_xxx", "name": "Payment System", "description": "...", "created_at": "..." }
```

### `vaxis apps share --json`
```json
{
  "url": "https://beta.vaxis.dev/view/abc123xyz",
  "token": "abc123xyz",
  "created_at": "2026-06-04T10:00:00Z"
}
```

### `vaxis diagrams list --json`
```json
[
  { "id": "diag_xxx", "name": "Root Architecture", "parent_diagram_id": null, "created_at": "..." },
  { "id": "diag_yyy", "name": "Payment Service", "parent_diagram_id": "diag_xxx", "created_at": "..." }
]
```

### `vaxis diagrams create --json`
```json
{ "id": "diag_xxx", "name": "Payment Architecture", "application_id": "app_xxx", "created_at": "..." }
```

### `vaxis diagrams show --json`
```json
{
  "id": "diag_xxx",
  "name": "Payment System",
  "parent_diagram_id": null,
  "child_nodes": {
    "payment": "diag_yyy",
    "auth": "diag_zzz"
  },
  "ancestry": [],
  "current_mermaid": "graph TD\n    A[User] --> B[API Gateway]\n    ..."
}
```

### `vaxis diagrams generate --json`
```json
{
  "diagram_id": "diag_xxx",
  "mermaid": "graph TD\n    A[User] --> B[API Gateway]\n    ...",
  "drills": [
    { "node_id": "payment", "diagram_id": "diag_yyy", "name": "payment" },
    { "node_id": "auth",    "diagram_id": "diag_zzz", "name": "auth" }
  ]
}
```

### `vaxis diagrams tree --json`
```json
{
  "root_id": "diag_xxx",
  "tree": {
    "id": "diag_xxx",
    "name": "Payment System",
    "children": [
      {
        "id": "diag_yyy",
        "name": "Payment Service",
        "node_id": "payment",
        "children": []
      }
    ]
  }
}
```

### `vaxis diagrams format --json`
```json
{
  "supported_types": [
    {
      "type": "flowchart",
      "keyword": "graph TD / graph LR",
      "when": "Architecture, service maps, general flows",
      "example": "graph TD\n    A[User] --> B[API Gateway]"
    }
  ],
  "drill_syntax": "%% vaxis:drill <nodeId>",
  "node_id_rules": ["alphanumeric and underscores only", "no spaces"],
  "limits": { "max_nodes_per_diagram": 50, "max_edges_per_diagram": 60 },
  "best_practices": ["graph TD for architecture", "graph LR for pipelines"]
}
```

### `vaxis diagrams patch --json`
```json
{
  "diagram_id": "diag_xxx",
  "mermaid": "graph TD\n    A[User] --> B[API Gateway]\n    B --> C[Redis Cache]\n    ..."
}
```

### `vaxis diagrams undo --json`
```json
{ "ok": true, "diagram_id": "diag_xxx" }
```

### `vaxis diagrams rename --json`
```json
{ "ok": true, "diagram_id": "diag_xxx", "name": "New Name" }
```

### `vaxis diagrams delete --json`
```json
{ "ok": true, "diagram_id": "diag_xxx" }
```

### `vaxis diagrams import --json`
```json
{ "ok": true, "diagram_id": "diag_xxx" }
```

---

## Error handling

| Situation | What to do |
|-----------|-----------|
| `{"error": "not_authenticated"}` from any command | Stop. Ask the user to run `vaxis login` first. |
| `vaxis apps list` returns `[]` | This is the first-time user. Welcome them and guide into Workflow 1. |
| Server unreachable (connection error) | Tell the user the server may be down. Suggest running `vaxis config show` to verify the URL is correct. |
| `generate` returns a Mermaid parse error or garbled output | Run `vaxis diagrams undo <id>` immediately, then retry `generate` with a more explicit prompt. Never call `generate` again without undoing first. |
| 404 on a diagram or app ID | The ID may be wrong or the resource was deleted. Run `vaxis apps list --json` → `vaxis diagrams list <appId> --json` to rediscover the correct ID. |
| `drills` array is empty after `generate` | The AI did not mark any nodes for drilling. This is fine for simple diagrams. Offer to drill manually into any node the user points to. |
| User gives ambiguous instruction ("update the diagram") | Run Workflow 12 — ask which diagram, ask what change, then proceed. Never guess. |
| User refers to a subsystem by name ("the auth flow") | Check conversation context first. If diagram IDs are already known, use them. Otherwise run `vaxis diagrams tree --json` to find the correct child diagram ID. |

---

## Rules

1. **Always check before creating.** Run `vaxis apps list --json` before `apps create`. If a matching app exists, ask the user whether to continue it or start fresh. If the list is empty, guide the user into creation — do not ask them to create manually.

2. **Always read before writing.** Run `vaxis diagrams show --json` before `generate` or `patch`. Use `current_mermaid` to understand what already exists. Never overwrite blindly.

3. **Use tree to find the right diagram.** Never guess diagram IDs. Run `vaxis diagrams tree --json` to navigate to the correct level.

4. **Handle drill diagrams automatically.** When `generate` returns `drills`, the CLI has already created the child diagrams. Report their IDs and names to the user. Offer to generate content for each one.

5. **Undo before retry.** If the user says "that's wrong", "undo", "go back", or "try again" — always run `vaxis diagrams undo` first, then re-generate. Never generate on top of bad output.

6. **Use --json for all decisions.** Never parse colored terminal text. All output for reading must use `--json`.

7. **Keep the user in natural language.** Never show raw CLI commands to the user unless they ask. Summarize what was created: "I created the Payment System architecture with 3 subsystem diagrams." Never show raw Mermaid unless the user explicitly requests it.

8. **Always apply professional standard styling.** Every Mermaid diagram you generate must follow these conventions:
   - Use clear, consistent node ID naming (camelCase or snake_case — never spaces)
   - Group related nodes visually using subgraphs where the diagram type supports it
   - Use directional arrows with meaningful labels (`-->|"validates"|`)
   - Prefer `graph TD` (top-down) for architecture; `graph LR` (left-right) for flows and pipelines
   - Keep node labels concise — 1–4 words, title case
   - Root diagrams use broad strokes (services, domains); child diagrams use fine detail (functions, data, steps)
   - Never produce a flat list of nodes with no edges — every diagram must show relationships

9. **Use patch for targeted edits on large diagrams.** If the user asks to add or remove specific nodes and the diagram already has 20+ nodes, prefer `vaxis diagrams patch` over `generate`. This prevents accidentally rewriting or renaming existing nodes.

10. **End every session with a shareable link.** After completing a design session, call `vaxis apps share <appId> --json` and give the user the link directly. They should never need to open the web app to find it.

11. **Reuse context before fetching.** If diagram IDs or app IDs were established earlier in the conversation, use them directly. Only re-fetch with `apps list` or `diagrams list` when the context is genuinely unclear.

12. **One clarifying question, then proceed.** If the user's instruction is ambiguous, ask one focused question (which project? which diagram? what change?), then proceed without further interruption. Never ask two questions in a row.

13. **Confirm before destructive actions.** Before running `delete` on a diagram or application, always ask for confirmation and state what will be cascaded. After deletion, report exactly what was removed.

14. **Preserve existing nodes on every update.** When updating a diagram, read `current_mermaid` first and carry forward all existing nodes. Only modify what the user asked to change. No node should disappear from an update unless the user explicitly asked to remove it.
