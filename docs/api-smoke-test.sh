#!/usr/bin/env bash
# docs/api-smoke-test.sh
#
# Vaxis CLI — Comprehensive API Smoke Test
#
# Exercises every backend endpoint the CLI is coupled to and reports PASS/FAIL
# per endpoint. Creates isolated test resources and cleans them up automatically.
#
# Usage:
#   bash docs/api-smoke-test.sh
#   INCLUDE_AI=1 bash docs/api-smoke-test.sh   # also test server-AI generate
#
# Prerequisites:
#   - vaxis binary on PATH  (or run from repo root with PATH=./target/release:$PATH)
#   - jq installed
#   - Logged in: vaxis me

set -uo pipefail

VAXIS="vaxis"
INCLUDE_AI="${INCLUDE_AI:-0}"

# ── Helpers ────────────────────────────────────────────────────────────────────

GREEN='\033[0;32m'
RED='\033[0;31m'
CYAN='\033[0;36m'
DIM='\033[2m'
RESET='\033[0m'

PASS=0
FAIL=0
FAILED_TESTS=()

ok()  { echo -e "  ${GREEN}✓${RESET}  $1"; ((PASS++)); }
fail(){ echo -e "  ${RED}✗${RESET}  $1"; ((FAIL++)); FAILED_TESTS+=("$1"); }

# Run a command, capturing output. Pass if exit 0 and output contains a non-empty
# value for the given jq key. Returns the raw output for further extraction.
run_and_check() {
  local label="$1" key="$2"; shift 2
  local out
  out=$("$VAXIS" "$@" 2>&1) || { fail "$label"; echo ""; return; }
  local val
  val=$(echo "$out" | jq -r "$key // empty" 2>/dev/null)
  if [[ -n "$val" && "$val" != "null" ]]; then
    ok "$label"
    echo "$val"
  else
    fail "$label (no $key in output: $out)"
    echo ""
  fi
}

# Run a command; pass if exit 0.
run_check() {
  local label="$1"; shift
  if "$VAXIS" "$@" >/dev/null 2>&1; then
    ok "$label"
  else
    local out
    out=$("$VAXIS" "$@" 2>&1 || true)
    fail "$label  ${DIM}→ $out${RESET}"
  fi
}

# Run a command and check that a JSON field equals an expected value.
run_check_eq() {
  local label="$1" key="$2" expected="$3"; shift 3
  local out
  out=$("$VAXIS" "$@" 2>&1) || { fail "$label (command failed: $out)"; return; }
  local val
  val=$(echo "$out" | jq -r "$key // empty" 2>/dev/null)
  if [[ "$val" == "$expected" ]]; then
    ok "$label"
  else
    fail "$label (expected $key=$expected, got $val)"
  fi
}

# Cleanup on EXIT — delete any test resources created.
APP_ID=""
DIAG_ID=""
CHILD_ID=""

cleanup() {
  echo ""
  echo -e "${DIM}── Cleanup ──────────────────────────────────────────────────${RESET}"
  if [[ -n "$CHILD_ID" ]]; then
    "$VAXIS" diagrams delete "$CHILD_ID" --force --json >/dev/null 2>&1 && \
      echo -e "  ${DIM}deleted child diagram $CHILD_ID${RESET}" || true
  fi
  if [[ -n "$DIAG_ID" ]]; then
    "$VAXIS" diagrams delete "$DIAG_ID" --force --json >/dev/null 2>&1 && \
      echo -e "  ${DIM}deleted diagram $DIAG_ID${RESET}" || true
  fi
  if [[ -n "$APP_ID" ]]; then
    "$VAXIS" apps delete "$APP_ID" --force --json >/dev/null 2>&1 && \
      echo -e "  ${DIM}deleted app $APP_ID${RESET}" || true
  fi
}
trap cleanup EXIT

# ── Section 0: Preflight ───────────────────────────────────────────────────────

echo ""
echo -e "${CYAN}── Section 0: Preflight ────────────────────────────────────${RESET}"

# Resolve live base URL from the CLI itself
BASE_URL=$("$VAXIS" config show --json 2>/dev/null | jq -r '.auth_url // "https://app.vaxis.dev"')
echo -e "  ${DIM}API URL: $BASE_URL${RESET}"

# Check login
ME_OUT=$("$VAXIS" me --json 2>&1) || true
ME_EMAIL=$(echo "$ME_OUT" | jq -r '.email // empty' 2>/dev/null)
if [[ -z "$ME_EMAIL" ]]; then
  echo -e "  ${RED}✗${RESET}  Not logged in. Run: vaxis login"
  exit 1
fi
ok "Auth: logged in as $ME_EMAIL"

# Check jq available
if ! command -v jq &>/dev/null; then
  echo -e "  ${RED}✗${RESET}  jq not found — required for JSON extraction"
  exit 1
