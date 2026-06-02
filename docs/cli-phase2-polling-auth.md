# CLI Phase 2 — Device Flow (Polling) Login

## What changed and why

Phase 1 used a `localhost` callback — the browser redirected to `http://localhost:54321` on your own machine. It worked but looked unofficial.

Phase 2 replaces this with a **polling flow** — exactly like GitHub CLI and Claude CLI. The browser opens your real domain (`vaxis.dev`), the CLI waits in the background, and the moment you finish logging in, the CLI receives the token automatically.

---

## How it works (step by step)

```
1. You run:  vaxis login

2. CLI calls:  POST https://vaxis.dev/api/cli/start
               ← gets back a unique state code (UUID)

3. CLI opens:  https://vaxis.dev/cli-auth?state=UUID
               ← this is YOUR real domain in the browser

4. You log in with Google on the vaxis.dev page
   Better Auth handles everything — Google OAuth, session creation

5. After login, the page calls:  POST /api/cli/complete  { state: UUID }
   ← stores the session token mapped to the state code in the database

6. Meanwhile, CLI is asking every 2 seconds:
   GET https://vaxis.dev/api/cli/poll?state=UUID
   → "pending"... "pending"... "pending"... "complete! here's your token"

7. CLI saves token to:  ~/.config/vaxis/config.toml

8. Terminal shows:  ✓ Logged in as Mani (mani@example.com)
```

---

## What was built

### Backend (apps/api) — 3 additions

#### 1. Database table: `cli_auth_state`
Migration: `src/db/migrations/006_add_cli_auth_state.sql`

Stores one row per `vaxis login` attempt:

| Column | Purpose |
|--------|---------|
| `state` | UUID — the unique code linking CLI ↔ browser |
| `status` | `pending` → `complete` or `expired` |
| `token` | Better Auth session token (filled after login) |
| `user_name` | User's display name (filled after login) |
| `user_email` | User's email (filled after login) |
| `created_at` | Timestamp — rows expire after 10 minutes |

#### 2. New API routes: `src/routes/cli.ts`

| Endpoint | Called by | Purpose |
|----------|-----------|---------|
| `POST /api/cli/start` | CLI | Creates a state row, returns the browser URL |
| `GET /api/cli/poll?state=` | CLI (every 2s) | Returns current status + token when ready |
| `POST /api/cli/complete` | Browser (frontend) | Saves session token to the state row |

#### 3. Route mounted in `src/index.ts`
`app.route('/api/cli', cliRoutes)` — added before requireAuth middleware.

---

### Frontend (apps/web) — 2 additions

#### 1. New page: `src/pages/CliAuthPage.tsx`

Minimal login page at `/cli-auth?state=UUID`:
- If already logged in → marks complete immediately, shows success
- If not logged in → shows "Continue with Google" button
- After Google login → marks complete, shows "Login successful! Close this tab."

#### 2. Route added in `src/App.tsx`
`/cli-auth` → `<CliAuthPage />`

---

### CLI (vaxis-cli) — 2 changes

#### 1. Removed `tiny_http` from `Cargo.toml`
No longer needed — no local HTTP server.

#### 2. Rewrote `src/commands/login.rs`
- Calls `/api/cli/start` to get state
- Opens browser to `/cli-auth?state=UUID`
- Polls `/api/cli/poll` every 2 seconds (timeout: 5 minutes)
- Saves token + profile to `~/.config/vaxis/config.toml`

---

## Environment variables

| Variable | Default | Purpose |
|----------|---------|---------|
| `VAXIS_AUTH_URL` | `https://vaxis.dev` | Points CLI to the right server |

For local development:
```bash
VAXIS_AUTH_URL=http://localhost:3000 vaxis login
```

---

## Testing

```bash
# Apply migration locally
pnpm wrangler d1 execute vaxis --local --file=apps/api/src/db/migrations/006_add_cli_auth_state.sql

# Start API (terminal 1)
cd apps/api && npm run dev

# Start frontend (terminal 2)
cd apps/web && npm run dev

# Install CLI (terminal 3)
cd vaxis-cli && cargo install --path .

# Run login
VAXIS_AUTH_URL=http://localhost:3000 vaxis login
```

---

## State lifecycle

```
POST /api/cli/start     → status: pending
                             ↓ (user logs in)
POST /api/cli/complete  → status: complete  (token saved)
                             ↓ (CLI reads it)
CLI done ✓

If user never logs in:
GET /api/cli/poll after 10 min → status: expired → CLI exits with error
```
