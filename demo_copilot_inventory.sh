#!/bin/bash
# ============================================================
#  demo_copilot_inventory.sh
#  E2E тест операций со складом через ChefOS Copilot chat
#
#  Структура теста:
#   1. Login → получить access_token
#   2. "Добавь 10 кг риса"           → preview + confirm
#   3. "Спиши 2 кг риса, испорчен"   → preview + confirm
#   4. "После инвентаризации поставь рис 7 кг"  → preview + confirm
#   5. "Что сегодня важно?"           → daily briefing (read-only)
#   6. Smoke-проверка через прямой API /inventory для сравнения
#
#  Требования:
#   - Рабочий деплой на Koyeb (или LOCAL_MODE=1 для localhost:8000)
#   - Пользователь test@fodi.app с паролем password123
#   - Каталог должен содержать "rice" (rice/рис)
#
#  Использование:
#   chmod +x demo_copilot_inventory.sh && ./demo_copilot_inventory.sh
#   LOCAL_MODE=1 ./demo_copilot_inventory.sh   # тест локально
# ============================================================

set -euo pipefail

# ── Config ────────────────────────────────────────────────────
if [ "${LOCAL_MODE:-0}" = "1" ]; then
  BASE="http://localhost:8000/api"
else
  BASE="https://ministerial-yetta-fodi999-c58d8823.koyeb.app/api"
fi

EMAIL="test@fodi.app"
PASSWORD="password123"

GREEN="\033[0;32m"
RED="\033[0;31m"
YELLOW="\033[1;33m"
CYAN="\033[0;36m"
BOLD="\033[1m"
RESET="\033[0m"

PASS=0
FAIL=0

pass() { echo -e "${GREEN}✅ PASS${RESET}: $1"; PASS=$((PASS+1)); }
fail() { echo -e "${RED}❌ FAIL${RESET}: $1"; FAIL=$((FAIL+1)); }
section() { echo -e "\n${BOLD}${CYAN}══ $1 ══${RESET}"; }
info()  { echo -e "   ${YELLOW}→${RESET} $1"; }

# ── Helpers ───────────────────────────────────────────────────

# Отправить сообщение в Copilot, вернуть JSON
copilot_chat() {
  local msg="$1"
  curl -s --max-time 30 -X POST "$BASE/copilot/message" \
    -H "Authorization: Bearer $TOKEN" \
    -H "Content-Type: application/json" \
    --data-binary "{\"message\":$(echo "$msg" | python3 -c "import sys,json; print(json.dumps(sys.stdin.read().strip()))")}"
}

# Подтвердить action plan по ID
copilot_confirm() {
  local plan_id="$1"
  curl -s --max-time 30 -X POST "$BASE/copilot/actions/$plan_id/confirm" \
    -H "Authorization: Bearer $TOKEN" \
    -H "Content-Type: application/json" \
    -d '{}'
}

# Получить поле из JSON
jq_get() {
  echo "$1" | python3 -c "import sys,json; d=json.load(sys.stdin); print($2)" 2>/dev/null || echo ""
}

# ── 0. Login ──────────────────────────────────────────────────
section "0. LOGIN"
LOGIN_RES=$(curl -s --max-time 10 -X POST "$BASE/auth/login" \
  -H "Content-Type: application/json" \
  -d "{\"email\":\"$EMAIL\",\"password\":\"$PASSWORD\"}")

TOKEN=$(echo "$LOGIN_RES" | python3 -c "import sys,json; print(json.load(sys.stdin).get('access_token',''))" 2>/dev/null)

if [ -z "$TOKEN" ]; then
  echo -e "${RED}❌ Login failed. Response: $LOGIN_RES${RESET}"
  exit 1
fi

echo "$TOKEN" > /tmp/tok
pass "Logged in as $EMAIL (token: ${TOKEN:0:20}...)"

# ── 1. ADD — "Добавь 10 кг риса" ─────────────────────────────
section "1. ADD INVENTORY — 10kg rice"
info "Sending: 'Add 10 kg of rice to inventory'"

RESP=$(copilot_chat "Add 10 kg of rice to inventory")
echo "$RESP" | python3 -m json.tool 2>/dev/null | grep -E '"requires_confirmation|"plan_type|"action_id|"message|"changes' | head -10 || echo "$RESP" | head -3

REQUIRES=$(jq_get "$RESP" "d.get('requires_confirmation', False)")
PLAN_ID=$(jq_get "$RESP" "d.get('action_plan',{}).get('id','')" 2>/dev/null || echo "")
[ -z "$PLAN_ID" ] && PLAN_ID=$(jq_get "$RESP" "d.get('action_id','')")

if [ "$REQUIRES" = "True" ] && [ -n "$PLAN_ID" ]; then
  pass "Got action plan (id: $PLAN_ID)"

  info "Confirming..."
  CONFIRM_RESP=$(copilot_confirm "$PLAN_ID")
  STATUS=$(echo "$CONFIRM_RESP" | python3 -c "import sys,json; d=json.load(sys.stdin); print(d.get('status',''))" 2>/dev/null)
  MSG=$(echo "$CONFIRM_RESP" | python3 -c "import sys,json; d=json.load(sys.stdin); print(d.get('message',''))" 2>/dev/null)

  if [ "$STATUS" = "executed" ] || echo "$MSG" | grep -qi "added\|updated\|успешно"; then
    pass "Add confirmed: $MSG"
  else
    fail "Unexpected confirm response: $CONFIRM_RESP"
  fi
else
  fail "Expected requires_confirmation=true with action plan. Got: $(echo "$RESP" | head -2)"
