#!/bin/bash
# Vaxis CLI — E2E Test Script
# Usage:  bash docs/test-commands.sh
# Cleanup: CLEANUP=1 bash docs/test-commands.sh

set -euo pipefail

# ── Phase 1: Auth ──────────────────────────────────────────────────────────────
echo "==> Phase 1: Auth"
vaxis config set-url http://localhost:3000
vaxis me --json

# ── Phase 2: Create App & Root Diagram ────────────────────────────────────────
echo "==> Phase 2: Create App & Root Diagram"
APP_ID=$(vaxis apps create "Payment System" --description "Stripe-based checkout" --json | jq -r '.id')
echo "    APP_ID=$APP_ID"

ROOT_ID=$(vaxis diagrams create "$APP_ID" "Payment System Architecture" --json | jq -r '.id')
echo "    ROOT_ID=$ROOT_ID"

# ── Phase 3: Root Diagram ─────────────────────────────────────────────────────
echo "==> Phase 3: Generate root diagram"

# %% vaxis:drill must appear immediately after the node it annotates
read -r -d '' ROOT_MERMAID <<'EOF'
graph TD
    subgraph Frontend
        ui[Web App]
        mobile[Mobile App]
    end
    subgraph Backend
        api[API Gateway]
        auth[Auth Service]
        %% vaxis:drill auth
        pay[Payment Service]
        %% vaxis:drill pay
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
EOF

GENERATE_OUT=$(vaxis diagrams generate "$ROOT_ID" --mermaid "$ROOT_MERMAID" --json)
echo "$GENERATE_OUT"

# ── Phase 4: Inspect ───────────────────────────────────────────────────────────
echo "==> Phase 4: Inspect root diagram"
vaxis diagrams show "$ROOT_ID" --json
vaxis diagrams tree "$ROOT_ID"

# ── Phase 5: Drill — Payment Service ──────────────────────────────────────────
echo "==> Phase 5: Drill into Payment Service"
# child_nodes is a { nodeId: childDiagramId } map on the tree root
PAYMENT_ID=$(vaxis diagrams tree "$ROOT_ID" --json | jq -r '.tree.child_nodes.pay')
echo "    PAYMENT_ID=$PAYMENT_ID"

read -r -d '' PAYMENT_MERMAID <<'EOF'
graph TD
    stripe[Stripe API]
    checkout[Checkout Handler]
    webhook[Webhook Receiver]
    idempotency[(Idempotency Store)]
    db[(PostgreSQL)]
    cache[(Redis Cache)]
    stripe -->|session.created| checkout
    stripe -->|payment.succeeded| webhook
    checkout -->|check duplicate| idempotency
    checkout -->|write| db
    webhook -->|update status| db
    checkout -->|cache session| cache
    webhook -->|invalidate| cache
EOF

vaxis diagrams generate "$PAYMENT_ID" --mermaid "$PAYMENT_MERMAID" --json

# ── Phase 6: Drill — Auth Service ─────────────────────────────────────────────
echo "==> Phase 6: Drill into Auth Service"
AUTH_ID=$(vaxis diagrams tree "$ROOT_ID" --json | jq -r '.tree.child_nodes.auth')
echo "    AUTH_ID=$AUTH_ID"

read -r -d '' AUTH_MERMAID <<'EOF'
graph TD
    google[Google OAuth]
    session[Session Manager]
    jwt[JWT Issuer]
    db[(User Store)]
    cache[(Session Cache)]
    google -->|id_token| session
    session -->|verify| jwt
    session -->|store user| db
    session -->|cache token| cache
    jwt -->|validate| cache
EOF

vaxis diagrams generate "$AUTH_ID" --mermaid "$AUTH_MERMAID" --json

# ── Phase 7: Share ─────────────────────────────────────────────────────────────
echo "==> Phase 7: Share app"
vaxis apps share "$APP_ID" --json

echo ""
echo "Done. APP_ID=$APP_ID  ROOT_ID=$ROOT_ID"

# ── Cleanup (opt-in: CLEANUP=1 bash test-commands.sh) ─────────────────────────
if [[ "${CLEANUP:-0}" == "1" ]]; then
  echo "==> Cleanup"
  vaxis apps delete "$APP_ID" --force
fi
