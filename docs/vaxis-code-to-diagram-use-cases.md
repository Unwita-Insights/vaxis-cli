# Vaxis — Code to Diagram Use Cases

This document covers the **Code to Diagram** feature category: scenarios where the user is
working inside a real project directory, the Vaxis skill reads the codebase through the AI
agent's file system access, and uses the CLI to create or update diagrams from the code.

## How it works (architectural context)

The AI agent (Claude Code, Codex, etc.) has file system tools available. The Vaxis skill
instructs the agent to:
1. Explore the project directory to discover the structure
2. Read relevant source files, config files, and manifests
3. Synthesize what it finds into Mermaid
4. Use `vaxis` CLI commands to create or update the diagram

The CLI itself never reads files — all code reading happens in the agent. The CLI receives
the resulting Mermaid and handles persistence, drill creation, and sharing.

---

## Category 1 — Project Discovery & Initial Analysis

### UC-C01 · Analyze the entire project and generate an architecture diagram
**Scenario:** User says "analyze this project and create an architecture diagram."
**Skill behavior:**
1. Explore the root directory: look for `package.json`, `Cargo.toml`, `pom.xml`, `go.mod`, `pyproject.toml`, `Dockerfile`, `docker-compose.yml`, `*.sln`, etc. to identify the stack and structure
2. Read key entry points (`main.rs`, `index.ts`, `app.py`, `cmd/`, etc.)
3. Identify major services, modules, packages, and their connections
4. Generate a high-level architecture flowchart with drills on composite modules
5. Create a Vaxis project and diagram; push Mermaid via `vaxis diagrams generate --mermaid`

**Expected behavior:** A top-level architecture diagram reflecting the real project structure — services, modules, datastores, external APIs — with drill children for each major component.
**Edge cases:**
- No recognizable manifest file → ask user "What kind of project is this?" before reading further
- Multiple languages in one repo → include all; indicate the language in node labels if relevant
- Very small project (1–3 files) → generate a simple flat diagram, no drills needed
- Project has both a frontend and backend → two subgraphs or two drill children

---

### UC-C02 · Auto-detect project type and choose the right diagram shape
**Scenario:** Agent opens the project and must infer what kind of diagram is most appropriate without being told.
**Skill behavior:**
- REST API project → flowchart TB showing routes, middleware, handlers, database
- Event-driven / message queue system → flowchart LR showing producers, queues, consumers
- Library or SDK → class diagram or dependency diagram
- Frontend SPA → component hierarchy + data flow (state management, API calls)
- Database-only or migration project → ER diagram from schema files
- Microservices monorepo → top-level architecture with one drill per service

**Expected behavior:** Skill picks the most appropriate diagram type before generating. If ambiguous, asks user once ("This looks like a REST API — shall I draw the service architecture or the request flow?").
**Edge cases:**
- Project could fit multiple types → ask user to choose; present 2–3 options
- Type detection confident but user wants a different type → honor the request

---

### UC-C03 · User specifies the project directory explicitly
**Scenario:** User says "analyze the `apps/api` directory."
**Skill behavior:** Scope all file reading to `apps/api/`. Treat it as the root of the analysis.
**Expected behavior:** Diagram reflects only the code in that directory — not the entire monorepo.
**Edge cases:**
- `apps/api` imports from shared libraries outside its directory → include those imports as labeled external dependencies (rectangles), don't recurse into them unless asked
- Directory doesn't exist → report clearly and ask for the correct path

---

### UC-C04 · User specifies individual files to analyze
**Scenario:** User says "generate a diagram from `src/auth/middleware.ts` and `src/auth/session.ts`."
**Skill behavior:** Read only those files. Infer relationships between them and any modules they import.
**Expected behavior:** Narrow diagram showing only what those files expose and how they relate.
**Edge cases:**
- Two files with no relationship to each other → diagram two unconnected subgraphs; ask if that's correct
- Files import heavily from a third file not listed → include the third file as an external dependency node, don't recurse unless asked

---

### UC-C05 · Skill identifies the project but cannot read key files (permissions, binary)
**Scenario:** Agent tries to read a compiled artifact, a binary, or a file it can't decode.
**Skill behavior:** Skip the unreadable file, note it was skipped, continue with what's readable.
**Expected behavior:** Diagram generated from readable files; a note shown: "Skipped `build/output.wasm` (binary)."
**Edge cases:** If the only relevant files are unreadable → tell user "I couldn't read the core files. Can you point me to the source directory?"

---

