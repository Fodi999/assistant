#!/usr/bin/env bash
# ═══════════════════════════════════════════════════════════════════════════════
#  🧪 Decision Test — тест продукта через LIVE Koyeb API
#
#  Проверяет: "помог ли продукт пользователю?"
#
#  Для каждого запроса проверяем:
#    1. Есть ли ответ (не пустой text)
#    2. Правильный ли intent
#    3. Есть ли рецепт (recipe card) когда ожидаем
#    4. Kcal в рамках цели
#    5. Белок достаточный
#    6. Есть шаги (steps)
#    7. Есть explain block (🎯 📊)
#
#  Usage: bash decision_test_koyeb.sh
# ═══════════════════════════════════════════════════════════════════════════════

set -euo pipefail

BASE="https://ministerial-yetta-fodi999-c58d8823.koyeb.app"
ENDPOINT="$BASE/public/chat"
PASS=0
FAIL=0
TOTAL=0

# ── Colors ────────────────────────────────────────────────────────────────────
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[0;33m'
CYAN='\033[0;36m'
BOLD='\033[1m'
NC='\033[0m'

# ── Helper: send chat request ────────────────────────────────────────────────
chat() {
    local input="$1"
    curl -s -X POST "$ENDPOINT" \
        -H "Content-Type: application/json" \
        -d "{\"input\": \"$input\"}" \
        --max-time 30
}

# ── Helper: assert condition ─────────────────────────────────────────────────
assert() {
    local label="$1"
    local condition="$2"  # "true" or "false"
    local detail="$3"
    TOTAL=$((TOTAL + 1))
    if [ "$condition" = "true" ]; then
        PASS=$((PASS + 1))
        echo -e "  ${GREEN}✅ $label${NC} — $detail"
    else
        FAIL=$((FAIL + 1))
        echo -e "  ${RED}❌ $label${NC} — $detail"
    fi
}

# ── Helper: check recipe response ────────────────────────────────────────────
check_recipe_response() {
    local label="$1"
    local input="$2"
    local expect_intent="$3"
    local max_kcal="$4"
    local min_protein="$5"

    echo ""
    echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${BOLD}  🧪 $label${NC}"
    echo -e "  📝 input: \"$input\""
    echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"

    local json
    json=$(chat "$input")

    if [ -z "$json" ] || [ "$json" = "null" ]; then
        echo -e "  ${RED}💀 Нет ответа от сервера!${NC}"
        FAIL=$((FAIL + 5))
        TOTAL=$((TOTAL + 5))
        return
    fi

    # Extract fields via jq
    local text intent timing_ms has_recipe
    local per_serving_kcal per_serving_protein steps_count
    local goal auto_fixes_count

    text=$(echo "$json" | jq -r '.text // ""')
    intent=$(echo "$json" | jq -r '.intent // "unknown"')
    timing_ms=$(echo "$json" | jq -r '.timing_ms // 0')
    lang=$(echo "$json" | jq -r '.lang // "?"')

    # Check if recipe card exists
    has_recipe=$(echo "$json" | jq 'any(.cards[]?; .type == "recipe")')
    [ "$has_recipe" = "null" ] && has_recipe="false"

    # Extract recipe fields if present
    per_serving_kcal=$(echo "$json" | jq '[.cards[]? | select(.type == "recipe") | .per_serving_kcal] | first // 0')
    per_serving_protein=$(echo "$json" | jq '[.cards[]? | select(.type == "recipe") | .per_serving_protein] | first // 0')
    steps_count=$(echo "$json" | jq '[.cards[]? | select(.type == "recipe") | .steps | length] | first // 0')
    goal=$(echo "$json" | jq -r '[.cards[]? | select(.type == "recipe") | .goal] | first // "none"')
    auto_fixes_count=$(echo "$json" | jq '[.cards[]? | select(.type == "recipe") | .auto_fixes | length] | first // 0')

    echo -e "  ⏱  ${timing_ms}ms | lang=$lang | intent=$intent"

    # ── 1. Есть ответ ──
    local text_ok="false"
    [ -n "$text" ] && [ "$text" != "null" ] && [ ${#text} -gt 5 ] && text_ok="true"
    assert "Есть text" "$text_ok" "${#text} chars"

    # ── 2. Правильный intent ──
    local intent_ok="false"
    [ "$intent" = "$expect_intent" ] && intent_ok="true"
    assert "Intent = $expect_intent" "$intent_ok" "got: $intent"

    # ── 3. Есть рецепт ──
    if [ "$expect_intent" = "recipe_help" ]; then
        assert "Есть recipe card" "$has_recipe" "has_recipe=$has_recipe"

        if [ "$has_recipe" = "true" ]; then
            # ── 4. Kcal в рамках цели ──
            local kcal_ok="false"
            [ "$per_serving_kcal" -le "$max_kcal" ] 2>/dev/null && kcal_ok="true"
            assert "Kcal ≤ $max_kcal" "$kcal_ok" "${per_serving_kcal} kcal/serv"

            # ── 5. Белок достаточный ──
            local protein_ok="false"
            local protein_int=${per_serving_protein%.*}  # truncate decimal
            [ "${protein_int:-0}" -ge "$min_protein" ] 2>/dev/null && protein_ok="true"
            assert "Protein ≥ ${min_protein}g" "$protein_ok" "${per_serving_protein}g/serv"

            # ── 6. Есть шаги ──
            local steps_ok="false"
            [ "$steps_count" -ge 2 ] 2>/dev/null && steps_ok="true"
            assert "Steps ≥ 2" "$steps_ok" "$steps_count steps"

            # ── 7. Goal tag ──
            echo -e "  📊 goal=$goal | auto_fixes=$auto_fixes_count"
        fi
    fi

    # ── 8. Explain block (🎯 📊 в тексте) ──
    if [ "$has_recipe" = "true" ]; then
        # 🎯 only shown for non-balanced goals (weight_loss, high_protein, etc.)
        if [ "$goal" != "balanced" ] && [ "$goal" != "none" ] && [ "$goal" != "null" ]; then
            local has_goal_emoji="false"
            echo "$text" | grep -q "🎯" && has_goal_emoji="true"
            assert "Text содержит 🎯 (goal=$goal)" "$has_goal_emoji" ""
        else
            echo -e "  📝 goal=$goal — 🎯 not expected for balanced"
        fi

        local has_macros="false"
        echo "$text" | grep -q "📊" && has_macros="true"
        assert "Text содержит 📊" "$has_macros" ""
    fi

    # Print first 200 chars of response
    echo -e "  💬 ${YELLOW}${text:0:200}${NC}"
}

# ── Helper: check product response ───────────────────────────────────────────
check_product_response() {
    local label="$1"
    local input="$2"
    local expect_intent="$3"

    echo ""
    echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${BOLD}  🧪 $label${NC}"
    echo -e "  📝 input: \"$input\""
    echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"

    local json
    json=$(chat "$input")

    if [ -z "$json" ] || [ "$json" = "null" ]; then
        echo -e "  ${RED}💀 Нет ответа от сервера!${NC}"
        FAIL=$((FAIL + 2))
        TOTAL=$((TOTAL + 2))
        return
    fi

    local text intent timing_ms lang cards_count
    text=$(echo "$json" | jq -r '.text // ""')
    intent=$(echo "$json" | jq -r '.intent // "unknown"')
    timing_ms=$(echo "$json" | jq -r '.timing_ms // 0')
    lang=$(echo "$json" | jq -r '.lang // "?"')
    cards_count=$(echo "$json" | jq '.cards | length // 0')

    echo -e "  ⏱  ${timing_ms}ms | lang=$lang | intent=$intent | cards=$cards_count"

    local text_ok="false"
    [ -n "$text" ] && [ "$text" != "null" ] && [ ${#text} -gt 5 ] && text_ok="true"
    assert "Есть text" "$text_ok" "${#text} chars"

    local intent_ok="false"
    [ "$intent" = "$expect_intent" ] && intent_ok="true"
    assert "Intent = $expect_intent" "$intent_ok" "got: $intent"

    local has_cards="false"
    [ "$cards_count" -gt 0 ] 2>/dev/null && has_cards="true"
    # Greeting and some intents don't return cards — that's OK
    if [ "$expect_intent" != "greeting" ]; then
        assert "Есть карточки" "$has_cards" "$cards_count cards"
    else
        echo -e "  📝 cards=$cards_count (greeting — cards optional)"
    fi

    echo -e "  💬 ${YELLOW}${text:0:200}${NC}"
}

# ═══════════════════════════════════════════════════════════════════════════════
echo ""
echo -e "${BOLD}╔═══════════════════════════════════════════════════════════════╗${NC}"
echo -e "${BOLD}║   🚀 DECISION TEST — Live Koyeb E2E                         ║${NC}"
echo -e "${BOLD}║   Тестируем: помог ли продукт пользователю?                  ║${NC}"
echo -e "${BOLD}╚═══════════════════════════════════════════════════════════════╝${NC}"
echo ""

# ── 0. Health check ──────────────────────────────────────────────────────────
echo -e "${BOLD}📡 Health check: $BASE${NC}"
HTTP_CODE=$(curl -s -o /dev/null -w "%{http_code}" "$BASE/health" --max-time 10 || echo "000")
if [ "$HTTP_CODE" = "200" ]; then
    echo -e "  ${GREEN}✅ Server is alive (HTTP $HTTP_CODE)${NC}"
else
    echo -e "  ${RED}❌ Server unreachable (HTTP $HTTP_CODE)${NC}"
    echo -e "  ${RED}Aborting tests.${NC}"
    exit 1
fi

# ═══════════════════════════════════════════════════════════════════════════════
#  СЦЕНАРИЙ 1: ПОХУДЕНИЕ — рецепт (RU)
# ═══════════════════════════════════════════════════════════════════════════════
check_recipe_response \
    "🇷🇺 Похудение + рецепт" \
    "приготовь лёгкий суп для похудения" \
    "recipe_help" \
    600 \
    5

# ═══════════════════════════════════════════════════════════════════════════════
#  СЦЕНАРИЙ 2: ПОХУДЕНИЕ — продукт (RU)
# ═══════════════════════════════════════════════════════════════════════════════
check_product_response \
    "🇷🇺 Похудение → продукты" \
    "хочу похудеть" \
    "healthy_product"

# ═══════════════════════════════════════════════════════════════════════════════
#  СЦЕНАРИЙ 3: НАБОР МАССЫ — рецепт (EN)
# ═══════════════════════════════════════════════════════════════════════════════
check_recipe_response \
    "🇬🇧 Muscle gain + recipe" \
    "make a high protein salad for muscle gain" \
    "recipe_help" \
    900 \
    15

# ═══════════════════════════════════════════════════════════════════════════════
#  СЦЕНАРИЙ 4: НАБОР МАССЫ — продукт (EN)
# ═══════════════════════════════════════════════════════════════════════════════
check_product_response \
    "🇬🇧 Muscle gain → products" \
    "I want to gain muscle mass" \
    "healthy_product"

# ═══════════════════════════════════════════════════════════════════════════════
#  СЦЕНАРИЙ 5: БЕЗ ГЛЮТЕНА — рецепт (PL)
# ═══════════════════════════════════════════════════════════════════════════════
check_recipe_response \
    "🇵🇱 Bezglutenowy przepis" \
    "ugotuj bezglutenową zupę" \
    "recipe_help" \
    800 \
    10

# ═══════════════════════════════════════════════════════════════════════════════
#  СЦЕНАРИЙ 6: БЕЗ ЛАКТОЗЫ — рецепт (UK)
# ═══════════════════════════════════════════════════════════════════════════════
check_recipe_response \
    "🇺🇦 Без лактози + рецепт" \
    "приготуй суп без лактози" \
    "recipe_help" \
    800 \
    10

# ═══════════════════════════════════════════════════════════════════════════════
#  СЦЕНАРИЙ 7: HIGH PROTEIN — рецепт (EN)
# ═══════════════════════════════════════════════════════════════════════════════
check_recipe_response \
    "🇬🇧 High protein recipe" \
    "make a high protein chicken stir-fry" \
    "recipe_help" \
    900 \
    20

# ═══════════════════════════════════════════════════════════════════════════════
#  СЦЕНАРИЙ 8: КЕТО / LOW CARB — рецепт (RU)
# ═══════════════════════════════════════════════════════════════════════════════
check_recipe_response \
    "🇷🇺 Кето рецепт" \
    "сделай кето-салат с курицей" \
    "recipe_help" \
    700 \
    15

# ═══════════════════════════════════════════════════════════════════════════════
#  СЦЕНАРИЙ 9: Что приготовить? → MealIdea (RU)
# ═══════════════════════════════════════════════════════════════════════════════
check_product_response \
    "🇷🇺 Что приготовить на ужин?" \
    "что приготовить на ужин?" \
    "meal_idea"

# ═══════════════════════════════════════════════════════════════════════════════
#  СЦЕНАРИЙ 10: Просто приветствие
# ═══════════════════════════════════════════════════════════════════════════════
check_product_response \
    "🖐 Приветствие" \
    "Привет!" \
    "greeting"

# ═══════════════════════════════════════════════════════════════════════════════
#  РЕЗУЛЬТАТЫ
# ═══════════════════════════════════════════════════════════════════════════════
echo ""
echo -e "${BOLD}╔═══════════════════════════════════════════════════════════════╗${NC}"
echo -e "${BOLD}║   📊 ИТОГИ DECISION TEST                                    ║${NC}"
echo -e "${BOLD}╠═══════════════════════════════════════════════════════════════╣${NC}"
echo -e "${BOLD}║   Total:  $TOTAL                                               ║${NC}"
echo -e "${BOLD}║   ${GREEN}Passed: $PASS${NC}${BOLD}                                               ║${NC}"
if [ "$FAIL" -gt 0 ]; then
echo -e "${BOLD}║   ${RED}Failed: $FAIL${NC}${BOLD}                                               ║${NC}"
else
echo -e "${BOLD}║   Failed: 0                                                  ║${NC}"
fi
echo -e "${BOLD}╚═══════════════════════════════════════════════════════════════╝${NC}"

if [ "$FAIL" -eq 0 ]; then
    echo ""
    echo -e "${GREEN}${BOLD}  🎉 ВСЕ ТЕСТЫ ПРОЙДЕНЫ — ЭТО ПРОДУКТ!${NC}"
    echo ""
    exit 0
else
    echo ""
    echo -e "${RED}${BOLD}  ⚠️  $FAIL из $TOTAL проверок не пройдены${NC}"
    echo ""
    exit 1
fi
