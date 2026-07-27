---
name: vaxis-core
description: Complete Vaxis CLI workflow and Vaxis-compatible Mermaid authoring instructions.
---

# Vaxis Skill

**Model-agnostic — works with Claude, GPT, Gemini, and any LLM.**

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

## Generation mode — check before you generate

The user has a stored preference for **how** diagrams get created. Before your first
`diagrams generate`, read it as JSON (Rule 6 — never parse colored text):

```bash
vaxis config show --json
# → { "auth_url": "https://app.vaxis.dev", "generation_mode": "mermaid" }
```

Look at the `generation_mode` field (may be `null`) and honor it:

- **`"generation_mode": "mermaid"`** (or `null` / unavailable) — **you** (the AI assistant)
  write the Mermaid code and pass it with `--mermaid`. This is the default and preferred
  flow: works with any LLM (Claude, GPT, Gemini, etc.), instant, deterministic, and drills
  are parsed from your `%% vaxis:drill` markers. You follow the **diagram generation rules**
  (model-independent) to structure the diagram correctly.

- **`"generation_mode": "prompt"`** — **Vaxis server AI** generates the diagram.
  Send the user's request with `--prompt` instead of writing the Mermaid yourself:
  `vaxis diagrams generate <id> --prompt "<the user's description>"`. This path supports
  `--intent` / `--session` and is subject to server-AI rate limits. The server uses its own
  internal generation logic.

Treat `null`/unset as `mermaid`. You never need to prompt the user for this — the CLI asks
them once, on their first interactive `diagrams generate`, and remembers the answer.

### Diagram Generation Rules

When generating Mermaid with `--mermaid`, follow the **universal diagram rules** below.
These rules work identically for any LLM — the structure and logic are model-independent.
The key is consistent application of shapes, hierarchy, and topology, not model-specific
features.

---

## Multi-LLM Compatibility

This skill is **model-agnostic** and works identically with:
- **Claude** (Haiku, Sonnet, Opus)
- **GPT-4 / GPT-4o** (OpenAI)
- **Gemini** (Google)
- **Any other LLM** (Llama, Mistral, etc.)

The diagram generation **rules are universal** — not specific to any model. What matters is
consistent application of shapes, hierarchy, topology, and preservation rules. The LLM's role
is to follow these rules when generating Mermaid code; the structure is the same regardless
of which model does the work.

If you need model-specific tuning (context windows, token budgets, API patterns), handle that
in your model's **own system prompt** — this skill stays pure to the diagram authoring contract.

When writing Mermaid with `--mermaid`, follow these core rules. They are model-independent
and work identically for Claude, GPT, Gemini, or any LLM. The structure and logic are what
matter, not which AI generates them.

### 1. Domain Detection
Classify the diagram as **SOFTWARE** or **NON-SOFTWARE** before generating:

**SOFTWARE signals:** API, backend, frontend, microservice, service, server, client, web app, database, DB, Postgres, cache, queue, Kafka, SQS, S3, Docker, Lambda, OAuth, JWT, REST, GraphQL, gRPC, or any tech stack.

**NON-SOFTWARE signals:** biology, anatomy, ecosystems, animals, plants, recipes, org charts, family trees, history, art, geography, sports plays, education.

- **In SOFTWARE diagrams:** use tech vocabulary freely (Service, API, Gateway, Database, Cache, Queue). Shapes are strict: rectangles for services/gateways, **cylinders for storage**, **rhombus for yes/no decisions only**.
- **In NON-SOFTWARE diagrams:** use natural domain terms (Species, Habitat, Migration — NOT "Species Identifier", "Habitat Module", "Migration Tracker"). Mostly rectangles; minimal shape variety.

### 2. Shape Mapping (Software Diagrams Only)

| Label contains | Shape | Syntax |
|---|---|---|
| `db`, `database`, `cache`, `queue`, `store`, `storage`, `bucket`, `kafka`, `redis`, `postgres`, `mongo`, `mysql`, `kv`, `r2`, `d1`, `table`, `log`, etc. (case-insensitive) | **Cylinder** | `nodeId[("Label")]` |
| Ends with `?` (yes/no branch) | **Rhombus** | `nodeId{"Question?"}` |
| Everything else (services, APIs, gateways, UIs, load balancers, handlers) | **Rectangle** | `nodeId["Label"]` |

**Self-check before returning:** every storage label → cylinder; every `?` label → rhombus; a software diagram with persistence has ≥1 cylinder; a branching flow has ≥1 rhombus.

### 3. Richness to Intent