## Category 2 — High-Level Architecture Diagrams

### UC-C06 · Generate a high-level architecture diagram from the full codebase
**Scenario:** User wants a bird's-eye view — major services, their roles, and how they connect.
**Skill behavior:**
1. Read manifest files and entry points only (not individual implementation files)
2. Identify: web server, background workers, scheduled jobs, databases, caches, external APIs called
3. Generate root diagram with subgraphs and `%% vaxis:drill` for each major service
4. Offer to drill into each service in follow-up turns

**Expected behavior:** Clean top-level diagram (~8–15 nodes), properly grouped, with drill markers on composite services. No implementation detail at this level.
**Edge cases:**
- Project has 20+ services → top-level diagram shows major domains (bounded contexts), not individual services. Individual services become drill children
- Services don't have clear boundaries → ask user "How do you group these services?" before generating

---

### UC-C07 · Generate an architecture diagram for a specific service within a monorepo
**Scenario:** User says "diagram the payment service" in a monorepo with 12 services.
**Skill behavior:** Navigate to the `services/payment/` (or equivalent) directory, analyze only that service's code.
**Expected behavior:** Diagram scoped to the payment service: its internal components, its database, its external API calls (Stripe, etc.), its queue consumers.
**Edge cases:**
- Service shares a database with other services → show the database as a shared node labeled "Shared DB" with a note it's not exclusive
- Service name is ambiguous (two services partially match "payment") → show both matches and ask which

---

### UC-C08 · Generate a dependency diagram (module/package relationships)
**Scenario:** User says "show me how the modules in this project depend on each other."
**Skill behavior:**
1. Read all import/require statements across source files
2. Map module-to-module dependencies (not function-level)
3. Generate a flowchart where each node is a module and edges are import relationships
4. Highlight circular dependencies if found

**Expected behavior:** Dependency graph with direction of dependency. Circular dependencies clearly marked (edge labeled "circular").
**Edge cases:**
- Hundreds of small modules → group by directory/package; show package-level dependencies, not file-level
- External npm/pip/cargo packages → show as a separate "External" subgraph or leaf nodes
- Circular dependency detected → warn user: "Found circular dependency: auth → session → auth. This may cause issues."

---

## Category 3 — API & Service Flow Diagrams

### UC-C09 · Generate an API flow diagram from route definitions
**Scenario:** User says "generate a diagram of the API routes and how requests flow through the system."
**Skill behavior:**
1. Find route definitions (`router.get(...)`, `@GetMapping(...)`, `app.route(...)`, etc.)
2. Trace each route through middleware → controller → service → repository → database
3. Generate a flowchart for the request lifecycle

**Expected behavior:** Sequence-like flowchart (LR) showing the main request path per key route. Group routes into subgraphs by resource (e.g., `/users`, `/payments`).
**Edge cases:**
- Too many routes to show individually → group by resource type; drill into each resource group
- Middleware chain is complex → show as a single "Middleware Stack" node unless user asks for details
- Routes defined dynamically at runtime → note: "Dynamic routes detected — showing only statically defined routes"

---

### UC-C10 · Generate a sequence diagram from a specific code flow
**Scenario:** User says "draw the sequence diagram for the login flow."
**Skill behavior:**
1. Find the login entry point (route or function)
2. Trace the execution call stack: controller → auth service → user repository → JWT issuer → session store
3. Generate a `sequenceDiagram` with each participant being a real module/class

**Expected behavior:** Sequence diagram with meaningful participant names (matching actual class/module names from the code), showing the call order, return values, and conditional paths.
**Edge cases:**
- Flow has async branches → show as `alt`/`opt` blocks in the sequence diagram
- Flow is too deep (10+ levels) → trace to service boundary; mark "internal detail omitted" for very deep recursion
- Login flow doesn't exist by that name → ask user "What's the entry point? I see `POST /auth/login` and `POST /session/create` — which one?"

---

### UC-C11 · Generate an API diagram showing how two specific modules communicate
**Scenario:** User says "show me how the payment module and the inventory module communicate."
**Skill behavior:**
1. Find both modules in the codebase
2. Read their interfaces, exported functions, and any shared types
3. Find all call sites where one module calls the other (direct calls, events, queues)
4. Generate a focused diagram of only these two modules and their interactions

**Expected behavior:** Small, precise diagram showing: what the payment module calls on inventory, what data passes between them, and which direction the dependency goes.
**Edge cases:**
- Modules communicate via a message queue → show the queue as an intermediary node
- Modules don't communicate directly (only through a shared database) → show that pattern accurately
- No communication found → report: "I couldn't find direct communication between these two modules. They may only share data through the database."