fi
ok "jq available"

# ── Section 1: Apps CRUD ───────────────────────────────────────────────────────

echo ""
echo -e "${CYAN}── Section 1: Apps CRUD ────────────────────────────────────${RESET}"

# GET /api/applications
run_check "GET  /api/applications" apps list --json

# POST /api/applications
TS=$(date +%s)
CREATE_OUT=$("$VAXIS" apps create "smoke-test-$TS" --description "api smoke test" --json 2>&1) || true
APP_ID=$(echo "$CREATE_OUT" | jq -r '.id // empty' 2>/dev/null)
if [[ -n "$APP_ID" && "$APP_ID" != "null" ]]; then
  ok "POST /api/applications  (id=$APP_ID)"
else
  fail "POST /api/applications  (output: $CREATE_OUT)"
fi

# PUT /api/applications/{id}  (update name via --json flags)
if [[ -n "$APP_ID" ]]; then
  run_check_eq "PUT  /api/applications/{id}" ".ok" "true" \
    apps update "$APP_ID" --name "smoke-test-${TS}-renamed" --json
fi

# ── Section 2: Diagrams CRUD ───────────────────────────────────────────────────

echo ""
echo -e "${CYAN}── Section 2: Diagrams CRUD ─────────────────────────────────${RESET}"

# GET /api/applications/{id}/diagrams
if [[ -n "$APP_ID" ]]; then
  run_check "GET  /api/applications/{id}/diagrams" diagrams list "$APP_ID" --json
fi

# POST /api/diagrams
if [[ -n "$APP_ID" ]]; then
  CREATE_DIAG_OUT=$("$VAXIS" diagrams create "$APP_ID" "smoke-diagram-$TS" --json 2>&1) || true
  DIAG_ID=$(echo "$CREATE_DIAG_OUT" | jq -r '.id // empty' 2>/dev/null)
  if [[ -n "$DIAG_ID" && "$DIAG_ID" != "null" ]]; then
    ok "POST /api/diagrams  (id=$DIAG_ID)"
  else
    fail "POST /api/diagrams  (output: $CREATE_DIAG_OUT)"
  fi
fi

# GET /api/diagrams/{id}
if [[ -n "$DIAG_ID" ]]; then
  run_check "GET  /api/diagrams/{id}" diagrams show "$DIAG_ID" --json
fi

# ── Section 3: Generate — direct --mermaid path + drill child creation ─────────

echo ""
echo -e "${CYAN}── Section 3: Generate (--mermaid) + drill child ────────────${RESET}"

# Flowchart with one drill marker — exercises:
#   POST /api/diagrams/{id}/generate  (direct mermaid)
#   POST /api/diagrams/{id}/children  (drill auto-expansion)
MERMAID='flowchart TB
  api[API Gateway]
  auth[Auth Service]
  db[(PostgreSQL)]
  api --> auth
  auth --> db
%% vaxis:drill auth'

if [[ -n "$DIAG_ID" ]]; then
  GEN_OUT=$("$VAXIS" diagrams generate "$DIAG_ID" --mermaid "$MERMAID" --json 2>&1) || true
  GEN_MERMAID=$(echo "$GEN_OUT" | jq -r '.mermaid // empty' 2>/dev/null)
  GEN_DRILLS=$(echo "$GEN_OUT"  | jq -r '.drills | length' 2>/dev/null)

  if [[ -n "$GEN_MERMAID" && "$GEN_MERMAID" != "null" ]]; then
    ok "POST /api/diagrams/{id}/generate (--mermaid)"
  else
    fail "POST /api/diagrams/{id}/generate (--mermaid)  (output: $GEN_OUT)"
  fi

  if [[ "$GEN_DRILLS" -ge 1 ]] 2>/dev/null; then
    CHILD_ID=$(echo "$GEN_OUT" | jq -r '.drills[0].diagram_id // empty' 2>/dev/null)
    ok "POST /api/diagrams/{id}/children  (drill id=$CHILD_ID)"
  else
    fail "POST /api/diagrams/{id}/children  (expected ≥1 drill, got: $GEN_OUT)"
  fi
fi

# ── Section 4: Tree ────────────────────────────────────────────────────────────

echo ""
echo -e "${CYAN}── Section 4: Tree ──────────────────────────────────────────${RESET}"

if [[ -n "$DIAG_ID" ]]; then
  TREE_OUT=$("$VAXIS" diagrams tree "$DIAG_ID" --json 2>&1) || true
  TREE_ROOT=$(echo "$TREE_OUT" | jq -r '.tree.id // empty' 2>/dev/null)
  if [[ -n "$TREE_ROOT" && "$TREE_ROOT" != "null" ]]; then
    ok "GET  /api/diagrams/{id}/tree"
  else
    fail "GET  /api/diagrams/{id}/tree  (output: $TREE_OUT)"
  fi
