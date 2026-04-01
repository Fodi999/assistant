#!/bin/bash
# ══════════════════════════════════════════════════════════════
#  test_lab_combos.sh — ChefOS Lab Combos Pipeline Test
#  Тестирует: DishProfile → skeleton → validation → SEO → quality
#
#  Использование:
#    bash test_lab_combos.sh                          # Koyeb prod
#    bash test_lab_combos.sh http://localhost:8000    # локально
# ══════════════════════════════════════════════════════════════

BASE="${1:-https://ministerial-yetta-fodi999-c58d8823.koyeb.app}"
API="$BASE/api"
ADMIN_EMAIL="${NEXT_PUBLIC_ADMIN_EMAIL:-admin@fodi.app}"
ADMIN_PASSWORD="${NEXT_PUBLIC_ADMIN_PASSWORD:-Admin123!}"

GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
BOLD='\033[1m'
NC='\033[0m'

ok()   { echo -e "${GREEN}✅ $1${NC}"; }
fail() { echo -e "${RED}❌ $1${NC}"; }
info() { echo -e "${CYAN}   $1${NC}"; }
warn() { echo -e "${YELLOW}⚠️  $1${NC}"; }
sep()  { echo -e "${BOLD}──────────────────────────────────────────${NC}"; }

PASS=0; FAIL=0

check() {
  local label="$1" cond="$2"
  if [ "$cond" = "true" ]; then
    ok "$label"; PASS=$((PASS+1))
  else
    fail "$label"; FAIL=$((FAIL+1))
  fi
}

echo ""
echo -e "${BOLD}🧠 ChefOS Lab Combos — Pipeline Test${NC}"
echo -e "📍 API: $API"
sep

# ──────────────────────────────────────────
# 0. Health
# ──────────────────────────────────────────
echo ""
echo -e "${BOLD}0️⃣  Health check${NC}"
HEALTH=$(curl -s --max-time 10 "$BASE/health")
[ "$HEALTH" = "OK" ] && ok "Server healthy: $HEALTH" || { fail "Server unavailable"; exit 1; }

# ──────────────────────────────────────────
# 1. Admin login
# ──────────────────────────────────────────
echo ""
echo -e "${BOLD}1️⃣  Admin login${NC}"
LOGIN_RESP=$(curl -s -X POST "$API/admin/auth/login" \
  -H "Content-Type: application/json" \
  -d "{\"email\":\"$ADMIN_EMAIL\",\"password\":\"$ADMIN_PASSWORD\"}")

ADMIN_TOKEN=$(echo "$LOGIN_RESP" | python3 -c "import sys,json; print(json.load(sys.stdin).get('token',''))" 2>/dev/null)

if [ -z "$ADMIN_TOKEN" ] || [ "$ADMIN_TOKEN" = "None" ]; then
  fail "Admin login failed"
  echo "$LOGIN_RESP" | python3 -m json.tool 2>/dev/null || echo "$LOGIN_RESP"
  exit 1
fi
ok "Admin token: ${ADMIN_TOKEN:0:30}..."

# ──────────────────────────────────────────
# 2. generate-all-locales: Gemelli (паста)
# ──────────────────────────────────────────
echo ""
echo -e "${BOLD}2️⃣  generate-all-locales → Gemelli pasta (en/pl/ru/uk)${NC}"
info "Pipeline: classify → DishProfile → skeleton → Gemini → validate → SEO"
info "⏳ Может занять 30-90 секунд..."

GEN_RESP=$(curl -s --max-time 120 -w "\n__STATUS:%{http_code}" \
  -X POST "$API/admin/lab-combos/generate-all-locales" \
  -H "Authorization: Bearer $ADMIN_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "ingredients": ["pasta", "garlic", "olive oil", "parmesan"],
    "locale": "en",
    "dish_name": "Gemelli aglio olio",
    "goal": "quick_dinner",
    "meal_type": "dinner",
    "diet": null,
    "cooking_time": "30min",
    "cuisine": "italian"
  }')