---

### UC-C12 · Generate a diagram for a specific feature end-to-end
**Scenario:** User says "diagram the checkout feature from UI click to database write."
**Skill behavior:**
1. Find frontend component for checkout (button click handler)
2. Trace the API call to the backend route
3. Follow the backend flow through middleware, service, payment provider call, database write
4. Generate a top-down flowchart or sequence diagram covering the full path

**Expected behavior:** End-to-end feature flow with clear layer separation (Frontend → API → Services → External → Data).
**Edge cases:**
- Frontend and backend are in different repos → diagram what's available; label missing parts as "External: Frontend" or "External: Backend"
- Feature has multiple sub-flows (success path, error path, retry) → ask user which path to diagram first, or generate with `alt` blocks

---

## Category 4 — Data & Database Diagrams

### UC-C13 · Generate a database schema diagram from model definitions
**Scenario:** User says "generate an ER diagram from the database models."
**Skill behavior:**
1. Find ORM models / schema definitions (Prisma schema, TypeORM entities, SQLAlchemy models, Diesel schema, ActiveRecord models, etc.)
2. Identify entities, their fields, and relationships (hasMany, belongsTo, etc.)
3. Generate an `erDiagram` with cardinality

**Expected behavior:** ER diagram with real entity names from the code, correct cardinality notation, and relationship labels.
**Edge cases:**
- No ORM found → look for SQL migration files (`*.sql`, `migrations/`) and parse CREATE TABLE statements
- Schema defined across multiple files → merge all into one ER diagram
- Many-to-many relationships with a join table → show the join table as an entity
- Large schema (30+ tables) → ask user "This schema has 35 tables. Shall I diagram all of them or focus on a specific domain?"

---

### UC-C14 · Generate a data-flow diagram based on the code
**Scenario:** User says "show me how data flows through the system — from input to storage."
**Skill behavior:**
1. Identify data entry points (API endpoints, file uploads, event listeners, webhooks)
2. Trace transformations: validation → normalization → business logic → persistence
3. Generate a flowchart showing data movement and transformation stages

**Expected behavior:** Left-to-right flowchart showing data sources → transformations → sinks. Use cylinders for storage nodes.
**Edge cases:**
- Data fans out to multiple sinks (DB + cache + event queue) → show all branches
- Transformation logic is complex → summarize as a labeled node ("Pricing Calculator"), offer to drill in
- Data enters through multiple channels → one subgraph per channel, merge at the processing layer

---

### UC-C15 · Generate a diagram showing which services own which databases
**Scenario:** User says "which service owns which database? Are any databases shared?"
**Skill behavior:**
1. Find all database connection strings / config references across services
2. Map service → database relationships
3. Generate a diagram where cylinders are databases and rectangles are services

**Expected behavior:** Ownership map. Shared databases have multiple incoming edges and are clearly labeled "Shared."
**Edge cases:**
- Connection strings are in environment variables → read `.env.example` or config templates; note: "Connection details read from env templates — verify against actual environment"
- Database names not deterministic from code alone → show database type (PostgreSQL, Redis) + service name as the identifier

---

## Category 5 — Updating Diagrams from Code Changes

### UC-C16 · Update an existing diagram after code changes
**Scenario:** User says "I've added a new notification service — update the diagram."
**Skill behavior:**
1. `vaxis diagrams show <diagramId> --json` → read `current_mermaid`
2. Read the new `services/notifications/` directory
3. Identify what the new service is, what it connects to, and whether it needs a drill
4. Add the new node + edges to the existing Mermaid; carry all existing nodes unchanged
5. `vaxis diagrams generate <diagramId> --mermaid "<updated>" --json`

**Expected behavior:** Diagram updated with the new service; all prior nodes intact.
**Edge cases:**
- New service already partially visible in old diagram (placeholder node) → update that node's label/connections rather than adding a duplicate
- New service replaces an old one → ask user: "I see you added `notificationServiceV2` — should I remove `notificationService` from the diagram?"

---

### UC-C17 · Compare the existing diagram with the current code and identify drift
**Scenario:** User says "the diagram might be out of date — check what's changed in the code."
**Skill behavior:**
1. `vaxis diagrams show <diagramId> --json` → read `current_mermaid`, extract all node IDs and labels
2. Analyze the current codebase to re-derive the architecture
3. Compare: find nodes in the diagram that no longer exist in code; find new components in code not in diagram
4. Report the drift in plain language before making any changes