fi

# ── Section 5: Share ───────────────────────────────────────────────────────────

echo ""
echo -e "${CYAN}── Section 5: Share ─────────────────────────────────────────${RESET}"

if [[ -n "$DIAG_ID" ]]; then
  # GET /api/diagrams/{id}/share  (read existing state) + POST if unshared
  SHARE_OUT=$("$VAXIS" diagrams share "$DIAG_ID" --json 2>&1) || true
  SHARE_URL=$(echo "$SHARE_OUT" | jq -r '.url // empty' 2>/dev/null)
  if [[ -n "$SHARE_URL" && "$SHARE_URL" != "null" ]]; then
    ok "GET+POST /api/diagrams/{id}/share  (view=$SHARE_URL)"
  else
    fail "GET+POST /api/diagrams/{id}/share  (output: $SHARE_OUT)"
  fi

  # POST /api/diagrams/{id}/share  (--rotate: force new token)
  ROTATE_OUT=$("$VAXIS" diagrams share "$DIAG_ID" --rotate --json 2>&1) || true
  ROTATE_URL=$(echo "$ROTATE_OUT" | jq -r '.url // empty' 2>/dev/null)
  if [[ -n "$ROTATE_URL" && "$ROTATE_URL" != "null" ]]; then
    ok "POST /api/diagrams/{id}/share --rotate"
  else
    fail "POST /api/diagrams/{id}/share --rotate  (output: $ROTATE_OUT)"
  fi

  # DELETE /api/diagrams/{id}/share
  REVOKE_OUT=$("$VAXIS" diagrams share "$DIAG_ID" --revoke --json 2>&1) || true
  if echo "$REVOKE_OUT" | jq -e '.ok == true and .shared == false' >/dev/null 2>&1; then
    ok "DELETE /api/diagrams/{id}/share"
  else
    fail "DELETE /api/diagrams/{id}/share  (output: $REVOKE_OUT)"
  fi
fi

# ── Section 6: Chat Sessions ───────────────────────────────────────────────────

echo ""
echo -e "${CYAN}── Section 6: Chat Sessions ─────────────────────────────────${RESET}"

SESSION_ID=""
if [[ -n "$DIAG_ID" ]]; then
  # GET /api/diagrams/{id}/chat/sessions
  run_check "GET  /api/diagrams/{id}/chat/sessions" diagrams sessions list "$DIAG_ID" --json

  # POST /api/diagrams/{id}/chat/sessions
  SESS_OUT=$("$VAXIS" diagrams sessions create "$DIAG_ID" --title "smoke-session-$TS" --json 2>&1) || true
  SESSION_ID=$(echo "$SESS_OUT" | jq -r '.session.id // empty' 2>/dev/null)
  if [[ -n "$SESSION_ID" && "$SESSION_ID" != "null" ]]; then
    ok "POST /api/diagrams/{id}/chat/sessions  (id=$SESSION_ID)"
  else
    fail "POST /api/diagrams/{id}/chat/sessions  (output: $SESS_OUT)"
  fi

  # PATCH /api/diagrams/{id}/chat/sessions/{sid}
  if [[ -n "$SESSION_ID" ]]; then
    run_check "PATCH /api/diagrams/{id}/chat/sessions/{sid}" \
      diagrams sessions rename "$DIAG_ID" "$SESSION_ID" "smoke-session-renamed-$TS" --json
  fi
fi

# ── Section 7: Ask (prose answer, no edit) ─────────────────────────────────────

echo ""
echo -e "${CYAN}── Section 7: Ask ────────────────────────────────────────────${RESET}"

if [[ -n "$DIAG_ID" ]]; then
  # POST /api/diagrams/{id}/generate  with intent=ask
  ASK_OUT=$("$VAXIS" diagrams ask "$DIAG_ID" --prompt "What services does this diagram show?" --json 2>&1) || true
  ASK_ANSWER=$(echo "$ASK_OUT" | jq -r '.answer // empty' 2>/dev/null)
  if [[ -n "$ASK_ANSWER" && "$ASK_ANSWER" != "null" ]]; then
    ok "POST /api/diagrams/{id}/generate (ask intent)"
  else
    fail "POST /api/diagrams/{id}/generate (ask intent)  (output: $ASK_OUT)"
  fi
fi

# ── Section 8: Import ──────────────────────────────────────────────────────────

echo ""
echo -e "${CYAN}── Section 8: Import ─────────────────────────────────────────${RESET}"

IMPORT_MERMAID='flowchart LR
  user[User] --> app[App]
  app --> api[API]'