HTTP_STATUS=$(echo "$GEN_RESP" | grep "__STATUS:" | cut -d: -f2)
GEN_BODY=$(echo "$GEN_RESP" | sed 's/__STATUS:.*//')

# Сохраняем в файл для передачи в python heredoc
TMPFILE=$(mktemp /tmp/lab_combo_XXXXXX.json)
echo "$GEN_BODY" > "$TMPFILE"

check "HTTP 201 Created" "$([ "$HTTP_STATUS" = "201" ] && echo true || echo false)"

PAGES_COUNT=$(python3 -c "import json; d=json.load(open('$TMPFILE')); print(len(d) if isinstance(d, list) else 0)" 2>/dev/null)
check "4 локали сгенерированы" "$([ "$PAGES_COUNT" = "4" ] && echo true || echo false)"
info "Получено страниц: $PAGES_COUNT"

# ──────────────────────────────────────────
# 3. Анализируем каждую страницу
# ──────────────────────────────────────────
echo ""
echo -e "${BOLD}3️⃣  Анализ качества каждой локали${NC}"

python3 << PYEOF
import json

try:
    pages = json.load(open("$TMPFILE"))
except Exception as e:
    print(f"  ❌ Не удалось распарсить JSON: {e}")
    exit(1)

if not isinstance(pages, list):
    print(f"  ❌ Ожидался список, получено: {type(pages)}")
    sys.exit(1)

locales_seen = []
all_ok = True

for p in pages:
    loc   = p.get("locale", "??")
    slug  = p.get("slug", "")
    title = p.get("title", "")
    qs    = p.get("quality_score", 0)
    status= p.get("status", "")
    intro = p.get("intro", "")
    how   = p.get("how_to_cook", [])
    faq   = p.get("faq", [])
    sr    = p.get("smart_response", {})
    cal   = p.get("calories_per_serving", 0)
    desc  = p.get("description", "")
    locales_seen.append(loc)

    steps_count = len(how) if isinstance(how, list) else 0
    faq_count   = len(faq) if isinstance(faq, list) else 0
    has_sr      = bool(sr and sr != {} and sr != [])

    ok_mark   = "✅"
    warn_mark = "⚠️ "
    fail_mark = "❌"

    print(f"\n  [{loc.upper()}] {title}")
    print(f"  {'─'*50}")
    print(f"  slug          : {slug}")
    print(f"  quality_score : {ok_mark if qs >= 70 else warn_mark if qs >= 50 else fail_mark} {qs}/100")
    print(f"  status        : {ok_mark if status == 'draft' else warn_mark} {status}")
    print(f"  calories/svg  : {ok_mark if cal > 0 else warn_mark} {cal:.0f} kcal")
    print(f"  intro length  : {ok_mark if len(intro) > 100 else warn_mark} {len(intro)} chars")
    print(f"  description   : {ok_mark if len(desc) > 60 else warn_mark} {len(desc)} chars")
    print(f"  how_to_cook   : {ok_mark if steps_count >= 3 else warn_mark} {steps_count} steps")
    print(f"  faq           : {ok_mark if faq_count >= 2 else warn_mark} {faq_count} items")
    print(f"  smart_response: {ok_mark if has_sr else fail_mark} {'present' if has_sr else 'MISSING'}")

    if qs < 50:
        all_ok = False

print(f"\n  Локали: {sorted(locales_seen)}")
expected = sorted(["en","pl","ru","uk"])
if sorted(locales_seen) == expected:
    print(f"  ✅ Все 4 локали присутствуют: {expected}")
else:
    print(f"  ❌ Ожидались {expected}, получены {sorted(locales_seen)}")
    all_ok = False

print(f"\n  {'✅ Качество OK' if all_ok else '⚠️  Есть страницы с качеством < 50'}")
PYEOF