**Expected behavior:** Plain-language diff report: "The diagram has `authService` but I don't see that module in the code anymore. The code has `oauthProvider` which isn't in the diagram. Want me to update the diagram?"
**Edge cases:**
- Diagram was intentionally simplified (omitting low-level details) → some code components legitimately not in diagram; don't flag everything
- Rename detected (old name in diagram, new name in code) → propose as rename, not delete + add
- No drift found → "The diagram matches the current code."

---

### UC-C18 · Add newly detected components to an existing diagram
**Scenario:** User says "add anything new you find in the code to the diagram."
**Skill behavior:**
1. Read the existing diagram's `current_mermaid`
2. Analyze the codebase for components not represented in the diagram
3. For each new component: determine its type (service, database, external API), its connections to existing nodes
4. Generate updated Mermaid adding new nodes + edges; preserve all existing content
5. Push via `generate --mermaid`

**Expected behavior:** Diagram extended with new components. Report: "Added `emailService` (connects to `api` and `notificationQueue`) and `emailQueue [(Redis)]`."
**Edge cases:**
- Many new components detected → ask user which to add first; don't dump 15 new nodes at once
- New component's connections are unclear from code alone → add the node but ask user "I found `webhookHandler` but I'm not sure what calls it — shall I connect it to the API Gateway?"

---

### UC-C19 · Remove or flag diagram components that no longer exist in the code
**Scenario:** User says "clean up the diagram — remove anything that's been deleted from the codebase."
**Skill behavior:**
1. Read the existing diagram
2. For each node in the diagram, check whether a corresponding module/service/component exists in the code
3. Identify stale nodes; confirm with user before removing
4. Generate updated Mermaid with stale nodes removed (and their edges)

**Expected behavior:** Confirmation step: "I found 3 components in the diagram that don't appear in the code: `legacyAuth`, `oldPaymentGateway`, `v1ApiRouter`. Remove all three?"
**Edge cases:**
- Component was renamed (not deleted) → detect via fuzzy match and propose rename instead of removal
- Component exists in a branch not yet merged → note uncertainty: "I don't see `featureX` in `main` — it may be in an active branch"
- Stale node was intentional (placeholder for future work) → if user confirms it's intentional, keep it

---

### UC-C20 · Detect that a diagram and code are fully in sync
**Scenario:** User asks "is the diagram up to date?" and it actually is.
**Skill behavior:** Run the comparison (UC-C17 flow) and find no drift.
**Expected behavior:** "The diagram looks up to date — all components in the diagram match the current code."
**Edge cases:** Confidence level qualifier: "I analyzed the main source files. I may have missed dynamically loaded modules or runtime-generated components."

---

## Category 6 — Scoped & Feature-Specific Analysis

### UC-C21 · Generate a diagram for a specific feature by name
**Scenario:** User says "diagram the 'forgot password' feature."
**Skill behavior:**
1. Search the codebase for the feature entry point (route, controller, function name containing "forgot" or "reset")
2. Trace the full flow: request → validation → token generation → email dispatch → token storage → verify endpoint → password update
3. Generate a sequence diagram or flowchart for this specific flow

**Expected behavior:** Precise feature diagram; not the whole system. Node labels match real function/class names from the code.
**Edge cases:**
- Feature spread across 4 services → still trace end-to-end; label each service boundary clearly
- Feature name ambiguous (multiple matches) → list matches and ask which: "I found `forgotPasswordController`, `passwordResetService`, and `resetTokenJob` — which flow do you want?"

---

### UC-C22 · Generate a diagram scoped to a specific module
**Scenario:** User says "diagram the auth module internals."
**Skill behavior:**
1. Navigate to `src/auth/` (or equivalent)
2. Read all files in that directory
3. Map: exported functions/classes, internal dependencies between files, external dependencies called
4. Generate a detailed flowchart of the module's internal structure

**Expected behavior:** Diagram shows internal components of the auth module (Login, Logout, Token Manager, Session Store, etc.) and their relationships. External dependencies shown as leaf nodes at the boundary.
**Edge cases:**
- Module is a single large file → extract function-level components as nodes
- Module imports from 10+ external packages → group external packages into a single "Dependencies" subgraph

---

### UC-C23 · Generate a diagram from selected files only
**Scenario:** User says "generate a diagram from just `src/routes/api.ts` and `src/services/userService.ts`."
**Skill behavior:** Read only the two specified files. Map their exports, imports, and call relationships.
**Expected behavior:** Focused diagram of just those two files and their relationship.
**Edge cases:**
- One file has no relationship to the other → two separate subgraphs; ask "These files don't seem to interact. Is that expected?"

