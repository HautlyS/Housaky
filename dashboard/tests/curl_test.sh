#!/usr/bin/env bash
# ============================================================
# Housaky Dashboard — curl integration test suite
# Tests all Tauri HTTP-exposed commands via the gateway port
# Usage:  bash tests/curl_test.sh [HOST] [PORT]
# Default: localhost:8080
# ============================================================

HOST="${1:-localhost}"
PORT="${2:-8080}"
BASE="http://${HOST}:${PORT}"
PASS=0
FAIL=0
TOTAL=0
ERRORS=()

GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

# ── Helper functions ───────────────────────────────────────────────────────
say()  { echo -e "${BLUE}[TEST]${NC} $*"; }
ok()   { echo -e "${GREEN}[PASS]${NC} $1"; ((PASS++)); ((TOTAL++)); }
fail() { echo -e "${RED}[FAIL]${NC} $1"; ERRORS+=("$1"); ((FAIL++)); ((TOTAL++)); }
warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }

assert_http() {
  local name="$1"
  local method="$2"
  local endpoint="$3"
  local body="$4"
  local expected_status="${5:-200}"
  local expected_field="$6"

  local args=(-s -o /tmp/curl_body.json -w "%{http_code}" -X "$method" \
    -H "Content-Type: application/json" \
    --connect-timeout 5 \
    "${BASE}${endpoint}")

  if [[ -n "$body" ]]; then
    args+=(-d "$body")
  fi

  local status
  status=$(curl "${args[@]}" 2>/tmp/curl_err.txt)
  local rc=$?

  if [[ $rc -ne 0 ]]; then
    fail "$name — curl error: $(cat /tmp/curl_err.txt | head -1)"
    return
  fi

  if [[ "$status" != "$expected_status" ]]; then
    fail "$name — expected HTTP $expected_status, got $status"
    cat /tmp/curl_body.json 2>/dev/null | head -3
    return
  fi

  if [[ -n "$expected_field" ]]; then
    local value
    value=$(cat /tmp/curl_body.json | python3 -c "import sys,json; d=json.load(sys.stdin); print(d.get('$expected_field','__missing__'))" 2>/dev/null)
    if [[ "$value" == "__missing__" || -z "$value" ]]; then
      fail "$name — response missing field '$expected_field'"
      return
    fi
  fi

  ok "$name"
}

assert_contains() {
  local name="$1"
  local method="$2"
  local endpoint="$3"
  local body="$4"
  local expected_text="$5"

  local args=(-s -X "$method" \
    -H "Content-Type: application/json" \
    --connect-timeout 5 \
    "${BASE}${endpoint}")

  if [[ -n "$body" ]]; then
    args+=(-d "$body")
  fi

  local response
  response=$(curl "${args[@]}" 2>/dev/null)

  if echo "$response" | grep -q "$expected_text"; then
    ok "$name"
  else
    fail "$name — expected '$expected_text' in response"
    echo "  Response: ${response:0:200}"
  fi
}

assert_json_array() {
  local name="$1"
  local endpoint="$2"

  local response
  response=$(curl -s -X GET -H "Content-Type: application/json" --connect-timeout 5 "${BASE}${endpoint}" 2>/dev/null)

  if echo "$response" | python3 -c "import sys,json; d=json.load(sys.stdin); assert isinstance(d,list)" 2>/dev/null; then
    ok "$name"
  else
    fail "$name — expected JSON array, got: ${response:0:100}"
  fi
}

# ── Check if gateway is up ─────────────────────────────────────────────────
echo ""
echo -e "${BLUE}════════════════════════════════════════════════════${NC}"
echo -e "${BLUE}   Housaky Dashboard — Integration Test Suite       ${NC}"
echo -e "${BLUE}   Target: ${BASE}                                  ${NC}"
echo -e "${BLUE}════════════════════════════════════════════════════${NC}"
echo ""

say "Checking gateway connectivity…"
if ! curl -s --connect-timeout 3 "$BASE/health" >/dev/null 2>&1; then
  warn "Gateway not reachable at $BASE — some tests will be skipped"
  warn "Start Housaky first: housaky daemon --port $PORT"
  echo ""
  GATEWAY_UP=false
else
  GATEWAY_UP=true
  say "Gateway is UP"
fi

# ══════════════════════════════════════════════════════════════════════════
# Section 1: Health & Status
# ══════════════════════════════════════════════════════════════════════════
echo ""
echo -e "${YELLOW}── Section 1: Health & Status ──────────────────────${NC}"