if [[ -n "$DIAG_ID" ]]; then
  IMPORT_OUT=$("$VAXIS" diagrams import "$DIAG_ID" --mermaid "$IMPORT_MERMAID" --json 2>&1) || true
  IMPORT_OK=$(echo "$IMPORT_OUT" | jq -r '.ok // empty' 2>/dev/null)
  if [[ "$IMPORT_OK" == "true" ]]; then
    ok "POST /api/diagrams/{id}/import"
  else
    fail "POST /api/diagrams/{id}/import  (output: $IMPORT_OUT)"
  fi
fi

# ── Section 9: Undo ────────────────────────────────────────────────────────────

echo ""
echo -e "${CYAN}── Section 9: Undo ───────────────────────────────────────────${RESET}"

if [[ -n "$DIAG_ID" ]]; then
  # Generate something first so there's a turn to undo
  "$VAXIS" diagrams generate "$DIAG_ID" --mermaid "$IMPORT_MERMAID" --json >/dev/null 2>&1 || true

  UNDO_OUT=$("$VAXIS" diagrams undo "$DIAG_ID" --json 2>&1) || true
  UNDO_OK=$(echo "$UNDO_OUT" | jq -r '.ok // empty' 2>/dev/null)
  if [[ "$UNDO_OK" == "true" ]]; then
    ok "DELETE /api/diagrams/{id}/chat/messages/last"
  else
    fail "DELETE /api/diagrams/{id}/chat/messages/last  (output: $UNDO_OUT)"
  fi
fi

# ── Section 10: Rename Diagram ─────────────────────────────────────────────────

echo ""
echo -e "${CYAN}── Section 10: Rename Diagram ───────────────────────────────${RESET}"

if [[ -n "$DIAG_ID" ]]; then
  RENAME_OUT=$("$VAXIS" diagrams rename "$DIAG_ID" "smoke-diagram-renamed-$TS" --json 2>&1) || true
  RENAME_OK=$(echo "$RENAME_OUT" | jq -r '.ok // empty' 2>/dev/null)
  if [[ "$RENAME_OK" == "true" ]]; then
    ok "PATCH /api/diagrams/{id}  (rename)"
  else
    fail "PATCH /api/diagrams/{id}  (rename)  (output: $RENAME_OUT)"
  fi
fi

# ── Section 11: Rules-Check ────────────────────────────────────────────────────

echo ""
echo -e "${CYAN}── Section 11: Rules-Check ──────────────────────────────────${RESET}"

RULES_OUT=$("$VAXIS" diagrams rules-check --json 2>&1) || true
RULES_OK=$(echo "$RULES_OUT" | jq -r '.ok // empty' 2>/dev/null)
if [[ "$RULES_OK" == "true" ]]; then
  ok "GET  /api/diagrams/rules  (no schema drift)"
elif [[ -n "$RULES_OK" ]]; then
  # Endpoint responded but drift detected — not a network failure
  DRIFT=$(echo "$RULES_OUT" | jq -r '.drift[]' 2>/dev/null | head -3)
  fail "GET  /api/diagrams/rules  (schema drift: $DRIFT)"
else
  fail "GET  /api/diagrams/rules  (output: $RULES_OUT)"
fi

# ── Section 12: Server-AI Generate (opt-in) ────────────────────────────────────

if [[ "$INCLUDE_AI" == "1" && -n "$DIAG_ID" ]]; then
  echo ""
  echo -e "${CYAN}── Section 12: Server-AI Generate (INCLUDE_AI=1) ────────────${RESET}"

  AI_OUT=$("$VAXIS" diagrams generate "$DIAG_ID" --prompt "Draw a simple web app with a frontend, API, and database" --intent replace --json 2>&1) || true
  AI_MERMAID=$(echo "$AI_OUT" | jq -r '.mermaid // empty' 2>/dev/null)
  if [[ -n "$AI_MERMAID" && "$AI_MERMAID" != "null" ]]; then
    ok "POST /api/diagrams/{id}/generate (--prompt server-AI)"
  else
    fail "POST /api/diagrams/{id}/generate (--prompt server-AI)  (output: $AI_OUT)"
  fi
fi

# ── Summary ────────────────────────────────────────────────────────────────────

TOTAL=$((PASS + FAIL))
echo ""
echo -e "${CYAN}────────────────────────────────────────────────────────────${RESET}"
echo -e "  API Smoke Test Results  ${DIM}(target: $BASE_URL)${RESET}"
echo -e "  Passed: ${GREEN}$PASS${RESET} / $TOTAL"
if [[ $FAIL -gt 0 ]]; then
  echo -e "  Failed: ${RED}$FAIL${RESET}"
  echo ""
  echo -e "  Failed tests:"
  for t in "${FAILED_TESTS[@]}"; do
    echo -e "    ${RED}✗${RESET}  $t"
  done
fi
echo -e "${CYAN}────────────────────────────────────────────────────────────${RESET}"
echo ""

[[ $FAIL -eq 0 ]]