---

### UC-C24 · Trace a code flow from a specific function or entry point
**Scenario:** User says "trace what happens when `processPayment()` is called."
**Skill behavior:**
1. Find `processPayment` in the codebase
2. Trace its call chain: what it calls, what those call, up to 3–4 levels deep
3. Identify any async operations, error paths, external calls (Stripe API, database writes)
4. Generate a flowchart following the execution path

**Expected behavior:** Execution-path flowchart starting from `processPayment`, branching at error conditions, showing all downstream calls.
**Edge cases:**
- Function is overloaded or has multiple implementations → ask which one
- Call chain is very deep (10+ levels) → cut at service boundaries; offer to drill into each service separately
- Function doesn't exist by that exact name → fuzzy-match and show candidates

---

## Category 7 — Monorepo & Multi-Service Projects

### UC-C25 · Analyze a monorepo and generate a service topology diagram
**Scenario:** User says "diagram this monorepo — show all services and how they relate."
**Skill behavior:**
1. Detect monorepo structure: look for `apps/`, `packages/`, `services/`, `libs/`, Nx/Turborepo/Lerna config
2. Enumerate all apps/services
3. For each service: identify what it exposes (HTTP port, message queue topic, gRPC service)
4. For each service: identify what it calls (other services, external APIs, databases)
5. Generate a top-level topology diagram with one node per service; drills into each service

**Expected behavior:** Clean service topology with inter-service connections (labeled with protocol: HTTP, gRPC, Kafka, etc.). One drill child per service offering to detail its internals.
**Edge cases:**
- 20+ services → group by domain/team first; drill per domain, then per service within domain
- Services communicate via a shared message bus → show the bus as a central cylinder node
- Services only documented in README, not wired in code → include but label as "documented only — not verified in code"

---

### UC-C26 · Analyze a polyglot monorepo (multiple languages)
**Scenario:** Node.js frontend, Rust backend, Python ML service — all in one repo.
**Skill behavior:** Detect each service's language via its manifest. Analyze each with the appropriate parsing strategy.
**Expected behavior:** Unified architecture diagram where service nodes are labeled with their language (e.g., `api["API Gateway (Rust)"]`). Cross-language communication shown via HTTP/queue edges.
**Edge cases:**
- ML service exposes a non-standard interface (gRPC, ZeroMQ) → represent the interface accurately; note if it can't be determined from code alone
- One language is unknown → read source files to infer, or ask user

---

### UC-C27 · Generate a diagram for a specific service within a monorepo, then offer to diagram others
**Scenario:** User says "diagram the `billing` service." Monorepo has 8 other services.
**Skill behavior:** Scope analysis to the billing service. After completing it, report: "Billing service diagram created. Do you want me to continue with the other services?"
**Expected behavior:** Billing service fully documented. Other services listed but not diagrammed unless user asks.
**Edge cases:**
- Billing service has a hard dependency on the `payments` service → include `payments` as an external node in the billing diagram; offer to drill into it separately

---

### UC-C28 · Handle a project with multiple independent applications (not a monorepo)
**Scenario:** Repo has `frontend/`, `backend/`, `mobile/` as sibling directories, each being an independent app.
**Skill behavior:** Treat the whole directory as a multi-app project. Generate a top-level diagram where each app is a major node, with connections between them (frontend calls backend API, mobile calls the same API).
**Expected behavior:** Top-level diagram with one node per app, drill children showing each app's internals.
**Edge cases:**
- Apps don't share code and don't call each other → separate, disconnected subgraphs per app

---

## Category 8 — Large Codebase Handling

### UC-C29 · Handle a large repository that cannot be read in its entirety
**Scenario:** Repo has 500+ files across 40 directories.
**Skill behavior:**
1. Start from the top-level manifest and entry points only
2. Identify the most important directories based on naming conventions (`src/`, `lib/`, `services/`, `api/`, `core/`)
3. Read only the top-level structure of each identified directory (not every file)
4. Generate a high-level diagram from manifest + entry points only; offer to drill into specific areas
5. Report: "This is a large codebase. I've analyzed the top-level structure. Want me to dive deeper into any specific service?"

