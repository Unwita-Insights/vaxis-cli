---
name: vaxis-diagram-to-code
description: Generate code scaffolding, API contracts, database schemas, and infrastructure definitions from an existing Vaxis architecture diagram.
---

# Vaxis Diagram-to-Code Skill

Reverse the code-to-diagram flow: given a Vaxis architecture diagram, generate code artifacts
that mirror the diagram's structure. Use this when a diagram already exists — either created
by WF19 (generate from code) or designed from scratch — and the user wants to scaffold
implementations from it.

The driving principle: **the diagram is the source of truth.** Generated code files are a
starting point, not a repeatable source. Never overwrite existing files without explicit
approval.

---

## When to load this skill

Load this skill when the user says something like:

- "Generate code from the diagram"
- "Scaffold the services from the architecture"
- "Create project structure from the Vaxis diagram"
- "Generate API contracts / OpenAPI / protobuf from the diagram"
- "Scaffold Docker Compose / IaC from the architecture"
- "Create database schema from the diagram"
- "Turn the diagram into code"
- "Implement the architecture"

Run `vaxis skills get diagram-to-code` to load this skill if it is not already in context.

---

## Prerequisites

```bash
vaxis me --json     # must return a user object, not {"error":"not_authenticated"}
```

If not authenticated, ask the user to run `vaxis login` before proceeding.

---

## Output types this skill can generate

| Output type | What is produced | Triggered by |
|---|---|---|
| **Project scaffold** | Folder structure + entry-point source files per service node | `service` / `gateway` nodes |
| **API contracts** | OpenAPI YAML stubs (REST) or `.proto` files (gRPC) from labeled edges | `service`→`service` edges with route labels |
| **Database schema** | SQL DDL / Prisma schema / ORM models | `storage` / cylinder nodes |
| **Infrastructure (IaC)** | `docker-compose.yml` + `.env.example` from full topology | Any diagram with ≥2 service nodes |
| **Message schemas** | JSON Schema or Avro definitions from async/event edges | Edges labeled `event`, `async`, `queue` |

---

## Core rules

### Rule 1 — Diagram is source of truth

The diagram drives all generation decisions. Do not invent components, connections, or
schemas that don't appear in the diagram. If a node has no description, derive context
from its label, shape, and edges.

Generated code is a **starting point** — stubs with correct structure and naming, not
full implementations. Say so clearly so users know to fill in business logic.

### Rule 2 — File safety

**Before writing any file that already exists at the target path:**

```
"⚠  [path] already exists. What should I do?"
options:
  - "Overwrite" — replace existing content
  - "Skip" — leave existing file untouched
  - "Rename new file to [path]-new" — keep both
```

Never create directories or files until the generation plan is approved (see count
recommendation section below).

### Rule 3 — Node-to-artifact mapping

Mermaid node shapes map to code artifacts:

| Shape | Mermaid syntax | What to generate |
|---|---|---|
| Rectangle (service) | `id["Label"]` | Service directory + language entry point |
| Cylinder (storage) | `id[("Label")]` | Schema file: SQL DDL, Prisma, or ORM model |
| Rhombus (decision/router) | `id{"Label?"}` | `if`/`else` branch or request router |
| Hexagon (external) | `id{{"Label"}}` | `.env.example` stub + typed client wrapper |

For `gateway` nodes (labeled "API Gateway", "Load Balancer", etc.): generate a reverse-proxy
config stub (`nginx.conf` or Traefik config), not application code.

### Rule 4 — Label-to-name translation (apply consistently)

Diagram node labels are Title Case. Translate them to code identifiers using these rules:

**Title Case label → code identifier:**
```
"Order Service"   → dir: order-service/   module: order_service   class: OrderService
"Auth System"     → dir: auth-system/      module: auth_system     class: AuthSystem
"User Repository" → dir: user-repository/  module: user_repository class: UserRepository
"API Gateway"     → dir: api-gateway/      module: api_gateway     file: api_gateway.conf
```