# ──────────────────────────────────────────
# 4. Проверка структуры skeleton (how_to_cook)
# ──────────────────────────────────────────
echo ""
sep
echo -e "${BOLD}4️⃣  Skeleton: структура how_to_cook (EN)${NC}"

python3 << PYEOF
import json

pages = json.load(open("$TMPFILE"))
en = next((p for p in pages if p.get("locale") == "en"), None)
if not en:
    print("  ❌ EN страница не найдена")
    sys.exit(1)

how = en.get("how_to_cook", [])
print(f"  Шагов в рецепте: {len(how)}")
print()

if isinstance(how, list):
    for i, step in enumerate(how):
        if isinstance(step, dict):
            t = step.get("title", step.get("step", f"Step {i+1}"))
            d = step.get("description", step.get("text", ""))
            tip = step.get("tip", step.get("chef_tip", ""))
            dur = step.get("duration_minutes", step.get("time", ""))
            tech = step.get("technique", "")
            print(f"  Step {i+1}: {t}")
            if d:    print(f"    📝 {d[:120]}{'...' if len(d)>120 else ''}")
            if tech: print(f"    🔧 technique: {tech}")
            if dur:  print(f"    ⏱  duration: {dur}")
            if tip:  print(f"    💡 tip: {tip[:80]}")
        else:
            print(f"  Step {i+1}: {str(step)[:100]}")
else:
    print(f"  Raw: {str(how)[:300]}")
PYEOF

# ──────────────────────────────────────────
# 5. SEO-поля (EN)
# ──────────────────────────────────────────
echo ""
sep
echo -e "${BOLD}5️⃣  SEO-поля (EN)${NC}"

python3 << PYEOF
import json

pages = json.load(open("$TMPFILE"))
en = next((p for p in pages if p.get("locale") == "en"), None)
if not en:
    print("  ❌ EN не найден"); sys.exit(1)

fields = ["title","description","h1","intro","why_it_works"]
for f in fields:
    v = en.get(f, "")
    l = len(v)
    mark = "✅" if l > 30 else "⚠️ "
    print(f"  {mark} {f:20s}: {l:4d} chars | {v[:80]}{'...' if l>80 else ''}")

print()
# FAQ
faq = en.get("faq", [])
print(f"  FAQ ({len(faq)} вопросов):")
for item in faq[:3]:
    if isinstance(item, dict):
        q = item.get("question", item.get("q", ""))
        print(f"    Q: {q[:80]}")
PYEOF

# ──────────────────────────────────────────
# 6. Nutrition (pre-calculated)
# ──────────────────────────────────────────
echo ""
sep
echo -e "${BOLD}6️⃣  Nutrition (USDA pre-calculated, EN)${NC}"

python3 << PYEOF
import json

pages = json.load(open("$TMPFILE"))
en = next((p for p in pages if p.get("locale") == "en"), None)
if not en: sys.exit(1)

fields = [
    ("total_weight_g",      "g",    "> 0"),
    ("servings_count",      "srv",  "> 0"),
    ("calories_per_serving","kcal", "> 0"),
    ("protein_per_serving", "g",    "> 0"),
    ("fat_per_serving",     "g",    "> 0"),
    ("carbs_per_serving",   "g",    "> 0"),
    ("fiber_per_serving",   "g",    ">= 0"),
]
all_ok = True
for key, unit, _ in fields:
    v = en.get(key, 0)
    mark = "✅" if v > 0 else "⚠️ "
    if v == 0: all_ok = False
    print(f"  {mark} {key:30s}: {v} {unit}")

print()
print(f"  {'✅ Все нутриенты рассчитаны' if all_ok else '⚠️  Часть нутриентов = 0'}")

si = en.get("structured_ingredients", [])
count = len(si) if isinstance(si, list) else 0
print(f"  ✅ structured_ingredients: {count} позиций")
PYEOF