**Expected behavior:** High-level diagram from structural analysis only. No deep file reading until the user narrows scope.
**Edge cases:**
- Entry points are unclear → look for CI/CD configs (`Dockerfile`, `docker-compose.yml`, `k8s/*.yaml`) to infer what runs
- Manifest missing → ask user "What's the main entry point for this project?"

---

### UC-C30 · Chunked analysis — user narrows scope progressively
**Scenario:** User says "start with just the API layer, then we'll add the data layer."
**Skill behavior:** Analyze and diagram the API layer first. In a follow-up, analyze the data layer and update the diagram (UC-C16).
**Expected behavior:** Incremental diagram building. Each pass adds a new layer without breaking what's already there.
**Edge cases:**
- Layers are not clearly separated in the code → ask for help identifying the boundary: "I see API and data code mixed in the same files. Shall I separate them logically?"

---

### UC-C31 · Summarize a large service for the top-level diagram without full detail
**Scenario:** One service has 80+ files. The top-level diagram should show it as a single node with a drill.
**Skill behavior:** Read only the service's public interface (`index.ts`, `lib.rs`, `__init__.py`, etc.) to understand what it exposes. Use that for the top-level node. Create an empty drill child for the detail level.
**Expected behavior:** Top-level: one rectangle node for the service with a drill marker. Child diagram: filled with detail when the user explicitly asks for it.
**Edge cases:**
- No public interface file → read a sample of files and summarize what the service does based on function/class names

---

## Category 9 — Unknown & Unsupported Structures

### UC-C32 · Unknown project structure — no recognizable manifest
**Scenario:** The project has no `package.json`, `Cargo.toml`, `pom.xml`, or any other known manifest.
**Skill behavior:**
1. Look for `*.sh`, `Makefile`, `*.yaml`, `README.md` for clues
2. List the top-level directories and present them to the user
3. Ask: "I couldn't detect the project type. I see these directories: `core/`, `gateway/`, `workers/`. Which should I start with, and what kind of project is this?"

**Expected behavior:** Skill does not guess blindly. One focused clarifying question, then proceeds.
**Edge cases:**
- README describes the architecture in prose → read the README and generate a diagram from the description (like a text-to-diagram flow, not code analysis)

---

### UC-C33 · Project uses a framework the skill doesn't know
**Scenario:** Project uses a niche framework with unusual conventions.
**Skill behavior:** Fall back to generic code analysis — read source files, identify function/class names, look for HTTP handler patterns, database connection code, and inter-file imports.
**Expected behavior:** Generic architecture diagram based on structural analysis, not framework-specific conventions. Label it: "Generated from structural analysis — framework-specific details may be simplified."
**Edge cases:**
- Framework is entirely configuration-driven (no source code) → read the configuration files and generate a diagram from the config structure

---

### UC-C34 · Highly dynamic architecture (runtime-configured services)
**Scenario:** Services are registered dynamically; which services exist is only known at runtime.
**Skill behavior:** Diagram what's statically visible in code. Clearly label dynamically registered components as "Dynamic — detected at runtime."
**Expected behavior:** Diagram shows static structure + a "Dynamic Services" box representing the runtime-discovered components.
**Edge cases:**
- User knows the dynamic services → ask them to list them so they can be added explicitly

---

## Category 10 — Safety, Secrets & File Exclusions

### UC-C35 · Skip files containing secrets or credentials
**Scenario:** Project has `.env` files, `secrets.yaml`, `credentials.json`, or similar.
**Skill behavior:** Never read files whose names or paths indicate secrets (`.env`, `*secret*`, `*credential*`, `*private*`, `*key*`, `.pem`, `.p12`, `*password*`). Exclude silently.
**Expected behavior:** Diagram generated without reading secret files. No credentials appear in node labels or edges.
**Edge cases:**
- Secret file is the ONLY place where a database connection is defined → diagram the database node using its role ("PostgreSQL") not its connection string. Note: "Database connection details read from environment — not included in diagram."

---

### UC-C36 · Respect .gitignore and generated/build artifacts
**Scenario:** Project has `dist/`, `build/`, `node_modules/`, `target/`, `.next/`, `__pycache__/` etc.
**Skill behavior:** Never read files or directories that are typically generated or ignored: `node_modules/`, `dist/`, `build/`, `target/release/`, `__pycache__/`, `.next/`, `vendor/`, `.git/`.
**Expected behavior:** Diagram generated from source code only, not compiled artifacts.
**Edge cases:**
- User explicitly asks to analyze `dist/` → explain why that's not advisable; offer to analyze source instead

---