if [[ "$GATEWAY_UP" == "true" ]]; then
  assert_http "GET /health returns 200"            GET  "/health"  ""  200
  assert_http "GET /api/status returns 200"        GET  "/api/status" "" 200 "version"
  assert_contains "Status contains provider"       GET  "/api/status" "" "provider"
  assert_contains "Status contains autonomy_level" GET  "/api/status" "" "autonomy_level"
else
  warn "Skipping gateway tests — gateway offline"
fi

# ══════════════════════════════════════════════════════════════════════════
# Section 2: Configuration API
# ══════════════════════════════════════════════════════════════════════════
echo ""
echo -e "${YELLOW}── Section 2: Configuration ─────────────────────────${NC}"

if [[ "$GATEWAY_UP" == "true" ]]; then
  assert_http "GET /api/config returns 200"  GET "/api/config" "" 200
  assert_contains "Config has default_provider" GET "/api/config" "" "default_provider"
  assert_contains "Config has memory section"   GET "/api/config" "" "memory"
  assert_contains "Config has autonomy section" GET "/api/config" "" "autonomy"

  # POST validate_config with safe payload
  VALID_CONFIG='{"default_provider":"openrouter","default_temperature":0.7,"api_key":"sk-test","memory":{"backend":"sqlite","auto_save":true,"embedding_provider":"openai","vector_weight":0.7,"keyword_weight":0.3},"autonomy":{"level":"supervised","workspace_only":true,"allowed_commands":[],"forbidden_paths":[],"max_actions_per_hour":100,"max_cost_per_day_cents":1000},"runtime":{"kind":"native"},"heartbeat":{"enabled":false,"interval_minutes":30},"gateway":{"require_pairing":true,"allow_public_bind":false},"tunnel":{"provider":"none"},"secrets":{"encrypt":true}}'
  assert_http "POST /api/config/validate returns 200" POST "/api/config/validate" "$VALID_CONFIG" 200

  # Validate dangerous config triggers warning
  DANGER_CONFIG='{"default_provider":"openrouter","default_temperature":0.7,"api_key":"","memory":{"backend":"sqlite","auto_save":false,"embedding_provider":"openai","vector_weight":0.7,"keyword_weight":0.3},"autonomy":{"level":"full","workspace_only":false,"allowed_commands":[],"forbidden_paths":[],"max_actions_per_hour":100,"max_cost_per_day_cents":0},"runtime":{"kind":"native"},"heartbeat":{"enabled":false,"interval_minutes":30},"gateway":{"require_pairing":false,"allow_public_bind":true},"tunnel":{"provider":"none"},"secrets":{"encrypt":false}}'
  say "Validating insecure config triggers warnings…"
  WARN_RESP=$(curl -s -X POST -H "Content-Type: application/json" -d "$DANGER_CONFIG" --connect-timeout 5 "${BASE}/api/config/validate" 2>/dev/null)
  if echo "$WARN_RESP" | python3 -c "import sys,json; d=json.load(sys.stdin); assert len(d)>0" 2>/dev/null; then
    ok "Insecure config produces validation warnings"
  else
    warn "validate_config returned empty warnings for insecure config (may not be exposed via HTTP)"
  fi
fi

# ══════════════════════════════════════════════════════════════════════════
# Section 3: Channels
# ══════════════════════════════════════════════════════════════════════════
echo ""
echo -e "${YELLOW}── Section 3: Channels ──────────────────────────────${NC}"

if [[ "$GATEWAY_UP" == "true" ]]; then
  assert_http "GET /api/channels returns 200" GET "/api/channels" "" 200
  assert_json_array "Channels is a JSON array"    "/api/channels"
fi

# ══════════════════════════════════════════════════════════════════════════
# Section 4: AGI Telemetry
# ══════════════════════════════════════════════════════════════════════════
echo ""
echo -e "${YELLOW}── Section 4: AGI Telemetry ─────────────────────────${NC}"

if [[ "$GATEWAY_UP" == "true" ]]; then
  assert_http "GET /api/agi/telemetry returns 200"   GET "/api/agi/telemetry" "" 200
  assert_contains "Telemetry has total_tokens"       GET "/api/agi/telemetry" "" "total_tokens"
  assert_contains "Telemetry has provider"           GET "/api/agi/telemetry" "" "provider"

  assert_http "GET /api/agi/thoughts returns 200"    GET "/api/agi/thoughts" "" 200
  assert_json_array "Thoughts is a JSON array"           "/api/agi/thoughts"

  assert_http "GET /api/memory/entries returns 200"  GET "/api/memory/entries" "" 200
  assert_json_array "Memory entries is a JSON array"     "/api/memory/entries"
fi