- **Bare object** ("add a laptop") → emit exactly that node, no invented neighbors.
- **Integration/Implementation** ("add Supabase with OAuth", "implement Stripe") → expand into real sub-components and edges. Supabase = Postgres + Auth + Storage + Realtime + Edge Functions + APIs. Stripe = Checkout + Webhooks + Billing + Customer store. One-box answers are **wrong**.
- **Open exploration** ("design an e-commerce backend") → produce a coherent multi-component architecture.
- **Named real tools:** use your knowledge of how they actually work internally, not generic placeholders.

### 4. Flowchart Direction

- **`flowchart LR`** — genuine sequences: pipelines, processes, lifecycles, step-by-step chains (grind → brew → pour; ingest → transform → load).
- **`flowchart TB`** — layered architectures and hierarchies (clients → services → data).
- **Explicit user request always wins.** When editing, preserve the existing direction unless the user asks to change it.

### 5. Hierarchy via Drilling (THE Core Feature)

**Emit drill blocks when:**
- User explicitly asks: "deep dive into X", "show internals of X", "expand X".
- The node genuinely hides real structure (a "Payments" service containing 5+ sub-services).
- **Production-system request** ("production-grade architecture", "end-to-end system") → drill 3–6 composite services, skip leaf nodes.
- **System default:** any fresh SOFTWARE system with 4+ composite services gets a drill per composite.
- **Canvas overflow:** content doesn't fit one readable diagram → restructure into main + drills.

**Never drill:** tiny diagrams, deliberately small asks, atomic nodes (single DB, cache, queue, UI screen, external API).

**Format (at column 0, after complete main diagram):**
```
graph TD
  api["API Gateway"]
  auth["Auth Service"]
  api --> auth

%% vaxis:drill auth
flowchart TB
  login["Login"]
  tokenMgr["Token Manager"]
  login --> tokenMgr
```

**Three rules (break any one and drills silently fail):**
1. **Column 0, no indentation.** `%% vaxis:drill <nodeId>` at the very start of its line.
2. **After the complete main diagram, never between nodes.** Everything before the first marker is the main diagram.
3. **`<nodeId>` must exist in the main diagram.** Exact ID match required.

### 6. Fan-Out Cap & DAG Topology

- **Max ~4 connections per node** (in + out combined). A hub with 5+ spokes renders as a tangled star.
- **Route extras through the layer that owns them:** instead of `orderService` → [payment, db, notification, merchant, dispatch], do `orderService` → `eventQueue` (one fan-out point), then `eventQueue` → [notification, merchant, dispatch]. Keep `orderService` → [payment, db] direct.
- **Prefer layered DAG:** clear tiers (UI → services → data), edges flowing primarily one direction.

### 7. Preservation on Edit (Critical)

When editing an existing diagram:
- **Read `current_mermaid` first.** Never overwrite blindly.
- **Keep EVERY existing node and edge unchanged** unless the user explicitly asked to remove them.
- **Only add/modify what was requested.** Silently dropping a node = silent deletion.
- **Copy each node's ID and shape syntax verbatim** — don't reshape an untouched node.
- **Keep the flowchart direction (TB/LR)** unless the user asks to change it.

Exception: **Whole-canvas transform** ("turn this into a hospital system") → the old content is replaced; re-derive the new system at full richness as if starting from scratch.

### 8. Color & Subgraphs

- Use `subgraph`blocks to group related nodes — **this is what produces colored groups in Vaxis**.
- Keep subgraphs **flat** (never nested); Vaxis auto-assigns distinct soft colors.
- Optional `style <subgraphId> fill:#color,stroke:#color` — helpful for non-Vaxis viewers, Vaxis overrides it with its own palette.
- Prefer semantic grouping (Backend, Frontend, Data, Infrastructure) over color decoration.

### 9. Node Labels & IDs

- **Labels:** concise (2–5 words), title case, readable on screen.
- **IDs:** camelCase or snake_case, alphanumeric + underscores only, no spaces.
- Keep IDs short (1–3 words) — they become child diagram names.

### 10. Limits

- **50 nodes, 60 edges per diagram** (hard ceiling).
- Don't design *to* the limit — structure as a hierarchy. Keep the root readable (~10 major nodes), push detail into drill children.
- For small edits to large diagrams: read `current_mermaid`, preserve every node, resend the full diagram (Workflow 14).

---

## Commands reference

All commands support `--json` for machine-readable output. Always use `--json` when reading output to make decisions.

### Skills

```bash
# Install the discovery skill (interactive by default)
vaxis install --skills

# Scripted project install for Codex
vaxis install --skills --agent codex --project --yes --json

# Inspect the version-matched instructions embedded in this CLI
vaxis skills list --json
vaxis skills path core --json
vaxis skills get core
vaxis skills preview core
```