### UC-C37 · Skip test files and fixtures
**Scenario:** Project has many test files (`*.test.ts`, `*_spec.rb`, `tests/`) that don't represent production architecture.
**Skill behavior:** Exclude test files from the architecture analysis by default. Test fixtures should not become nodes in the production diagram.
**Expected behavior:** Architecture reflects production code only. Test infrastructure noted as "Tests not included."
**Edge cases (future):** If user says "show me the test infrastructure" → include test files and generate a test-dependency diagram.

---

### UC-C38 · User asks to exclude specific directories from analysis
**Scenario:** User says "analyze this project but ignore the `legacy/` directory."
**Skill behavior:** Exclude the specified directory from all file reads during analysis.
**Expected behavior:** Diagram reflects only the non-excluded code. Note: "Excluded: `legacy/` per your request."
**Edge cases:**
- Excluded directory is imported by included code → show as an external dependency node labeled "Legacy Module (excluded from analysis)"

---

## Category 11 — Clarification Flows

### UC-C39 · Codebase doesn't provide enough context — ask user for clarification
**Scenario:** Agent reads the code but can't determine how two services communicate (no imports, no shared config, may use service discovery).
**Skill behavior:** Generate the diagram with what's known, and highlight the unknown connection: "I see `OrderService` and `InventoryService` but I can't tell how they communicate from the code. Do they call each other directly, or via a message queue?"
**Expected behavior:** Diagram created with the unknown relationship shown as a dashed or labeled "?" edge. User answers → diagram updated.
**Edge cases:**
- Multiple unknowns → ask about the most critical one first; don't ask 5 questions at once

---

### UC-C40 · Multiple valid interpretations of the architecture
**Scenario:** Code could be read as either a layered MVC architecture or a hexagonal (ports & adapters) architecture.
**Skill behavior:** Choose the interpretation that most clearly shows the system's purpose. If genuinely ambiguous, ask: "I can diagram this as a layered architecture or a ports-and-adapters model. Which would be more useful?"
**Expected behavior:** One focused question; AI picks the most sensible default and offers to switch.
**Edge cases:**
- User doesn't know the difference → AI briefly explains and picks the simpler one

---

### UC-C41 · Feature name doesn't match code naming
**Scenario:** User says "diagram the 'checkout flow'" but the code uses `OrderController`, `CartService`, `PurchaseRepository`.
**Skill behavior:** Try to match "checkout" semantically to the code: look for cart, order, purchase, payment-related identifiers. Confirm with user: "I think 'checkout' maps to `CartService` → `OrderController` → `PurchaseRepository`. Does that sound right?"
**Expected behavior:** Correct diagram created after user confirms the mapping. Not a dead end.
**Edge cases:**
- No semantic match found at all → ask user: "I couldn't find anything matching 'checkout flow'. What's the entry point or filename?"

---

### UC-C42 · User says "update the diagram" but code and diagram have diverged significantly
**Scenario:** The diagram is 6 months old and the codebase has been heavily refactored. 40% of nodes no longer exist.
**Skill behavior:**
1. Run a full drift analysis (UC-C17)
2. Report the scale of the change: "The diagram has 12 components that no longer exist in the code, and I found 8 new components. This is a significant drift — shall I regenerate the diagram from scratch, or merge the changes incrementally?"
3. Wait for user decision

**Expected behavior:** User chooses: "regenerate from scratch" → treat as UC-C01; "merge" → apply changes as UC-C18 + UC-C19.
**Edge cases:**
- User says "keep anything that still exists" → carefully preserve valid nodes, remove stale, add new

---

## Category 12 — Continuous & Workflow Integration

### UC-C43 · "Keep the diagram up to date as I code" — periodic sync
**Scenario:** User wants the diagram to automatically reflect code changes after each significant update.
**Skill behavior (future):** Not currently supported. Would require watching for file changes (watch mode) or a git hook.
**Expected behavior (desired):** `vaxis sync --watch` → monitors file changes; re-runs analysis on change; updates diagram.
**Edge cases:** Debounce: don't re-analyze on every keystroke; trigger after a pause in activity. Only re-generate if the structural change is meaningful (a new function inside an existing class ≠ trigger; a new file in `services/` = trigger).

---

### UC-C44 · Generate a diagram as part of a CI/CD pipeline
**Scenario:** Every PR generates or updates the architecture diagram automatically.
**Skill behavior (current):** CI script calls `vaxis diagrams generate --json` with AI-generated Mermaid. The AI step is outside CI (run by Claude Code pre-commit or in a separate job).
**Expected behavior (future):** A dedicated CLI mode: `vaxis analyze --source ./src --app <appId> --diagram <diagramId>` that runs code analysis + Mermaid generation server-side.
**Edge cases:** Secrets must never appear in CI logs. The CLI must not read `.env` files in CI mode.

