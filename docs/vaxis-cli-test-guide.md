# Vaxis CLI — End-to-End Test Guide

Run these commands in order. Every command is on a single line — no line continuations.

---

## Prerequisites

- Vaxis server running at `http://localhost:3000`
- `jq` installed (`brew install jq`)
- `vaxis` binary built (`cargo build && cp target/debug/vaxis /usr/local/bin/vaxis`)

---

## Phase 1 — Setup & Auth

```bash
# Set server URL (one-time)
vaxis config set-url http://localhost:3000

# Check config saved correctly
vaxis config show

# Login with Google
vaxis login

# Confirm you're logged in
vaxis me --json
```

**Expected from `vaxis me --json`:**
```json
{ "name": "Your Name", "email": "you@example.com" }
```

---

## Phase 2 — Create App & Diagram

```bash
# Create a new application and capture its ID
APP_ID=$(vaxis apps create "Payment System" --description "Stripe-based checkout flow" --json | jq -r '.id')
echo "APP_ID=$APP_ID"

# Verify it appears in the list
vaxis apps list --json

# Create a root diagram inside the app
ROOT_ID=$(vaxis diagrams create $APP_ID "Payment System Architecture" --json | jq -r '.id')
echo "ROOT_ID=$ROOT_ID"

# Verify it appears in the list
vaxis diagrams list $APP_ID --json
```

---

## Phase 3 — Generate Root Diagram

Two modes — use whichever matches your setup:

### Mode A — You provide the Mermaid (works without server AI configured)

```bash
vaxis diagrams generate $ROOT_ID --mermaid "graph TD
    subgraph Frontend
        ui[Web App]
    end
    subgraph Backend
        api[API Gateway]
        auth[Auth Service]
        pay[Payment Service]
    end
    db[(PostgreSQL)]
    ui -->|HTTPS| api
    api -->|validates| auth
    api -->|charges| pay
    pay --> db
    %% vaxis:drill pay
    %% vaxis:drill auth" --json
```

### Mode B — Server AI generates (requires server AI to be configured)

```bash
vaxis diagrams generate $ROOT_ID --prompt "Design a payment system with a web frontend, API gateway, auth service, payment service using Stripe, and PostgreSQL database. Mark payment and auth as drill subsystems." --json
```

**Expected output (both modes):**
```json
{
  "diagram_id": "...",
  "mermaid": "graph TD\n    ...",
  "drills": [
    { "node_id": "pay",  "diagram_id": "diag_yyy", "name": "pay" },
    { "node_id": "auth", "diagram_id": "diag_zzz", "name": "auth" }
  ]
}
```

> If `drills` is empty in Mode A, the server is not parsing `%% vaxis:drill` annotations yet — check server-side implementation.
> If `mermaid` is empty in Mode B, the server's AI is not configured — use Mode A instead.

---

## Phase 4 — Inspect the Diagram

```bash
# Show diagram metadata + current Mermaid + child node map
vaxis diagrams show $ROOT_ID --json

# Show the full hierarchy as a tree (human-readable)
vaxis diagrams tree $ROOT_ID

# Show tree as JSON
vaxis diagrams tree $ROOT_ID --json
```

---

## Phase 5 — Drill Into a Subsystem

Copy the child diagram ID from the `drills` array in Phase 3 output, then:

```bash
# Replace with actual ID from drills output
PAYMENT_ID=<paste-payment-diagram-id-here>

# Generate detail for the payment subsystem — provide Mermaid directly
vaxis diagrams generate $PAYMENT_ID --mermaid "graph TD
    stripe[Stripe API]
    checkout[Checkout Handler]
    webhook[Webhook Receiver]
    idempotency[(Idempotency Store)]
    db[(PostgreSQL)]
    stripe -->|session.created| checkout
    stripe -->|payment.succeeded| webhook
    checkout --> idempotency
    webhook --> db" --json

# View the result
vaxis diagrams show $PAYMENT_ID --json
```

---

## Phase 6 — Undo and Retry

```bash
# If the last generation was bad — remove it
vaxis diagrams undo $PAYMENT_ID --json

# Retry with corrected Mermaid
vaxis diagrams generate $PAYMENT_ID --mermaid "graph TD
    stripe[Stripe API]
    checkout[Checkout Handler]
    webhook[Webhook Receiver]
    db[(PostgreSQL)]
    stripe -->|session| checkout
    stripe -->|event| webhook
    checkout --> db
    webhook --> db" --json
```

---

## Phase 7 — Import Raw Mermaid

```bash
# Save your own Mermaid directly without calling AI
vaxis diagrams import $PAYMENT_ID --mermaid "graph TD
    stripe[Stripe API]
    webhook[Webhook Handler]
    idempotency[(Idempotency Store)]
    db[(PostgreSQL)]
    stripe -->|payment.succeeded| webhook
    webhook -->|check| idempotency
    webhook -->|write| db" --json

# Confirm it saved
vaxis diagrams show $PAYMENT_ID --json
```

---

## Phase 8 — Rename a Diagram

```bash
vaxis diagrams rename $PAYMENT_ID "Payment Service Detail" --json
```

**Expected:**
```json
{ "ok": true, "diagram_id": "...", "name": "Payment Service Detail" }
```

---

## Phase 9 — Share the App

```bash
vaxis apps share $APP_ID --json
```

**Expected:**
```json
{ "url": "https://vaxis.dev/view/abc123xyz", "token": "abc123xyz", "created_at": "..." }
```

---

## Phase 10 — Format Reference (offline, no server needed)

```bash
# Get the full Mermaid format spec
vaxis diagrams format --json

# See just the supported diagram types
vaxis diagrams format --json | jq '[.supported_types[] | {type, when}]'
```

---

## Phase 11 — Update App Metadata

```bash
vaxis apps update $APP_ID --name "Payment System v2" --json
vaxis apps update $APP_ID --description "Updated Stripe checkout flow" --json
```

---

## Phase 12 — Cleanup

```bash
# Delete a specific diagram (and all its children)
vaxis diagrams delete $PAYMENT_ID --force

# Verify the tree shrank
vaxis diagrams tree $ROOT_ID

# Delete the whole app
vaxis apps delete $APP_ID --force

# Confirm it's gone
vaxis apps list --json
```

---

## Quick Smoke Test (no server needed)

Run these to verify the CLI binary is working before connecting to any server:

```bash
vaxis --version
vaxis --help
vaxis diagrams --help
vaxis apps --help
vaxis diagrams format --json
```

---

## Troubleshooting

| Symptom | Cause | Fix |
|---------|-------|-----|
| `mermaid: ""` and `drills: []` from generate | Server AI not returning output | Check server logs for AI call errors |
| `✗ Cannot connect to server` | Wrong URL or server not running | Run `vaxis config show`, verify server is up |
| `✗ Session expired` | Token stale | Run `vaxis login` again |
| `✗ Diagram not found` | Wrong ID | Run `vaxis diagrams list $APP_ID --json` to find correct ID |
| `error: unexpected argument ' '` | Space after `\` in multiline command | Put the whole command on one line |
| `apps share` returns 404 | Server `/share` endpoint not built yet | Build `POST /api/applications/:id/share` on server |
| `diagrams import` returns 404 | Server `/import` endpoint not built yet | Build `POST /api/diagrams/:id/import` on server |
| `diagrams patch` returns 404 | Server `/patch` endpoint not built yet | Build `POST /api/diagrams/:id/patch` on server |