`--json` never prompts. Missing install selections and unknown skill names return
`{"error":"<stable_code>","message":"<details>"}` with exit status `1`. `get core` prints the
exact raw embedded content; `preview core` currently prints the same content for explicit
human inspection. Codex installs use the shared `.agents/skills/vaxis/SKILL.md` path at both
project and global scope.

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
vaxis apps delete <id> --force --json

```

### Diagrams

```bash
# List diagrams in an application
vaxis diagrams list <appId> --json

# Create a new diagram
vaxis diagrams create <appId> "Diagram Name" --json

# Share a diagram (this is THE share command — one link unlocks this diagram
# plus the sub-diagrams it drills into). Safe to call repeatedly: it returns the
# existing link if there is one, and only creates a link when there is none.
vaxis diagrams share <diagramId> --json
vaxis diagrams share <diagramId> --rotate --json   # mint a new link, breaking the old one
vaxis diagrams share <diagramId> --revoke --json   # turn sharing off

# Claude provides Mermaid directly (preferred — Claude is the AI, Vaxis stores + processes drills)
# Drill markers go at COLUMN 0 (no indentation) AFTER the complete main diagram — see "Drill syntax".
vaxis diagrams generate <diagramId> --mermaid "graph TD
    ui[Web App]
    api[API Gateway]
    auth[Auth Service]
    pay[Payment Service]
    db[(PostgreSQL)]
    ui -->|HTTPS| api
    api -->|validates| auth
    api -->|charges| pay
    pay --> db
%% vaxis:drill auth
%% vaxis:drill pay" --json

# Server AI generates (use only when testing server AI directly, not when Claude is the AI)
vaxis diagrams generate <diagramId> --prompt "Design a payment service with Stripe integration" --json
# Server-AI intent + chat session are optional (server-AI path only). Default intent is `auto`.
#   --intent auto|edit|replace|drill|detail|simplify|ask   --session <chatSessionId>
vaxis diagrams generate <diagramId> --prompt "add a Redis cache between api and db" --intent edit --json

# Ask a question about a diagram — server AI answers in prose, makes no edit
vaxis diagrams ask <diagramId> --prompt "What talks to the database?" --json

# AI chat sessions (server-AI conversation threads on a diagram)
vaxis diagrams sessions list <diagramId> --json
vaxis diagrams sessions create <diagramId> --title "Refactor pass" --json
vaxis diagrams sessions rename <diagramId> <sessionId> "New title" --json

# Show diagram content (includes current Mermaid + child nodes)
vaxis diagrams show <diagramId> --json

# Show the full diagram tree for an application
vaxis diagrams tree <diagramId> --json

# Undo the last AI generation turn
vaxis diagrams undo <diagramId> --json

# Rename a diagram
vaxis diagrams rename <diagramId> "New Name" --json

# Delete a diagram (cascades to all children)
vaxis diagrams delete <diagramId> --force --json
# If you don't know the diagram ID, list the application first and use the returned ID
vaxis diagrams list <appId> --json

# Get full Mermaid format reference (diagram types, syntax rules, limits)
vaxis diagrams format --json

# Compare the embedded authoring rules with the connected Vaxis server
vaxis diagrams rules-check --json

# Evaluate recorded prompt/direct Mermaid captures offline against the parity catalog
vaxis diagrams evaluate --captures <captures.json> --json
vaxis diagrams evaluate --captures <captures.json> --output <report.json> --json

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