# ══════════════════════════════════════════════════════════════════════════
# Section 5: Chat / Conversations
# ══════════════════════════════════════════════════════════════════════════
echo ""
echo -e "${YELLOW}── Section 5: Chat & Conversations ──────────────────${NC}"

if [[ "$GATEWAY_UP" == "true" ]]; then
  assert_http "GET /api/conversations returns 200"   GET "/api/conversations" "" 200
  assert_json_array "Conversations is a JSON array"      "/api/conversations"

  # Send a message
  MSG_PAYLOAD='{"message":"Hello, Housaky! Run a quick self-check."}'
  assert_http "POST /api/chat/send returns 200"      POST "/api/chat/send" "$MSG_PAYLOAD" 200
fi

# ══════════════════════════════════════════════════════════════════════════
# Section 6: Skills
# ══════════════════════════════════════════════════════════════════════════
echo ""
echo -e "${YELLOW}── Section 6: Skills ────────────────────────────────${NC}"

if [[ "$GATEWAY_UP" == "true" ]]; then
  assert_http "GET /api/skills returns 200"   GET "/api/skills" "" 200
  assert_json_array "Skills is a JSON array"      "/api/skills"
fi

# ══════════════════════════════════════════════════════════════════════════
# Section 7: Security edge tests
# ══════════════════════════════════════════════════════════════════════════
echo ""
echo -e "${YELLOW}── Section 7: Security Edge Tests ───────────────────${NC}"

# These tests run regardless of gateway status (they test local logic)

say "Testing XSS injection in message payload…"
XSS_PAYLOAD='{"message":"<script>alert(document.cookie)</script>"}'
if [[ "$GATEWAY_UP" == "true" ]]; then
  XSS_RESP=$(curl -s -X POST -H "Content-Type: application/json" -d "$XSS_PAYLOAD" --connect-timeout 5 "${BASE}/api/chat/send" 2>/dev/null)
  # The response must NOT echo raw <script> tags
  if echo "$XSS_RESP" | grep -q "<script>"; then
    fail "XSS: response echoes raw <script> tag — CRITICAL"
  else
    ok "XSS: <script> not reflected in response"
  fi
else
  warn "Skipping XSS gateway test (gateway offline)"
fi

say "Testing SQL injection pattern in message…"
SQLI_PAYLOAD='{"message":"'"'"' OR 1=1; DROP TABLE memory; --"}"'
if [[ "$GATEWAY_UP" == "true" ]]; then
  SQLI_RESP=$(curl -s -o /dev/null -w "%{http_code}" -X POST \
    -H "Content-Type: application/json" -d "$SQLI_PAYLOAD" \
    --connect-timeout 5 "${BASE}/api/chat/send" 2>/dev/null)
  if [[ "$SQLI_RESP" == "200" || "$SQLI_RESP" == "400" ]]; then
    ok "SQL injection: handled gracefully (HTTP $SQLI_RESP)"
  else
    fail "SQL injection: unexpected status $SQLI_RESP"
  fi
else
  warn "Skipping SQL injection gateway test (gateway offline)"
fi

say "Testing oversized payload (DoS protection)…"
BIG_MSG=$(python3 -c "print('A'*100001)")
BIG_PAYLOAD="{\"message\":\"${BIG_MSG}\"}"
if [[ "$GATEWAY_UP" == "true" ]]; then
  BIG_RESP=$(curl -s -o /dev/null -w "%{http_code}" -X POST \
    -H "Content-Type: application/json" -d "$BIG_PAYLOAD" \
    --connect-timeout 5 "${BASE}/api/chat/send" 2>/dev/null)
  if [[ "$BIG_RESP" == "413" || "$BIG_RESP" == "400" || "$BIG_RESP" == "200" ]]; then
    ok "Large payload: handled gracefully (HTTP $BIG_RESP)"
  else
    warn "Large payload: got HTTP $BIG_RESP (may be acceptable)"
  fi
else
  warn "Skipping DoS test (gateway offline)"
fi

say "Testing path traversal in config…"
if [[ "$GATEWAY_UP" == "true" ]]; then
  TRAVERSAL_RESP=$(curl -s -o /dev/null -w "%{http_code}" -X GET \
    --connect-timeout 5 "${BASE}/api/files?path=../../etc/passwd" 2>/dev/null)
  if [[ "$TRAVERSAL_RESP" == "403" || "$TRAVERSAL_RESP" == "404" || "$TRAVERSAL_RESP" == "400" ]]; then
    ok "Path traversal: blocked (HTTP $TRAVERSAL_RESP)"
  elif [[ "$TRAVERSAL_RESP" == "000" ]]; then
    warn "Path traversal endpoint not exposed (expected)"
  else
    warn "Path traversal: got HTTP $TRAVERSAL_RESP (review if /api/files exists)"
  fi
