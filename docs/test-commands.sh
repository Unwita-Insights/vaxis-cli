#!/bin/bash
# Vaxis CLI — E2E Test Commands
# Run each block in order in your terminal

# ── Phase 1: Auth ──────────────────────────────────────────────────────────────
vaxis config set-url http://localhost:3000
vaxis me --json

# ── Phase 2: Create App & Diagram ──────────────────────────────────────────────
APP_ID=$(vaxis apps create "Payment System" --description "Stripe-based checkout" --json | jq -r '.id')
echo "APP_ID=$APP_ID"

ROOT_ID=$(vaxis diagrams create $APP_ID "Payment System Architecture" --json | jq -r '.id')
echo "ROOT_ID=$ROOT_ID"

# ── Phase 3: Load Mermaid ───────────────────────────────────────────────────────
vaxis diagrams generate $ROOT_ID --mermaid "graph TD
    subgraph Frontend
        ui[Web App]
        mobile[Mobile App]
    end
    subgraph Backend
        api[API Gateway]
        auth[Auth Service]
        pay[Payment Service]
        notify[Notification Service]
    end
    db[(PostgreSQL)]
    cache[(Redis Cache)]
    ui -->|HTTPS| api
    mobile -->|HTTPS| api
    api -->|validates| auth
    api -->|charges| pay
    api -->|triggers| notify
    pay -->|reads| cache
    pay -->|writes| db
    auth --> db
    %% vaxis:drill pay
    %% vaxis:drill auth" --json

# ── Phase 4: Inspect ───────────────────────────────────────────────────────────
vaxis diagrams show $ROOT_ID --json
vaxis diagrams tree $ROOT_ID

# ── Phase 5: Drill (replace IDs from Phase 3 drills[] output) ─────────────────
# PAYMENT_ID=<paste-from-drills-output>
# AUTH_ID=<paste-from-drills-output>

# vaxis diagrams generate $PAYMENT_ID --mermaid "graph TD
#     stripe[Stripe API]
#     checkout[Checkout Handler]
#     webhook[Webhook Receiver]
#     idempotency[(Idempotency Store)]
#     db[(PostgreSQL)]
#     stripe -->|session.created| checkout
#     stripe -->|payment.succeeded| webhook
#     checkout --> idempotency
#     webhook --> db" --json

# ── Phase 6: Share ─────────────────────────────────────────────────────────────
vaxis apps share $APP_ID --json

# ── Cleanup (optional) ─────────────────────────────────────────────────────────
# vaxis apps delete $APP_ID --force