4. If the description is too thin to draw accurately, clarify FIRST (Rule 17):
   → Ask 2–4 focused questions with selectable options (use your environment's UI if available)
     — main components? what connects to what? datastore? external services?
   → Skip this when the description is already specific enough to draw something useful.

5. Generate the Mermaid yourself based on the description (and answers), then save it:
   vaxis diagrams generate <ROOT_ID> --mermaid "<your-generated-mermaid>" --json
   → For each entry in drills[]: save diagram_id as child diagram IDs

6. Tell the user what was created. Offer to drill into any subsystem.

7. vaxis diagrams share <ROOT_ID> --json
   → Give the user the shareable link at the end of the session. Share the ROOT
     diagram — the link covers the diagrams it drills into.
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
   vaxis apps delete <appId> --force --json

4. Report clearly:
   "Done — deleted Auth Service Prototype and its 2 child diagrams."
```

### Workflow 14 — Edit a large diagram (preserve every existing node)

```
Use this when the diagram has many nodes and only a small change is needed.
There is no diff/patch endpoint — you are the AI, so you make the edit yourself.

1. vaxis diagrams show <diagramId> --json   → read current_mermaid and note every existing node ID

2. Edit the Mermaid yourself: carry forward ALL existing nodes and edges unchanged,
   then add / remove / modify only what the user asked for.

3. vaxis diagrams generate <diagramId> --mermaid "<full updated mermaid>" --json
   → Resend the COMPLETE diagram. Never drop a node the user didn't ask to remove (see Rule 14).
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
1. vaxis diagrams share <rootDiagramId> --json
   → Returns { "url": "https://app.vaxis.dev/view/abc123xyz", ... }

2. Give the user the link directly in the chat:
   "Here's your shareable link: https://app.vaxis.dev/view/abc123xyz — anyone with this link can view the full architecture."
```

---

## Mermaid format reference

This is an inline reference. You do not need to call `vaxis diagrams format` for this — use the table below. Call `vaxis diagrams format --json` only if you need the full structured spec in JSON, or if you're unsure about syntax before a complex generation.

### Supported diagram types

**Editable / re-generatable types** (only `flowchart` supports drill blocks):

| Type | Keyword | When to use |
|------|---------|-------------|
| Flowchart | `flowchart TB` / `flowchart LR` (`graph TD/LR` also works) | Architecture, services, processes, data flow — **the default**, and the only drillable type |
| Sequence | `sequenceDiagram` | Request/response, protocol, API interaction, lifecycle over time |
| Class diagram | `classDiagram` | Object models, domain entities, inheritance/composition |
| ER diagram | `erDiagram` | Database entities, tables, relationships, cardinality |
| State machine | `stateDiagram-v2` | Finite states, lifecycle, status transitions, workflow states |

**Image-fallback types** — valid Mermaid, but rendered as a **static image** (NOT editable or drillable). Use only when the user explicitly asks for that family: `gantt`, `pie`, `journey`, `timeline`, `mindmap`, `requirementDiagram`, `C4`, `sankey`, `xychart`, `block`, `kanban`, `radar`, `treemap`, `venn`, and more. (Note: `journey` is image-fallback, **not** an editable type.)

When editing an existing diagram, keep its current type unless the user explicitly asks to convert it.

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
    end
    db[(PostgreSQL)]
    ui -->|"HTTPS"| api
    mobile -->|"HTTPS"| api
    api -->|"validates"| auth
    api -->|"charges"| pay
    pay --> db
%% vaxis:drill pay
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

Mark any node that needs its own child diagram. **Write the complete main diagram
first (every node AND every edge), then list the drill markers after it:**

```
graph TD
    api[API Gateway]
    payment[Payment Service]
    auth[Auth Service]
    api --> payment
    api --> auth
%% vaxis:drill payment
%% vaxis:drill auth
```

**Three rules — break any one and you silently get zero drills:**

1. **Column 0, no indentation.** Each `%% vaxis:drill <nodeId>` must start at the very
   beginning of its line. The server only recognises a marker matching `^%% vaxis:drill`;
   an indented marker is treated as an ordinary Mermaid comment and ignored.
2. **After the complete main diagram, never between its nodes.** Everything *before* the
   first marker is the main diagram; everything *after* a marker (up to the next marker)
   is treated as that node's child content. A marker placed mid-diagram therefore swallows
   the nodes and edges that follow it into a drill block and mangles the main diagram.
3. **`<nodeId>` must be a node defined in the main diagram above** (exact ID). Markers whose
   ID isn't a real node in the main diagram are dropped.

Leave a marker bare (as above) to auto-create an **empty** child you fill later with a
separate `generate <childId> --mermaid`. Optionally, seed the child in the same call by
following the marker with a full sub-diagram:

```
graph TD
    api[API Gateway]
    auth[Auth Service]
    api --> auth
%% vaxis:drill auth
flowchart TB
    login[Login]
    tokenMgr[Token Manager]
    sessionStore[(Session Store)]
    login --> tokenMgr --> sessionStore
```

A seeded child contains ONLY that node's new internals — never repeat a parent node ID
inside it, and drill blocks are one level deep (a child may not contain its own markers).
When you seed a child, give it **at least 3 real, meaningfully named nodes** connected by
useful edges. The server drops non-empty seeded drills below this floor. This minimum does
not apply to a bare marker: a bare `%% vaxis:drill <nodeId>` intentionally creates an empty
child to fill later and must remain supported.
The CLI auto-creates a child diagram for every marker after `generate` returns. **Drill
blocks work on flowcharts only** — never add them to sequence / class / er / state or any
image-fallback diagram.

**Drilling is Vaxis's core feature — use it by default for architecture.** When you draw an architecture or system diagram, structure it as a hierarchy from the start, not a flat single-level graph:

- **Top level = the major subsystems** (services, domains, bounded contexts) — the broad strokes only.
- **Every composite subsystem gets a `%% vaxis:drill`** — any node with real internal structure worth its own diagram (a service made of components, a subsystem with steps/parts). Its internals belong in the child diagram, not crammed into the root.
- **Atomic / leaf nodes do NOT drill** — a database, cache, message queue, or external SaaS (e.g. `PostgreSQL`, `Redis`, `Stripe API`) has nothing inside worth a child diagram. Drilling these produces empty, noisy children — don't.

Worked example — "draw a payment system architecture":

```
graph TD
    web[Web App]
    api[API Gateway]
    auth[Auth Service]
    pay[Payment Service]
    order[Order Service]
    db[(PostgreSQL)]
    cache[(Redis)]
    web --> api
    api --> auth
    api --> pay
    api --> order
    order --> db
    pay --> db
    api --> cache
%% vaxis:drill auth
%% vaxis:drill pay
%% vaxis:drill order
```

The three services drill (composite); the database and cache don't (leaf). Flattening auth/pay/order's internals into the root — or drilling `db`/`cache` — is wrong.

Don't over-fragment: a genuinely small diagram (a handful of nodes with no real subsystems) needs no drills — an empty `drills[]` is correct there. The rule is **"drill composites," not "drill everything."**

### Shape & color conventions

<!-- vaxis-authoring-rules: 1.0.0 -->

Both the `--mermaid` and `--prompt` paths store Mermaid and render through the **same**
server normalization and browser styling — palette coloring by subgraph, shape coercion,
and ELK layout. The visual polish is *not* something only the server AI gets; the renderer
applies it to your diagram too. What differs is the graph **structure** you author, and that
structure is what the styling acts on: group nodes into subgraphs (that is what gets them
colored), pick the right shapes, and build a hierarchy. Get the structure right and a
`--mermaid` diagram renders on par with `generate --prompt`. These rules are a **condensed
mirror** of `vaxis`'s own prompt rules (`S_FLOWCHART_SHAPES`, `S_COLOR`, `S_OUTPUT_FLOWCHART`,
`S_DRILL` in `apps/api/src/prompts.ts`, and `STORAGE_KEYWORD_TOKENS` in
`packages/scene-serializer/src/shapeRules.ts`) — see the STRONG RULE in this repo's
`CLAUDE.md` if that source ever needs re-checking for drift.

**Match richness to intent:** a bare-object request gets only that object; do not pad it
with invented neighbors. An integration or implementation request for a named real-world
tool must show its real relevant capabilities and connections rather than one generic box
(for example, Supabase can expand to Postgres, Auth, Storage, Realtime, Edge Functions,
and APIs; Stripe can expand to Checkout, Webhooks, Billing, and customers). A broad design
request gets a coherent multi-component architecture. If you do not know a named tool,
model only the role the user described rather than inventing internals.

**Use domain-appropriate vocabulary:** classify the diagram as SOFTWARE or NON-SOFTWARE
from the request and surrounding diagram. Software diagrams may use terms such as Service,
API, Database, Queue, and Gateway. In non-software diagrams, use the natural concept name
and do not invent software suffixes such as Module, Service, Database, DB, Store, Manager,
Handler, Engine, Tracker, Analyzer, Identifier, Processor, System, Gateway, or API. For
example, prefer `Habitat`, `Migration`, and `Threat` over `Habitat Module`, `Migration
Tracker`, and `Threat Engine`.

**Choose direction from structure:** use `flowchart LR` for a genuine sequence—pipeline,
process, lifecycle, journey, or step-by-step chain—and `flowchart TB` for layered
architectures and hierarchies. An explicit user direction always wins. When editing,
preserve the existing direction unless the user explicitly asks to change it.

**Shape mapping (flowcharts, software diagrams) — not optional, self-check before returning:**

| Shape | Syntax | When |
|---|---|---|
| Rectangle | `nodeId["Label"]` | Default — services, APIs, gateways, UIs, load balancers. Even "gateway" stays a rectangle — diamonds squeeze text. |
| Cylinder | `nodeId[("Label")]` | **Required** whenever the label contains a storage word (case-insensitive, whole-word match): `db`, `database`, `store`, `storage`, `cache`, `queue`, `bucket`, `table`, `log`, `index`, `vector store`, `blob`, `s3`, `redis`, `postgres(ql)`, `mongo(db)`, `mysql`, `sqlite`, `kafka`, `sqs`, `d1`, `kv`, `r2`, `sql`, `nosql`, `dynamodb`, `firestore`, `memcached`, `elasticsearch`, `gcs`. |
| Rhombus | `nodeId{"Label?"}` | **Required** only when the label is a genuine yes/no branch (ends in `?`) — `authCheck{"Authenticated?"}`. Never for a service name. |

Before returning a flowchart, self-check it: every storage-token label uses a cylinder;
every label ending in `?` uses a rhombus; a software diagram that mentions persistence has
at least one cylinder; and a flow with genuine branching has at least one rhombus. Fix the
Mermaid before sending it when any check fails.

**Forbidden for new nodes** (won't render correctly in Vaxis): hexagon `{{"..."}}`, stadium `(["..."])`, circle `(("..."))`, Mermaid v11 `nodeId@{shape:...}`, or any "shape-name in parens" like `nodeId(rounded["..."])`. Exception: an *existing* node already using one of these — copy it through unchanged, don't reshape it.

**Group related nodes into flat `subgraph` blocks — this is what produces the colored groups.**
Vaxis's renderer assigns each subgraph a distinct color automatically, by its order among the
subgraphs; a flat, ungrouped graph gets no group color, which is what makes a diagram look
plain. The load-bearing act is the grouping itself, not a color directive.
```
subgraph backendLayer["Backend"]
    api["API Gateway"]
    auth["Auth Service"]
end
style backendLayer fill:#FFF3E0,stroke:#BF360C,color:#BF360C
```
The `style <id> fill:...` line is **optional** — Vaxis's primary (v2) renderer overrides it
with its own palette. Keep it only as harmless fallback for non-Vaxis Mermaid viewers and
Vaxis's fallback render path; if you add one, give each subgraph a distinct soft fill (rotate
`#E1F5FE`, `#E8F5E9`, `#FFF3E0`, `#F3E5F5`, `#ECEFF1`). Keep subgraphs flat (never nested).

**Fan-out cap:** at most ~4 connections (in + out) per node. A node wired to 5+ peers renders
as a tangled star — route the extras through the layer/bus that owns them instead of wiring
everything directly to one hub.

Keep flowcharts close to a **layered DAG**: arrange clear tiers such as entry/UI → core
services → data/infra and let edges flow primarily in one direction. Avoid a mesh of
cross-cutting connections. Connect a shared utility from the layer that owns it, or relay
fan-out through a gateway, bus, or queue, instead of connecting every component directly.

**Drill by default at scale:** a fresh SOFTWARE system with **4 or more composite services**
gets a drill block per composite service as the default posture, not an exception — this
sharpens the composite-vs-leaf rule above with the server AI's own numeric threshold. Fewer
than 4 composite services, or a genuinely small/simple ask, needs no drills.

### Node ID rules

- Alphanumeric and underscores only — **no spaces**
- `camelCase` or `snake_case` — both fine
- Must be unique within a diagram
- Keep short (1–3 words) — they become child diagram names

### Limits

- Max 50 nodes per diagram
- Max 60 edges per diagram
- 50 nodes / 60 edges is a hard ceiling — don't design up to it. Structure architecture as a drill hierarchy from the start (see **"Drilling is Vaxis's core feature"** above): keep the root readable (roughly a dozen major nodes) and push detail into drill children rather than growing one flat diagram toward the cap
- For small changes to large diagrams, edit `current_mermaid` and resend the full diagram via `generate --mermaid`, preserving every existing node (see Workflow 14)

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

### `vaxis diagrams share --json`
```json
{
  "diagram_id": "dgm_xxx",
  "shared": true,
  "url": "https://app.vaxis.dev/view/abc123xyz",
  "token": "abc123xyz",
  "edit_url": "https://app.vaxis.dev/collab/def456uvw",
  "edit_token": "def456uvw"
}
```
`url`/`token` are the read-only view link; `edit_url`/`edit_token` are the collaborative
edit link. Give people the plain `url` unless they need to edit.

`--revoke` returns `{ "ok": true, "diagram_id": "...", "shared": false }`.

### `vaxis diagrams ask --json`
```json
{ "answer": "The API Gateway and the Payment Service both write to PostgreSQL.", "unchanged": true, "chat_session_id": "sess_xxx" }
```

### `vaxis diagrams sessions list --json`
```json
{
  "sessions": [
    { "id": "sess_xxx", "title": "Main", "is_active": 1, "message_count": 4, "created_at": "...", "updated_at": "..." }
  ],
  "active_chat_session_id": "sess_xxx"
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

**A `--prompt` generate does not always edit the diagram.** The server routes the turn
to Ask when `--intent ask` is given *or* when the intent is `auto` (the default) and the
prompt reads as a question. It also declines no-op or mode-mismatched turns. Those
responses look like this instead — note `unchanged: true` and the empty `drills`:
```json
{
  "diagram_id": "diag_xxx",
  "mermaid": "graph TD\n    ...",
  "drills": [],
  "unchanged": true,
  "answer": "The auth service validates the JWT before the gateway routes ..."
}
```
When `unchanged` is `true`, **`mermaid` is the diagram's existing content echoed back, not
a new version — do not treat it as an edit and do not report one.** Surface `answer` (or
`notice` / `mode_mismatch.message` when there is no `answer`) to the user. If you wanted an
edit, re-run with an explicit `--intent edit|replace|drill`, or supply `--mermaid` yourself.
The `--mermaid` path never routes to Ask.

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
  "schema_version": "1.0.0",
  "editable_types": [
    {
      "type": "flowchart",
      "keyword": "flowchart TB / flowchart LR (graph TD/LR also works)",
      "when": "Architecture, services, processes, data flow, general diagrams",
      "drillable": true,
      "example": "flowchart TB\n    A[User] --> B[API Gateway]"
    }
  ],
  "editable_types_note": "These 5 types (flowchart, sequence, class, er, state) are editable/re-generatable in Vaxis. Only flowchart supports drill blocks / child diagrams. Prefer flowchart for general architecture.",
  "image_fallback_types": ["gantt", "pie", "journey", "timeline", "mindmap", "..."],
  "image_fallback_note": "Valid Mermaid, but rendered as a static image in Vaxis — NOT editable or drillable. Use only when the user explicitly asks for that family. Note: 'journey' is image-fallback here, not an editable type.",
  "drill_syntax": "%% vaxis:drill <nodeId>",
  "drill_description": "FLOWCHART ONLY. Do NOT use drill blocks with sequence/class/er/state or any image-fallback type.",
  "preserve_type_on_edit": "When editing an existing diagram, keep its current type unless the user explicitly asks to convert it.",
  "node_id_rules": ["alphanumeric and underscores only", "no spaces"],
  "limits": {
    "max_nodes_per_diagram": 50,
    "max_edges_per_diagram": 60,
    "max_connections_per_node": 4,
    "min_seeded_drill_nodes": 3
  },
  "authoring_contract": {
    "richness": { "integration": "Expand named real-world tools into relevant real capabilities and connections." },
    "domain": { "non_software": "Use natural domain concepts; do not invent software component suffixes." },
    "shapes": { "storage": "cylinder", "decision": "rhombus", "storage_keywords": ["db", "database", "..."] },
    "topology": { "preferred": "layered DAG" },
    "direction": { "LR": "Pipelines and strict step chains.", "TB": "Layered architectures and hierarchies." },
    "drills": { "bare_marker": "Valid and creates an empty child.", "seeded_min_nodes": 3 }
  },
  "best_practices": ["flowchart TB for architecture", "flowchart LR for pipelines"]
}
```
Full response includes all 5 editable types (flowchart, sequence, class, er, state) and the complete `image_fallback_types` list — see `format_cmd` in `src/commands/diagrams.rs` for the authoritative shape.

### `vaxis diagrams rules-check --json`
```json
{
  "ok": true,
  "cli_version": "1.0.0",
  "server_version": "1.0.0",
  "drift": []
}
```
This command requires authentication and exits with status `2` when the contracts are
reachable but incompatible. Network, authentication, and response errors use status `1`.

### `vaxis diagrams evaluate --captures <captures.json> --json`
```json
{
  "report_version": "1.0.0",
  "summary": {
    "total_captures": 2,
    "passed_captures": 1,
    "failed_captures": 1,
    "missing_case_ids": []
  },
  "results": [
    {
      "case_id": "strict-step-chain",
      "path": "mermaid",
      "model": "example-model",
      "rules_version": "1.0.0",
      "captured_at": "2026-07-22T00:00:00Z",
      "viewport": { "width": 1440, "height": 900 },
      "theme": "light",
      "metrics": {
        "direction": "LR",
        "node_count": 4,
        "edge_count": 3,
        "subgraph_count": 0,
        "max_connections": 2,
        "cylinder_count": 0,
        "rhombus_count": 0,
        "drill_count": 0
      },
      "failures": []
    }
  ]
}
```
Evaluation is offline and always emits the report as JSON unless `--output` writes it to a
file. It exits with status `2` when deterministic expectations fail and status `1` for invalid
input or file errors.

### `vaxis diagrams undo --json`
```json
{ "ok": true, "diagram_id": "diag_xxx" }
```

### `vaxis diagrams rename --json`
```json
{ "ok": true, "diagram_id": "diag_xxx", "name": "New Name" }
```

### `vaxis diagrams delete <id> --force --json`
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
| `drills` array is empty after `generate` | Fine for a genuinely small diagram with no real subsystems. But if you just drew an **architecture** with composite subsystems and got no drills, you under-structured it (Rule 15) — add `%% vaxis:drill` to each composite node and regenerate. |
| User gives ambiguous instruction ("update the diagram") | Run Workflow 12 — ask which diagram, ask what change, then proceed. Never guess. |
| User refers to a subsystem by name ("the auth flow") | Check conversation context first. If diagram IDs are already known, use them. Otherwise run `vaxis diagrams tree --json` to find the correct child diagram ID. |

---

## Rules

1. **Always check before creating.** Run `vaxis apps list --json` before `apps create`. If a matching app exists, ask the user whether to continue it or start fresh. If the list is empty, guide the user into creation — do not ask them to create manually.

2. **Always read before writing.** Run `vaxis diagrams show --json` before `generate`. Use `current_mermaid` to understand what already exists. Never overwrite blindly.

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

9. **Edit large diagrams by regenerating with care.** If the user asks to add or remove specific nodes on a diagram that already has many nodes, read `current_mermaid` first, then resend the FULL updated Mermaid via `generate --mermaid` — carrying every existing node forward unchanged. There is no diff/patch endpoint; you are the AI, so you make the edit (see Workflow 14 and Rule 14).

10. **End every session with a shareable link.** After completing a design session, call `vaxis diagrams share <rootDiagramId> --json` and give the user the link directly. They should never need to open the web app to find it. Share the ROOT diagram — one link covers the sub-diagrams it drills into. Never `--rotate` just to fetch a link; a plain `share` already returns the existing one, and rotating breaks links the user has handed out.

11. **Reuse context before fetching.** If diagram IDs or app IDs were established earlier in the conversation, use them directly. Only re-fetch with `apps list` or `diagrams list` when the context is genuinely unclear.

12. **One clarifying question, then proceed.** If the user's instruction is ambiguous, ask one focused question (which project? which diagram? what change?), then proceed without further interruption. Never ask two questions in a row.

13. **Confirm before destructive actions.** Before running `delete` on a diagram or application, always ask for confirmation and state what will be cascaded. After deletion, report exactly what was removed.

14. **Preserve existing nodes on every update.** When updating a diagram, read `current_mermaid` first and carry forward all existing nodes. Only modify what the user asked to change. No node should disappear from an update unless the user explicitly asked to remove it.

15. **Drill by default — it's the core feature.** When generating an architecture, never emit a flat single-level diagram. Structure it as a hierarchy: major subsystems at the root, a `%% vaxis:drill` on every composite subsystem, and no drills on leaf/atomic nodes (databases, caches, external services). A diagram that could have subsystems but has none is a missed use of Vaxis. See **"Drilling is Vaxis's core feature"** in the Drill syntax section for the composite-vs-leaf rule and a worked example.

16. **Honor the user's generation mode.** Before generating, read `vaxis config show --json` and check the `generation_mode` field. If it is `"prompt"`, generate via `--prompt` (Vaxis server AI). If it is `"mermaid"`, `null`, or unavailable, write the Mermaid yourself and use `--mermaid` (the default). See **"Generation mode"** near the top of this skill. Never switch modes on your own — the flag you pass must match the stored preference.

17. **Clarify before drawing from a thin description — ask with options, not free text.** When a diagram request lacks the detail needed for an accurate result (e.g. "draw my app" with no components, stack, or data flow named), gather what's missing **before** generating:
    - Ask a focused batch of clarifying questions (2–4) covering the key unknowns: the main components/services, what connects to what, datastores, and external services (Stripe, S3, auth provider…).
    - **Present them as structured multiple-choice questions with selectable options — not free text — whenever your environment allows it.** If your platform has a structured question/selection UI, use it (e.g., Claude Code's AskUserQuestion, specialized chat interfaces). If not, ask the same questions concisely in text.
    - Generate the diagram from the answers.

    Don't over-ask: if the request is already specific enough to draw something useful, draw it and offer to refine. One good round of option-based questions beats a vague diagram the user has to fix. (Consistent with Rule 12 — a focused round, never an endless back-and-forth.) This applies to the `--mermaid` path where you author the diagram; on `--prompt`, Vaxis's server AI handles the request directly.
