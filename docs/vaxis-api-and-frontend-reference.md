# Vaxis — API & Frontend Complete Reference

## Table of Contents
1. [Database Schema](#database-schema)
2. [Auth Layer](#auth-layer)
3. [Middleware](#middleware)
4. [API Endpoints](#api-endpoints)
5. [Frontend Routes](#frontend-routes)
6. [End-to-End Flows](#end-to-end-flows)
7. [Dependency Map](#dependency-map)

---

## Database Schema

### Better Auth tables (auto-managed)
| Table | Key Columns |
|-------|-------------|
| `user` | `id`, `name`, `email`, `emailVerified`, `image`, `createdAt`, `updatedAt` |
| `session` | `id`, `userId` (FK→user), `token`, `expiresAt`, `createdAt`, `updatedAt` |
| `account` | `id`, `userId` (FK→user), `provider`, `providerAccountId`, `accessToken`, … |

### Application tables
| Table | Key Columns | Notes |
|-------|-------------|-------|
| `application` | `id`, `user_id` (FK→user), `name`, `description`, `created_at`, `updated_at` | Root container for a project |
| `diagram` | `id`, `application_id` (FK→application), `name`, `scene_json`, `scene_version`, `parent_diagram_id` (FK→diagram), `parent_node_id`, `created_at`, `updated_at` | Hierarchical; `parent_diagram_id` NULL = root diagram |
| `node_child` | `diagram_id` (FK→diagram), `node_id` (text), `child_diagram_id` (FK→diagram) | Maps a Mermaid node → child diagram for drill-in |
| `chat_message` | `id`, `diagram_id` (FK→diagram root), `role` ('user'\|'assistant'), `content`, `created_at` | One thread per application root diagram |
| `application_share` | `token` (PK), `application_id` (FK→application, UNIQUE), `created_at` | Public share token; UPSERT on rotate |
| `cli_auth_state` | `state` (PK/UUID), `status` ('pending'\|'complete'\|'expired'), `token`, `user_name`, `user_email`, `created_at` | Device-flow polling state; 10-min TTL |

---

## Auth Layer

**Better Auth** runs at `/api/auth/*`. It handles:
- Email + password (`emailAndPassword: { enabled: true }`)
- Google OAuth (`prompt: 'select_account'` — always shows account picker)

Session stored as a cookie in the browser. Session `token` from `session.token` is given to the CLI after device-flow login.

### ALLOWED_ORIGINS (must stay in sync in 4 places)
- `apps/api/src/config.ts` — Hono CORS + Better Auth `trustedOrigins`
- `wrangler.toml [vars]`
- Google Cloud Console → Authorized redirect URIs

---

## Middleware

**`requireAuth`** (`apps/api/src/middleware.ts`) protects `/api/applications/*` and `/api/diagrams/*`.

Priority order:
1. **Cookie** — reads Better Auth session cookie (browser / web app)
2. **Bearer token** — reads `Authorization: Bearer <token>`, looks up in `session` table by `token` + `expiresAt > now()` (CLI)

Returns `401` if neither passes. Sets `c.Variables.userId` for downstream route handlers.

---

## API Endpoints

### Better Auth — `/api/auth/*`
All handled internally by Better Auth. No custom code.

| What | Endpoint |
|------|----------|
| Google OAuth start | `GET /api/auth/sign-in/social` |
| Google OAuth callback | `GET /api/auth/callback/google` |
| Sign out | `POST /api/auth/sign-out` |
| Get session | `GET /api/auth/get-session` |

---

### Applications — `/api/applications`
**Auth:** Cookie or Bearer required on all routes.

| Method | Path | Body / Query | Response | DB |
|--------|------|-------------|----------|----|
| `GET` | `/api/applications` | — | `[{ id, name, description, created_at, updated_at }]` | application |
| `POST` | `/api/applications` | `{ name, description? }` | `{ id, name, description }` 201 | application |
| `GET` | `/api/applications/:id` | — | Full row | application |
| `PUT` | `/api/applications/:id` | `{ name?, description? }` | `{ ok: true }` | application |
| `DELETE` | `/api/applications/:id` | — | `{ ok: true }` | application |
| `GET` | `/api/applications/:id/share` | — | `{ token \| null, created_at \| null }` | application_share |
| `POST` | `/api/applications/:id/share` | — | `{ token }` (creates or rotates) | application_share |
| `DELETE` | `/api/applications/:id/share` | — | `{ ok: true }` | application_share |

**Share token:** 18 random bytes → base64url (24 chars, 144-bit entropy). `POST` is always create-or-rotate (UPSERT). Old token invalidated immediately.

---

### Diagrams — `/api/diagrams`
**Auth:** Cookie or Bearer required on all routes.

| Method | Path | Body / Query | Response | DB |
|--------|------|-------------|----------|----|
| `GET` | `/api/diagrams` | `?applicationId=X` | `[{ id, name, created_at, updated_at, parent_diagram_id, scene_json }]` | diagram |
| `POST` | `/api/diagrams` | `{ applicationId, name }` | `{ id, name }` 201 | diagram |
| `GET` | `/api/diagrams/:id` | — | Full diagram + `child_nodes: { nodeId→childDiagramId }` + `ancestry: [{diagram_id, diagram_name, node_id?}]` | diagram, node_child |
| `GET` | `/api/diagrams/:id/tree` | — | `{ root_id, tree: { id, name, children: [...] } }` | diagram (recursive) |
| `PUT` | `/api/diagrams/:id` | `{ scene_json, scene_version, name? }` | `{ ok: true, scene_version }` or `409 { error: 'stale scene_version', current_scene_version }` | diagram |
| `PATCH` | `/api/diagrams/:id/meta` | `{ name }` | `{ ok: true, name }` | diagram |
| `DELETE` | `/api/diagrams/:id` | — | `{ ok: true }` (cascades all children) | diagram |
| `POST` | `/api/diagrams/:id/children` | `{ nodeId, nodeLabel? }` | `{ id, name, scene_json, scene_version, already_exists? }` 201/200 | diagram, node_child |
| `DELETE` | `/api/diagrams/:id/children/:nodeId` | — | `{ ok: true }` | diagram, node_child |
| `GET` | `/api/diagrams/:id/chat` | — | `{ messages: [{id, role, content, created_at}], chatThreadRootId }` | chat_message |
| `DELETE` | `/api/diagrams/:id/chat/last` | — | `{ ok: true }` (removes last user+assistant pair) | chat_message |
| `POST` | `/api/diagrams/:id/generate` | `{ prompt, currentMermaid? }` | `{ mermaid, drills: [{nodeId, mermaid}], messages: [...] }` | diagram, chat_message |

**Optimistic concurrency (`PUT`):** Incoming `scene_version` must be `>` stored version (or stored = 0). Otherwise returns `409` — client must re-fetch before saving.

**AI generation (`POST /generate`):**
1. Walks `parent_diagram_id` chain to find root → determines `chatThreadRootId`
2. Loads last 6 `chat_message` rows (3 turns) for context
3. Sends full architecture tree + diagram name + app name + history to Workers AI (`@cf/meta/llama-3.3-70b-instruct-fp8-fast`)
4. Parses response: main Mermaid + optional drill blocks (`%% vaxis:drill <nodeId>`)
5. Persists user prompt + assistant Mermaid as two `chat_message` rows
6. Returns `drills` array → frontend creates child diagrams automatically

---

### Public Share — `/api/public`
**Auth:** None. Open to anyone with a valid token.

| Method | Path | Response | DB |
|--------|------|----------|----|
| `GET` | `/api/public/:token` | `{ application: {id, name, description}, root_diagram: {id, name} }` | application_share, application, diagram |
| `GET` | `/api/public/:token/diagrams/:diagramId` | Full diagram + `child_nodes` + `ancestry` (same shape as auth'd GET) | diagram, node_child |
| `GET` | `/api/public/:token/diagrams/:diagramId/tree` | `{ root_id, tree: {...nested} }` | diagram |

**Security:** Every request validates that the `diagramId` belongs to the application the token unlocks. No user metadata, no tokens, no ownership data in response.

---

### CLI Device Flow — `/api/cli`
**Auth:** None on `start` / `poll`. `complete` validates Better Auth session from browser cookie internally.

| Method | Path | Body / Query | Response | DB |
|--------|------|-------------|----------|----|
| `POST` | `/api/cli/start` | — | `{ state (UUID), url (/cli-auth?state=...) }` | cli_auth_state |
| `GET` | `/api/cli/poll` | `?state=UUID` | `{ status: 'pending'\|'complete'\|'expired', token?, user_name?, user_email? }` | cli_auth_state |
| `POST` | `/api/cli/complete` | `{ state }` | `{ ok: true }` | cli_auth_state |

**Expiry:** 10 minutes from `created_at`. Checked on every `poll` — expired rows get status updated to `'expired'` lazily.

---

## Frontend Routes

**Tech:** React + React Router. All protected routes redirect to `/login` if no session.

| Route | Page | Auth | API calls | Purpose |
|-------|------|------|-----------|---------|
| `/login` | `LoginPage` | Public | `signIn.social()` (Google), `signIn.email()` | Email + Google login |
| `/` | `AppList` | Protected | `GET /api/applications` | List / create / search / pin applications |
| `/app/:appId` | `DiagramList` | Protected | `GET /api/applications/:appId`, `GET /api/diagrams?applicationId=X` | List / create / search / pin diagrams in an app |
| `/diagram/:diagramId` | `DiagramEditor` | Protected | Most diagram + chat + share endpoints | Full canvas editor (see below) |
| `/share#HASH` | `MermaidSharePage` | Public | None (hash decoded locally) | Legacy: show Mermaid as text for LLM copy-paste |
| `/view/:token` | `SharedViewerPage` | Public | `GET /api/public/:token`, `/api/public/:token/diagrams/*` | Read-only public viewer |
| `/view/:token/diagram/:diagramId` | `SharedViewerPage` | Public | Same public endpoints | Same viewer, starting at a specific diagram |
| `/cli-auth?state=X` | `CliAuthPage` | Public | `POST /api/cli/complete` | Device-flow browser leg — login + complete CLI auth |
| `*` | `NotFound` | — | — | 404 |

### DiagramEditor — API calls
- `GET /api/diagrams/:id` — load scene on mount
- `PUT /api/diagrams/:id` — auto-save canvas (debounced, optimistic concurrency)
- `PATCH /api/diagrams/:id/meta` — rename diagram
- `POST /api/diagrams/:id/generate` — AI prompt → new Mermaid + drills
- `POST /api/diagrams/:id/children` — create child diagram from drill
- `DELETE /api/diagrams/:id/children/:nodeId` — detach drill
- `GET /api/diagrams/:id/chat` — load chat history
- `DELETE /api/diagrams/:id/chat/last` — undo last AI turn
- `GET /api/diagrams/:id/tree` — bird's-eye / expand-subtree
- `POST /api/applications/:appId/share` — generate public link
- `GET /api/applications/:appId/share` — check existing share token

---

## End-to-End Flows

### 1. User creates and edits a diagram
```
Login (/login)
  → POST /api/auth/sign-in/social (Google OAuth)
  → GET / → GET /api/applications

Create app
  → POST /api/applications { name }
  → navigate /app/:appId

Create diagram
  → POST /api/diagrams { applicationId, name }
  → navigate /diagram/:diagramId

Edit canvas
  → PUT /api/diagrams/:id { scene_json, scene_version }  (auto-save)

AI generation
  → POST /api/diagrams/:id/generate { prompt }
  → frontend: mermaidToSceneElements → updateScene
  → if drills: POST /api/diagrams/:id/children { nodeId } for each
  → child badges appear on canvas nodes
```

### 2. Drill into a child diagram
```
Click node with drill badge
  → GET /api/diagrams/:childId  (already exists)
  → navigate /diagram/:childId
  → breadcrumbs built from ancestry[] in response

First-time drill (no child yet)
  → POST /api/diagrams/:parentId/children { nodeId }
  → creates new diagram with parent_diagram_id + parent_node_id set
  → navigate /diagram/:newChildId
```

### 3. Share a diagram publicly
```
DiagramEditor → Share menu
  → POST /api/applications/:appId/share  (create or rotate token)
  → shareable URL: /view/:token

Public user opens /view/:token
  → GET /api/public/:token  (returns root_diagram.id)
  → redirect to /view/:token/diagram/:rootDiagramId
  → GET /api/public/:token/diagrams/:diagramId
  → Excalidraw canvas in view-mode (no edit, no AI)
  → can drill into children via GET /api/public/:token/diagrams/:childId
```

### 4. CLI login (device flow)
```
$ vaxis login
  → POST /api/cli/start           → { state, url }
  → open browser: /cli-auth?state=UUID

Browser: /cli-auth
  → user sees login button (always fresh — no auto-complete)
  → click → signIn.social({ provider: 'google', callbackURL: '/cli-auth?state=UUID&returning=1' })
  → Google OAuth → redirect back with returning=1
  → useEffect: session && returning → POST /api/cli/complete { state }
  → cli_auth_state updated: status='complete', token=session.token

CLI: polling every 2s
  → GET /api/cli/poll?state=UUID  → { status: 'complete', token, user_name, user_email }
  → save token to ~/.config/vaxis/config.toml
  → ✓ Logged in as ...

$ vaxis apps list
  → GET /api/applications
  → Authorization: Bearer <session.token>
  → middleware: lookup session table by token + expiresAt
  → returns applications[]
```

---

## Dependency Map

```
user
 └── session          ← Better Auth, used by middleware (cookie + bearer)
 └── account          ← Better Auth, OAuth accounts
 └── application
      └── application_share   ← public share token (1 per app)
      └── diagram (root, parent_diagram_id = NULL)
           └── chat_message   ← AI thread (all diagrams in app share root thread)
           └── node_child     ← nodeId → child diagram mapping
           └── diagram (child, parent_diagram_id = root.id)
                └── node_child
                └── diagram (grandchild...)

cli_auth_state         ← independent, short-lived, links to session.token after complete
```

### Route → Endpoint → Table chain

| CLI Command | Endpoint | Tables touched |
|-------------|----------|---------------|
| `vaxis login` | `POST /cli/start`, `GET /cli/poll`, `POST /cli/complete` | cli_auth_state, session |
| `vaxis apps list` | `GET /api/applications` | application |
| `vaxis apps create` | `POST /api/applications` | application |

| Frontend Page | Key Endpoint | Tables touched |
|--------------|-------------|---------------|
| AppList | `GET /api/applications` | application |
| DiagramList | `GET /api/diagrams?applicationId` | diagram |
| DiagramEditor (load) | `GET /api/diagrams/:id` | diagram, node_child |
| DiagramEditor (save) | `PUT /api/diagrams/:id` | diagram |
| DiagramEditor (AI) | `POST /api/diagrams/:id/generate` | diagram, chat_message |
| DiagramEditor (drill) | `POST /api/diagrams/:id/children` | diagram, node_child |
| DiagramEditor (share) | `POST /api/applications/:id/share` | application_share |
| SharedViewerPage | `GET /api/public/:token/diagrams/:id` | application_share, diagram, node_child |
| CliAuthPage | `POST /api/cli/complete` | cli_auth_state, session |