**Abbreviation mapping** (keep consistent with the code-to-diagram skill):
```
"Authentication" → auth/         (short form for directories and module names)
"Configuration"  → config/
"Repository"     → repo/ for directories, full "Repository" for class/type names
"Management"     → mgmt/ for directories, "Manager" for class names
"Processor"      → processor/    (don't abbreviate to proc/)
"Handler"        → handler/
"Message"        → message/      (don't abbreviate to msg/)
```

**Target-language naming conventions:**

| Language | Directory | Module / File | Class / Type |
|---|---|---|---|
| TypeScript / JS | `kebab-case/` | `camelCase.ts` | `PascalCase` |
| Python | `snake_case/` | `snake_case.py` | `PascalCase` |
| Go | `lowercase/` | `snake_case.go` | `PascalCase` |
| Rust | `snake_case/` | `snake_case.rs` | `PascalCase` |
| Java / Kotlin | `kebab-case/` | `PascalCase.java` | `PascalCase` |

Infer the target language from existing project files (`package.json`, `Cargo.toml`,
`go.mod`, `pyproject.toml`, `pom.xml`). If not determinable, ask before generating.

### Rule 5 — Edge labels → API contract hints

| Edge label | What to generate |
|---|---|
| `REST`, `HTTP`, `HTTPS` | OpenAPI YAML stub with placeholder paths, request/response schemas |
| `gRPC`, `protobuf` | `.proto` file with `service` + `rpc` definitions |
| `event`, `async`, `queue`, `stream` | Message schema (JSON Schema or Avro) + producer/consumer stubs |
| `SQL`, `DB`, unlabeled to storage | ORM query method stubs in the source service |
| Unlabeled (service→service) | Plain import / method call — no contract file |

Place contract files in the **source service's** directory (the service that initiates
the call), in a `contracts/` or `api/` subdirectory.

### Rule 6 — Generated file header comments

Every generated file MUST begin with a header comment derived from the node's description.
The description is the richest context available and should appear verbatim (or lightly
condensed) at the top of each generated file.

```
✗ Bad:  // OrderService
✓ Good: // Order Service — processes customer orders from placement through fulfillment;
//       owns the order lifecycle state machine and coordinates with Payment Service
//       and Inventory Service.
```

Source of the description:
1. **Primary**: `description` field from `vaxis diagrams show <id> --json` → `nodes[]`
2. **Fallback**: Derive from the node's label, shape, and immediate edges:
   - "Handles [label] operations" + list of connected services

Apply this rule to ALL generated files: service stubs, schema files, contract files,
IaC configs, and `.env.example`.

---

## Generation count recommendation

After reading the diagram but **before generating any file**, state the planned output
explicitly and ask for approval:

```
"Based on [diagram name], I plan to generate:
 • [N] service scaffolds: [ServiceA], [ServiceB], ...
 • [M] API contract files: [A → B (REST)], [B → C (gRPC)], ...
 • [K] schema files: [StorageNode1], [StorageNode2], ...
 • [J] IaC files: docker-compose.yml, .env.example
 Total: [X] files across [Y] directories."
```

Then use AskUserQuestion:

```
header: "Generation plan"
question: "I plan to generate [X] files as shown above. Proceed?"
options:
  - label: "Looks good — generate"
    description: "Create all planned files"
  - label: "Adjust the plan"
    description: "I'll describe what to add, remove, or rename before generating"
  - label: "Skip"
    description: "Abort; don't generate anything"
```

If "Adjust the plan": apply the user's changes, recount, re-state totals, then call
AskUserQuestion again. Loop until "Looks good" or "Skip".

For a hierarchical diagram (with drill children), also state the scope of sub-module
generation:
```
"Expanding [M] drill diagrams will produce ~[X] additional files at the sub-module level."
```

---

## Workflows