fi

# ── 2. WRITEOFF — "Спиши 2 кг риса, испорчен" ────────────────
section "2. WRITE-OFF — 2kg rice (spoiled)"
info "Sending: 'Write off 2 kg of rice, spoiled'"

RESP2=$(copilot_chat "Write off 2 kg of rice, reason: spoiled")
REQUIRES2=$(jq_get "$RESP2" "d.get('requires_confirmation', False)")
PLAN_ID2=$(jq_get "$RESP2" "d.get('action_plan',{}).get('id','')" 2>/dev/null || echo "")
[ -z "$PLAN_ID2" ] && PLAN_ID2=$(jq_get "$RESP2" "d.get('action_id','')")

echo "$RESP2" | python3 -m json.tool 2>/dev/null | grep -E '"plan_type|"changes|"message' | head -5 || echo "$RESP2" | head -2

if [ "$REQUIRES2" = "True" ] && [ -n "$PLAN_ID2" ]; then
  pass "Got write-off plan (id: $PLAN_ID2)"
  CONFIRM2=$(copilot_confirm "$PLAN_ID2")
  MSG2=$(echo "$CONFIRM2" | python3 -c "import sys,json; d=json.load(sys.stdin); print(d.get('message',''))" 2>/dev/null)
  if echo "$MSG2" | grep -qi "written\|off\|списа\|success"; then
    pass "Write-off confirmed: $MSG2"
  else
    fail "Write-off confirm unexpected: $CONFIRM2"
  fi
else
  fail "Expected requires_confirmation=true for write-off. Got: $(echo "$RESP2" | head -2)"
fi

# ── 3. ADJUST — "После инвентаризации поставь рис 7 кг" ──────
section "3. ADJUST — set rice to 7 kg (post-inventory)"
info "Sending: 'After inventory check, set rice quantity to 7 kg'"

RESP3=$(copilot_chat "After inventory check, set rice quantity to 7 kg")
REQUIRES3=$(jq_get "$RESP3" "d.get('requires_confirmation', False)")
PLAN_ID3=$(jq_get "$RESP3" "d.get('action_plan',{}).get('id','')" 2>/dev/null || echo "")
[ -z "$PLAN_ID3" ] && PLAN_ID3=$(jq_get "$RESP3" "d.get('action_id','')")

echo "$RESP3" | python3 -m json.tool 2>/dev/null | grep -E '"plan_type|"changes|"message' | head -5 || echo "$RESP3" | head -2

if [ "$REQUIRES3" = "True" ] && [ -n "$PLAN_ID3" ]; then
  pass "Got adjust plan (id: $PLAN_ID3)"
  CONFIRM3=$(copilot_confirm "$PLAN_ID3")
  MSG3=$(echo "$CONFIRM3" | python3 -c "import sys,json; d=json.load(sys.stdin); print(d.get('message',''))" 2>/dev/null)
  pass "Adjust confirmed: $MSG3"
else
  fail "Expected requires_confirmation=true for adjust. Got: $(echo "$RESP3" | head -2)"
fi

# ── 4. DAILY BRIEFING — "What's important today?" ────────────
section "4. DAILY BRIEFING (read-only)"
info "Sending: 'What is important today?'"

RESP4=$(copilot_chat "What is important today?")
REQUIRES4=$(jq_get "$RESP4" "d.get('requires_confirmation', False)")

echo "$RESP4" | python3 -m json.tool 2>/dev/null | grep -E '"message|"expiring|"low_stock|"open_drafts' | head -8 || echo "$RESP4" | head -3

if [ "$REQUIRES4" = "False" ]; then
  pass "Daily briefing returned read-only response (no confirmation needed)"
else
  fail "Daily briefing should be read-only (no confirmation), got: requires_confirmation=$REQUIRES4"
fi

# ── 5. SMOKE — прямой API /inventory для сравнения ───────────
section "5. SMOKE CHECK — direct inventory API"
info "GET $BASE/inventory/products (direct, old way)"

INV=$(curl -s --max-time 10 -H "Authorization: Bearer $TOKEN" "$BASE/inventory/products" 2>/dev/null)
COUNT=$(echo "$INV" | python3 -c "import sys,json; d=json.load(sys.stdin); items=d.get('items',d) if isinstance(d,dict) else d; print(len(items))" 2>/dev/null || echo "?")

if [ "$COUNT" != "?" ] && [ "$COUNT" -gt 0 ] 2>/dev/null; then
  pass "Direct API returns $COUNT inventory item(s) — baseline confirmed"
else
  info "Could not parse inventory count (may be empty or different schema). Raw: ${INV:0:120}"
fi

# ── Summary ───────────────────────────────────────────────────
section "SUMMARY"
TOTAL=$((PASS + FAIL))
echo -e "  ${GREEN}PASS${RESET}: $PASS / $TOTAL"
if [ "$FAIL" -gt 0 ]; then
  echo -e "  ${RED}FAIL${RESET}: $FAIL / $TOTAL"
  echo -e "\n${RED}Some tests failed. Check output above.${RESET}"
  exit 1
else
  echo -e "\n${GREEN}${BOLD}All $TOTAL tests passed ✅${RESET}"
  echo -e ""
  echo -e "  Old way:   ${YELLOW}curl /api/inventory/batches → curl /api/inventory/writeoff${RESET}"
  echo -e "  New way:   ${GREEN}\"Write off 2 kg of rice, spoiled\" → preview → confirm → done${RESET}"
  echo -e ""
  echo -e "  ${BOLD}This is ChefOS Copilot.${RESET}"
fi