---

### UC-C45 · Compare diagrams across git branches to show architectural differences
**Scenario:** User says "show me what changed architecturally between `main` and `feature/new-auth`."
**Skill behavior:** Agent checks out (or reads) both branches, runs analysis on each, generates two diagrams, diffs them structurally, and presents the differences.
**Expected behavior (future):** Two diagrams created in Vaxis (one per branch). A diff summary: "Branch `feature/new-auth` adds `oauthProvider`, removes `legacyAuth`, and adds a new edge from `api` to `sessionStore`."
**Edge cases:** Shared code changes that don't affect architecture → ignored. Only structural (node/edge) changes reported.

---

## Category 13 — Incremental Diagram Building from Code

### UC-C46 · Start with a skeleton and fill in details progressively
**Scenario:** User says "give me a rough first draft, then we'll refine."
**Skill behavior:**
1. First pass: read only manifests and entry points → generate skeleton diagram with the major nodes and minimal edges
2. Second pass (on user request): read implementation files for each service → fill in connections and data stores
3. Third pass: add drill children for each composite service

**Expected behavior:** Iterative refinement. User sees value after step 1, not just at the end.
**Edge cases:** Skeleton diagram should still follow all Mermaid authoring rules — correct shapes, no fan-out violations

---

### UC-C47 · Generate diagram, then ask the user to validate it against reality
**Scenario:** AI generates a diagram from code. Some connections may be wrong (dynamic dispatch, runtime config).
**Skill behavior:** After generating, add a validation prompt: "I've generated this from the code. Does this match how the system actually works? Let me know if I missed any connections or if any are wrong."
**Expected behavior:** User reviews and says "add a connection from `api` to `analyticsService`" or "the `db` is actually PostgreSQL not MySQL." AI updates accordingly.
**Edge cases:** This is standard practice — code analysis cannot capture runtime behavior perfectly. Always offer validation.

---

### UC-C48 · Generate drill diagrams for each detected service automatically
**Scenario:** User says "diagram everything — root and all service internals."
**Skill behavior:**
1. Generate the root architecture diagram with `%% vaxis:drill` markers for each service
2. For each drill child created: immediately read that service's code and generate its detailed diagram
3. Continue until all services have content

**Expected behavior:** Full diagram tree populated from a single user request. User ends up with a root + N child diagrams all containing content.
**Edge cases:**
- One service is huge (hundreds of files) → generate a summary-level child diagram for it and offer to go deeper
- 10+ services → ask user: "There are 12 services. Shall I diagram all of them, or start with the most critical ones?"

---

## Summary — Code to Diagram Coverage

| Sub-category | Use Cases | Notes |
|---|---|---|
| Project Discovery & Initial Analysis | UC-C01 to UC-C05 | Auto-detect stack, scope, unreadable files |
| High-Level Architecture | UC-C06 to UC-C08 | Service topology, dependency graphs |
| API & Service Flow | UC-C09 to UC-C12 | Routes, sequences, module communication, features |
| Data & Database | UC-C13 to UC-C15 | ER diagrams, data flow, DB ownership |
| Updating from Code Changes | UC-C16 to UC-C20 | Sync, drift detection, add/remove components |
| Scoped & Feature Analysis | UC-C21 to UC-C24 | Feature, module, file, function tracing |
| Monorepo & Multi-Service | UC-C25 to UC-C28 | Monorepos, polyglot, multi-app |
| Large Codebase Handling | UC-C29 to UC-C31 | Chunked analysis, progressive zoom-in |
| Unknown & Unsupported Structures | UC-C32 to UC-C34 | Unknown frameworks, dynamic architectures |
| Safety & File Exclusions | UC-C35 to UC-C38 | Secrets, build artifacts, test files, user exclusions |
| Clarification Flows | UC-C39 to UC-C42 | Unknown connections, naming mismatch, heavy drift |
| CI/CD & Workflow Integration | UC-C43 to UC-C45 | Watch mode, CI pipeline, branch comparison |
| Incremental Building | UC-C46 to UC-C48 | Skeleton → detail, user validation, full-tree generation |

**Total Code-to-Diagram use cases: 48** (UC-C01 to UC-C48)

Combined with the main catalog (UC-01 to UC-103): **151 use cases total.**