### WF-D2C0 — Read the diagram

Run before any generation workflow:

```bash
# 1. Read root diagram content and metadata
vaxis diagrams show <diagramId> --json
```

Parse the JSON response:
- `current_mermaid`: parse node IDs, labels, shapes, and edges
- `name`: use as the project/service name anchor

```bash
# 2. Discover child diagrams (drills)
vaxis diagrams tree <diagramId> --json
```

For each child diagram in the tree:
```bash
vaxis diagrams show <childId> --json
```

Build an in-memory map per node:
```
node → { id, label, shape, description, edges[], drill_children[] }
```

Use the node map to drive all subsequent generation steps.

---

### WF-D2C1 — Project scaffold

Trigger: user asks for service structure, project scaffold, or folder layout.

1. Run WF-D2C0 to read the diagram.
2. For each rectangle/gateway node in the root diagram, determine the target directory
   name using Rule 4.
3. Show the planned directory tree **before creating any file**:
   ```
   "I'll create the following structure:
    order-service/
      index.ts           (entry point)
      order-service.ts   (main module)
    auth-system/
      index.ts
      auth-system.ts
    ..."
   ```
4. Use AskUserQuestion (generation plan — see count recommendation section).
5. If approved: create directories and entry-point files. Each file gets a header comment
   (Rule 6) and a `// TODO: implement` placeholder body.
6. For drill children: if the user wants sub-module scaffolding, run WF-D2C5 to generate
   the sub-module level.

---

### WF-D2C2 — API contracts

Trigger: user asks for OpenAPI, protobuf, API stubs, or contracts.

1. Run WF-D2C0.
2. Collect all labeled edges between service/gateway nodes.
3. For each edge, determine the contract type (Rule 5).
4. Show the planned contract file list before generating.
5. Use AskUserQuestion (generation plan).
6. If approved: generate one contract file per labeled edge:
   - **REST**: `<source-service>/contracts/<target-service>.openapi.yaml` with placeholder
     `paths`, `components/schemas`, and example `requestBody`/`response`.
   - **gRPC**: `<source-service>/contracts/<target-service>.proto` with `syntax = "proto3"`,
     a `service` block, and one `rpc` stub per edge label suffix (if specified).
   - **Event**: `<source-service>/contracts/<target-service>.schema.json` (JSON Schema) or
     `.avro` (Avro), with a matching producer stub and consumer stub.

---

### WF-D2C3 — Database schema

Trigger: user asks for SQL, schema, database setup, ORM models, or migration files.

1. Run WF-D2C0.
2. Collect all cylinder nodes and edges that point to them.
3. Ask for the target format if not determinable from project files:
   ```
   header: "Schema format"
   question: "What database schema format should I generate?"
   options:
     - label: "SQL DDL"
     - label: "Prisma schema"
     - label: "SQLAlchemy models"
     - label: "GORM models"
   ```
4. Show the planned schema files before generating.
5. If approved: for each cylinder node, generate a schema file:
   - Table / model name = node label (Rule 4 class naming).
   - Columns inferred from edges (e.g., FK columns for service→storage edges).
   - Add placeholder columns for common fields: `id`, `created_at`, `updated_at`.
   - Include a header comment (Rule 6) explaining the storage node's purpose.

---

### WF-D2C4 — Infrastructure (IaC)

Trigger: user asks for Docker Compose, IaC, deployment config, or infrastructure setup.

1. Run WF-D2C0.
2. Build `docker-compose.yml`:
   - One `service` block per rectangle node.
   - `image: <label-kebab-case>:latest` as placeholder.
   - `ports` for gateway nodes (e.g., `8080:8080`).
   - One `volume` block per cylinder node.
   - `environment` block with references to `.env` variables for external node connections.
3. Build `.env.example`:
   - One variable per external (hexagon) node: `<LABEL_UPPER_SNAKE>=<description>`.
   - Include a comment line above each variable derived from the node's description (Rule 6).