# ──────────────────────────────────────────
# 7. Сохраняем ID первой страницы и публикуем
# ──────────────────────────────────────────
echo ""
sep
echo -e "${BOLD}7️⃣  Publish EN страницы${NC}"

PAGE_ID=$(python3 -c "
import json
pages=json.load(open('$TMPFILE'))
en=next((p for p in pages if p.get('locale')=='en'),None)
print(en['id'] if en else '')
" 2>/dev/null)

if [ -z "$PAGE_ID" ]; then
  warn "Не удалось получить ID страницы"
else
  info "Page ID: $PAGE_ID"
  PUB_RESP=$(curl -s -w "\n__STATUS:%{http_code}" \
    -X POST "$API/admin/lab-combos/$PAGE_ID/publish" \
    -H "Authorization: Bearer $ADMIN_TOKEN")
  PUB_STATUS=$(echo "$PUB_RESP" | grep "__STATUS:" | cut -d: -f2)
  PUB_BODY=$(echo "$PUB_RESP" | sed 's/__STATUS:.*//')
  check "Publish HTTP 200" "$([ "$PUB_STATUS" = "200" ] && echo true || echo false)"
  PUB_FIELD=$(echo "$PUB_BODY" | python3 -c "import sys,json; print(json.load(sys.stdin).get('status',''))" 2>/dev/null)
  check "status = published" "$([ "$PUB_FIELD" = "published" ] && echo true || echo false)"
  info "published_at: $(echo "$PUB_BODY" | python3 -c "import sys,json; print(json.load(sys.stdin).get('published_at',''))" 2>/dev/null)"
fi

# ──────────────────────────────────────────
# 8. Public endpoint (frontend)
# ──────────────────────────────────────────
echo ""
sep
echo -e "${BOLD}8️⃣  Public endpoint (blog frontend)${NC}"

SLUG=$(python3 -c "
import json
pages=json.load(open('$TMPFILE'))
en=next((p for p in pages if p.get('locale')=='en'),None)
print(en.get('slug','') if en else '')
" 2>/dev/null)

if [ -n "$SLUG" ]; then
  info "Slug: $SLUG"
  PUB_PAGE=$(curl -s -w "\n__STATUS:%{http_code}" \
    "$BASE/public/lab-combos/$SLUG?locale=en")
  PUB_PAGE_STATUS=$(echo "$PUB_PAGE" | grep "__STATUS:" | cut -d: -f2)
  check "Public GET HTTP 200" "$([ "$PUB_PAGE_STATUS" = "200" ] && echo true || echo false)"
fi

# ──────────────────────────────────────────
# 9. Cleanup — удаляем все 4 страницы
# ──────────────────────────────────────────
echo ""
sep
echo -e "${BOLD}9️⃣  Cleanup (удаляем все 4 страницы)${NC}"

python3 -c "
import json
pages=json.load(open('$TMPFILE'))
for p in pages:
    print(p['id'])
" 2>/dev/null | while read ID; do
  DEL=$(curl -s -w "%{http_code}" -o /dev/null \
    -X DELETE "$API/admin/lab-combos/$ID" \
    -H "Authorization: Bearer $ADMIN_TOKEN")
  [ "$DEL" = "204" ] && ok "Удалён $ID" || warn "Статус удаления $ID: $DEL"
done

rm -f "$TMPFILE"

# ──────────────────────────────────────────
# Итог
# ──────────────────────────────────────────
echo ""
sep
echo -e "${BOLD}📊 ИТОГ${NC}"
echo -e "  ${GREEN}PASS: $PASS${NC}"
echo -e "  ${RED}FAIL: $FAIL${NC}"
if [ "$FAIL" -eq 0 ]; then
  echo -e "\n${GREEN}${BOLD}🚀 ChefOS Pipeline — все проверки прошли!${NC}\n"
else
  echo -e "\n${YELLOW}${BOLD}⚠️  Есть ошибки: $FAIL${NC}\n"
fi