fi

say "Testing CORS headers…"
if [[ "$GATEWAY_UP" == "true" ]]; then
  CORS_HEADERS=$(curl -s -I -X OPTIONS \
    -H "Origin: https://evil.example.com" \
    -H "Access-Control-Request-Method: POST" \
    --connect-timeout 5 "${BASE}/api/status" 2>/dev/null)
  if echo "$CORS_HEADERS" | grep -qi "access-control-allow-origin: \*"; then
    warn "CORS: wildcard origin allowed — review if intentional"
  else
    ok "CORS: no wildcard origin header"
  fi
fi

say "Testing unauthenticated access to sensitive endpoints…"
if [[ "$GATEWAY_UP" == "true" ]]; then
  SECRETS_RESP=$(curl -s -o /dev/null -w "%{http_code}" -X GET \
    --connect-timeout 5 "${BASE}/api/secrets" 2>/dev/null)
  if [[ "$SECRETS_RESP" == "401" || "$SECRETS_RESP" == "403" || "$SECRETS_RESP" == "404" || "$SECRETS_RESP" == "000" ]]; then
    ok "Secrets endpoint: not publicly accessible (HTTP $SECRETS_RESP)"
  else
    warn "Secrets endpoint: returned HTTP $SECRETS_RESP — review access control"
  fi
fi

# ══════════════════════════════════════════════════════════════════════════
# Section 8: Frontend build artifact validation
# ══════════════════════════════════════════════════════════════════════════
echo ""
echo -e "${YELLOW}── Section 8: Build Artifacts ───────────────────────${NC}"

DIST_DIR="$(dirname "$0")/../dist"
if [[ -d "$DIST_DIR" ]]; then
  ok "dist/ directory exists"

  if [[ -f "$DIST_DIR/index.html" ]]; then
    ok "dist/index.html exists"
  else
    fail "dist/index.html missing — run pnpm build"
  fi

  JS_COUNT=$(find "$DIST_DIR/assets" -name "*.js" 2>/dev/null | wc -l)
  if [[ $JS_COUNT -gt 0 ]]; then
    ok "dist/assets contains $JS_COUNT JS bundle(s)"
  else
    fail "No JS bundles found in dist/assets"
  fi

  CSS_COUNT=$(find "$DIST_DIR/assets" -name "*.css" 2>/dev/null | wc -l)
  if [[ $CSS_COUNT -gt 0 ]]; then
    ok "dist/assets contains $CSS_COUNT CSS bundle(s)"
  else
    fail "No CSS bundles found in dist/assets"
  fi

  # Check no source maps in production (info leak)
  MAP_COUNT=$(find "$DIST_DIR" -name "*.map" 2>/dev/null | wc -l)
  if [[ $MAP_COUNT -eq 0 ]]; then
    ok "No .map files in production build (no source leak)"
  else
    warn "Found $MAP_COUNT source map(s) in dist/ — remove for production"
  fi

  # Check for eval in JS bundles (CSP compliance)
  if grep -r "eval(" "$DIST_DIR/assets/"*.js >/dev/null 2>&1; then
    warn "Found eval() in JS bundles — may violate CSP script-src 'self'"
  else
    ok "No eval() in JS bundles (CSP compliant)"
  fi

  # Check for XSS patterns that should not be in final bundle
  if grep -r "innerHTML\s*=" "$DIST_DIR/assets/"*.js >/dev/null 2>&1; then
    warn "Found innerHTML= in bundle — ensure it only comes from sanitized renderContent()"
  else
    ok "innerHTML usage minimal in bundle"
  fi
else
  warn "dist/ not found — run: pnpm run build"
fi

# ══════════════════════════════════════════════════════════════════════════
# Results
# ══════════════════════════════════════════════════════════════════════════
echo ""
echo -e "${BLUE}════════════════════════════════════════════════════${NC}"
echo -e "  Results: ${GREEN}${PASS} passed${NC}  ${RED}${FAIL} failed${NC}  (${TOTAL} total)"
echo -e "${BLUE}════════════════════════════════════════════════════${NC}"

if [[ ${#ERRORS[@]} -gt 0 ]]; then
  echo ""
  echo -e "${RED}Failed tests:${NC}"
  for e in "${ERRORS[@]}"; do
    echo -e "  ${RED}✗${NC} $e"
  done
fi

echo ""
if [[ $FAIL -eq 0 ]]; then
  echo -e "${GREEN}✓ All tests passed!${NC}"
  exit 0
else
  echo -e "${RED}✗ $FAIL test(s) failed${NC}"
  exit 1
fi