4. Show planned files and use AskUserQuestion (generation plan).

---

### WF-D2C5 — Per-level generation (hierarchical diagrams)

Use when the diagram has drill children and the user wants sub-module code.

1. Generate root-level code first (root diagram nodes → top-level services).
2. After root level is complete, state the scope of the next level:
   ```
   "Root level done — [X] files created. For the sub-module level, I found [Y] drill
   diagrams; this will produce ~[Z] more files."
   ```
3. Use AskUserQuestion:
   ```
   header: "Sub-module level"
   question: "Ready to generate sub-module code? (~[Z] files)"
   options:
     - label: "Continue"
       description: "Generate code for all [Y] drill diagrams"
     - label: "Select which services to expand"
       description: "I'll list the drill targets — you choose which ones to scaffold"
     - label: "Stop here"
       description: "Keep the root-level code as-is; skip sub-modules"
   ```
4. If "Continue": for each drill child, run WF-D2C1 scoped to that child's nodes,
   placing generated files inside the parent service's directory.
5. Repeat for deeper levels until all drills are exhausted or the user stops.

---

### WF-D2C6 — Full generation (complete project from diagram)

Trigger: user asks to "generate everything" or "scaffold the full project."

1. Run WF-D2C0.
2. State the generation plan for ALL output types (count recommendation section).
3. Run in sequence, with a confirmation gate between each type:
   - WF-D2C1 (scaffold) → confirm → WF-D2C2 (contracts) → confirm → WF-D2C3 (schema)
     → confirm → WF-D2C4 (IaC)
4. For each type, ask:
   ```
   header: "Generate [type]?"
   question: "Proceed with generating [output type]?"
   options:
     - label: "Yes"
     - label: "Skip this type"
     - label: "Stop all generation"
   ```
5. Report a summary on completion:
   ```
   "Generated [total] files:
    • [N] service scaffolds
    • [M] API contract files
    • [K] schema files
    • docker-compose.yml, .env.example"
   ```

---

### WF-D2C7 — Selective generation

Trigger: user asks for a specific output type only (just OpenAPI, just schema, etc.)

1. If the output type is clear from the request, proceed directly to the relevant workflow
   (WF-D2C1 through WF-D2C4).
2. If the request is ambiguous ("generate the contracts" when there are multiple edge types):
   ```
   header: "Contract types"
   question: "Which contract types should I generate?"
   multiSelect: true
   options:
     - label: "OpenAPI (REST)"
     - label: "protobuf (gRPC)"
     - label: "Message schemas (events/async)"
   ```
3. Run only the selected types.

---

## Error handling

| Error | Response |
|---|---|
| `401` from any `vaxis` command | "Session expired. Run `vaxis login` then retry." |
| Diagram not found | "Diagram `[id]` not found. Run `vaxis diagrams list --json` to see available diagrams." |
| Empty `current_mermaid` | "This diagram has no Mermaid content yet. Generate diagram content first using `vaxis diagrams generate`." |
| Drill child has no content | Scaffold from the parent node's label only. Note the gap: "⚠ Drill child `[name]` is empty — scaffolding from parent node label only." |
| No labeled edges for contracts | "No labeled edges found. Add protocol labels (REST, gRPC, event) to edges in the diagram before generating contracts." |
| No cylinder nodes for schema | "No storage nodes found in this diagram. Cylinder-shaped nodes represent databases and data stores." |

---

## Diagram navigation commands (reference)

```bash
vaxis diagrams list --json                     # list all diagrams in all apps
vaxis diagrams show <id> --json                # read diagram content + metadata
vaxis diagrams tree <id> --json                # show full drill hierarchy
vaxis apps list --json                         # list apps (each app contains diagrams)
vaxis diagrams list <appId> --json             # list diagrams within a specific app
```

Prefer `--json` output in all commands so field values can be parsed exactly.
