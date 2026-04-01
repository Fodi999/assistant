#!/bin/bash

# test_recipe_create.sh — Тест создания рецепта через /api/recipes/v2
# Запускать при работающем сервере (локально или на Koyeb)
#
# Использование:
#   bash test_recipe_create.sh                          # Koyeb prod
#   bash test_recipe_create.sh http://localhost:8000/api # локально

API_URL="${1:-https://ministerial-yetta-fodi999-c58d8823.koyeb.app/api}"
ADMIN_EMAIL="${NEXT_PUBLIC_ADMIN_EMAIL:-admin@fodi.app}"
ADMIN_PASSWORD="${NEXT_PUBLIC_ADMIN_PASSWORD:-Admin123!}"
USER_EMAIL="recipe_tester_$(date +%s)@example.com"
USER_PASSWORD="Password123!"

echo "🍽️  Тест создания рецепта (Recipe V2)"
echo "📍 API: $API_URL"
echo "========================================"

# ──────────────────────────────────────────
# 0. Ждём пока сервер ответит на /health
# ──────────────────────────────────────────
echo ""
echo "0️⃣  Проверяем доступность сервера..."
HEALTH=$(curl -s --max-time 10 "${API_URL%/api}/health" 2>/dev/null)
if [ -z "$HEALTH" ]; then
  echo "❌ Сервер недоступен: ${API_URL%/api}/health"
  exit 1
fi
echo "✅ Сервер отвечает: $HEALTH"

# ──────────────────────────────────────────
# 1. Регистрируем обычного пользователя (для /api/recipes/v2 нужен user-токен)
# ──────────────────────────────────────────
echo ""
echo "1️⃣  Регистрируем тестового пользователя: $USER_EMAIL"
AUTH_RESPONSE=$(curl -s -X POST "$API_URL/auth/register" \
  -H "Content-Type: application/json" \
  -d "{
    \"email\": \"$USER_EMAIL\",
    \"password\": \"$USER_PASSWORD\",
    \"first_name\": \"Recipe\",
    \"last_name\": \"Tester\",
    \"language\": \"ru\",
    \"restaurant_name\": \"Test Kitchen\"
  }")

TOKEN=$(echo "$AUTH_RESPONSE" | python3 -c "import sys,json; d=json.load(sys.stdin); print(d.get('access_token',''))" 2>/dev/null)

if [ -z "$TOKEN" ] || [ "$TOKEN" = "None" ]; then
  echo "  ⚠️  Регистрация не удалась — пробуем логин..."
  AUTH_RESPONSE=$(curl -s -X POST "$API_URL/auth/login" \
    -H "Content-Type: application/json" \
    -d "{\"email\":\"$USER_EMAIL\",\"password\":\"$USER_PASSWORD\"}")
  TOKEN=$(echo "$AUTH_RESPONSE" | python3 -c "import sys,json; d=json.load(sys.stdin); print(d.get('access_token',''))" 2>/dev/null)
fi

if [ -z "$TOKEN" ] || [ "$TOKEN" = "None" ]; then
  echo "❌ Не удалось получить токен. Ответ:"
  echo "$AUTH_RESPONSE" | python3 -m json.tool 2>/dev/null || echo "$AUTH_RESPONSE"
  exit 1
fi
echo "✅ User-токен получен: ${TOKEN:0:30}..."

# ──────────────────────────────────────────
# 2. Поиск ингредиентов в каталоге
# ──────────────────────────────────────────
echo ""
echo "2️⃣  Ищем ингредиенты в каталоге..."

ING1_RESP=$(curl -s "$API_URL/catalog/ingredients?q=chicken" \
  -H "Authorization: Bearer $TOKEN")
ING1_ID=$(echo "$ING1_RESP" | python3 -c "
import sys, json
d = json.load(sys.stdin)
items = d.get('ingredients', d.get('data', d if isinstance(d, list) else []))
print(items[0]['id'] if items else '')
" 2>/dev/null)

ING2_RESP=$(curl -s "$API_URL/catalog/ingredients?q=garlic" \
  -H "Authorization: Bearer $TOKEN")
ING2_ID=$(echo "$ING2_RESP" | python3 -c "
import sys, json
d = json.load(sys.stdin)
items = d.get('ingredients', d.get('data', d if isinstance(d, list) else []))
print(items[0]['id'] if items else '')
" 2>/dev/null)

if [ -z "$ING1_ID" ] || [ "$ING1_ID" = "None" ]; then
  echo "⚠️  Chicken не найдено — пробуем 'flour'..."
  ING1_RESP=$(curl -s "$API_URL/catalog/ingredients?q=flour" \
    -H "Authorization: Bearer $TOKEN")
  ING1_ID=$(echo "$ING1_RESP" | python3 -c "
import sys, json
d = json.load(sys.stdin)
items = d.get('ingredients', d.get('data', d if isinstance(d, list) else []))
print(items[0]['id'] if items else '')
" 2>/dev/null)
fi

if [ -z "$ING2_ID" ] || [ "$ING2_ID" = "None" ]; then
  echo "⚠️  Garlic не найдено — пробуем 'onion'..."
  ING2_RESP=$(curl -s "$API_URL/catalog/ingredients?q=onion" \
    -H "Authorization: Bearer $TOKEN")
  ING2_ID=$(echo "$ING2_RESP" | python3 -c "
import sys, json
d = json.load(sys.stdin)
items = d.get('ingredients', d.get('data', d if isinstance(d, list) else []))
print(items[0]['id'] if items else '')
" 2>/dev/null)
fi

echo "  Ingredient 1 ID: $ING1_ID"
echo "  Ingredient 2 ID: $ING2_ID"

if [ -z "$ING1_ID" ] || [ -z "$ING2_ID" ]; then
  echo "❌ Не удалось найти ингредиенты в каталоге. Проверьте каталог."
  exit 1
fi

# ──────────────────────────────────────────
# 3. Создание рецепта — минимальный (без ингредиентов)
# ──────────────────────────────────────────
echo ""
echo "3️⃣  Создаём рецепт БЕЗ ингредиентов (валидация)..."
EMPTY_RESP=$(curl -s -X POST "$API_URL/recipes/v2" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Test Empty Recipe",
    "instructions": "Just a test.",
    "language": "ru",
    "servings": 0,
    "ingredients": []
  }')
echo "  Ответ (ожидаем ошибку servings=0):"
echo "$EMPTY_RESP" | python3 -m json.tool 2>/dev/null || echo "  $EMPTY_RESP"

# ──────────────────────────────────────────
# 4. Создание рецепта — корректный
# ──────────────────────────────────────────
echo ""
echo "4️⃣  Создаём рецепт гемени (Gemeni) с ингредиентами..."
CREATE_RESP=$(curl -s -w "\n__HTTP_STATUS:%{http_code}" -X POST "$API_URL/recipes/v2" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d "{
    \"name\": \"Гемени — тест рецепта\",
    \"instructions\": \"1. Подготовить ингредиенты.\\n2. Смешать всё вместе.\\n3. Запечь при 180°C — 30 минут.\",
    \"language\": \"ru\",
    \"servings\": 2,
    \"image_url\": null,
    \"ingredients\": [
      {
        \"catalog_ingredient_id\": \"$ING1_ID\",
        \"quantity\": \"0.5\",
        \"unit\": \"kg\"
      },
      {
        \"catalog_ingredient_id\": \"$ING2_ID\",
        \"quantity\": \"3\",
        \"unit\": \"cloves\"
      }
    ]
  }")

HTTP_STATUS=$(echo "$CREATE_RESP" | grep "__HTTP_STATUS:" | cut -d: -f2)
BODY=$(echo "$CREATE_RESP" | sed 's/__HTTP_STATUS:.*//')

echo "  HTTP Status: $HTTP_STATUS"
echo "  Ответ:"
echo "$BODY" | python3 -m json.tool 2>/dev/null || echo "  $BODY"

RECIPE_ID=$(echo "$BODY" | python3 -c "import sys,json; d=json.load(sys.stdin); print(d.get('id',''))" 2>/dev/null)

if [ "$HTTP_STATUS" != "201" ] || [ -z "$RECIPE_ID" ]; then
  echo "❌ Создание рецепта не удалось (status=$HTTP_STATUS)"
  exit 1
fi
echo "✅ Рецепт создан! ID: $RECIPE_ID"

# ──────────────────────────────────────────
# 5. Получение рецепта по ID
# ──────────────────────────────────────────
echo ""
echo "5️⃣  Получаем рецепт по ID: $RECIPE_ID"
GET_RESP=$(curl -s -w "\n__HTTP_STATUS:%{http_code}" "$API_URL/recipes/v2/$RECIPE_ID" \
  -H "Authorization: Bearer $TOKEN")

HTTP_STATUS=$(echo "$GET_RESP" | grep "__HTTP_STATUS:" | cut -d: -f2)
BODY=$(echo "$GET_RESP" | sed 's/__HTTP_STATUS:.*//')

echo "  HTTP Status: $HTTP_STATUS"
echo "$BODY" | python3 -m json.tool 2>/dev/null || echo "  $BODY"

[ "$HTTP_STATUS" = "200" ] && echo "✅ Рецепт получен" || echo "❌ Ошибка получения рецепта"

# ──────────────────────────────────────────
# 6. Список рецептов
# ──────────────────────────────────────────
echo ""
echo "6️⃣  Получаем список рецептов пользователя..."
LIST_RESP=$(curl -s -w "\n__HTTP_STATUS:%{http_code}" "$API_URL/recipes/v2" \
  -H "Authorization: Bearer $TOKEN")

HTTP_STATUS=$(echo "$LIST_RESP" | grep "__HTTP_STATUS:" | cut -d: -f2)
BODY=$(echo "$LIST_RESP" | sed 's/__HTTP_STATUS:.*//')

COUNT=$(echo "$BODY" | python3 -c "import sys,json; d=json.load(sys.stdin); print(len(d) if isinstance(d, list) else 'N/A')" 2>/dev/null)
echo "  HTTP Status: $HTTP_STATUS  |  Рецептов в ответе: $COUNT"
[ "$HTTP_STATUS" = "200" ] && echo "✅ Список получен" || echo "❌ Ошибка"

# ──────────────────────────────────────────
# 7. Обновление рецепта
# ──────────────────────────────────────────
echo ""
echo "7️⃣  Обновляем рецепт (PUT /recipes/v2/$RECIPE_ID)..."
UPD_RESP=$(curl -s -w "\n__HTTP_STATUS:%{http_code}" -X PUT "$API_URL/recipes/v2/$RECIPE_ID" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d "{
    \"name\": \"Гемени — обновлённый рецепт\",
    \"instructions\": \"1. Подготовить ингредиенты.\\n2. Смешать.\\n3. Запечь при 190°C — 25 минут.\",
    \"language\": \"ru\",
    \"servings\": 4,
    \"image_url\": null,
    \"ingredients\": [
      {
        \"catalog_ingredient_id\": \"$ING1_ID\",
        \"quantity\": \"1\",
        \"unit\": \"kg\"
      }
    ]
  }")

HTTP_STATUS=$(echo "$UPD_RESP" | grep "__HTTP_STATUS:" | cut -d: -f2)
BODY=$(echo "$UPD_RESP" | sed 's/__HTTP_STATUS:.*//')
echo "  HTTP Status: $HTTP_STATUS"
echo "$BODY" | python3 -c "import sys,json; d=json.load(sys.stdin); print(f'  name={d.get(\"name\")}, servings={d.get(\"servings\")}, status={d.get(\"status\")}')" 2>/dev/null
[ "$HTTP_STATUS" = "200" ] && echo "✅ Рецепт обновлён" || echo "❌ Ошибка обновления"

# ──────────────────────────────────────────
# 8. Публикация рецепта
# ──────────────────────────────────────────
echo ""
echo "8️⃣  Публикуем рецепт (POST /recipes/v2/$RECIPE_ID/publish)..."
PUB_RESP=$(curl -s -w "\n__HTTP_STATUS:%{http_code}" -X POST "$API_URL/recipes/v2/$RECIPE_ID/publish" \
  -H "Authorization: Bearer $TOKEN")

HTTP_STATUS=$(echo "$PUB_RESP" | grep "__HTTP_STATUS:" | cut -d: -f2)
BODY=$(echo "$PUB_RESP" | sed 's/__HTTP_STATUS:.*//')
echo "  HTTP Status: $HTTP_STATUS"
echo "$BODY" | python3 -c "import sys,json; d=json.load(sys.stdin); print(f'  status={d.get(\"status\")}, is_public={d.get(\"is_public\")}')" 2>/dev/null
[ "$HTTP_STATUS" = "200" ] && echo "✅ Рецепт опубликован" || echo "❌ Ошибка публикации"

# ──────────────────────────────────────────
# 9. AI Insights
# ──────────────────────────────────────────
echo ""
echo "9️⃣  Запрашиваем AI Insights для рецепта (GET /recipes/v2/$RECIPE_ID/insights/ru)..."
INSIGHTS_RESP=$(curl -s -w "\n__HTTP_STATUS:%{http_code}" \
  "$API_URL/recipes/v2/$RECIPE_ID/insights/ru" \
  -H "Authorization: Bearer $TOKEN")

HTTP_STATUS=$(echo "$INSIGHTS_RESP" | grep "__HTTP_STATUS:" | cut -d: -f2)
BODY=$(echo "$INSIGHTS_RESP" | sed 's/__HTTP_STATUS:.*//')
echo "  HTTP Status: $HTTP_STATUS"
echo "$BODY" | python3 -m json.tool 2>/dev/null || echo "  $BODY"

# ──────────────────────────────────────────
# 10. Удаление рецепта (cleanup)
# ──────────────────────────────────────────
echo ""
echo "🔟  Удаляем рецепт (DELETE /recipes/v2/$RECIPE_ID)..."
DEL_RESP=$(curl -s -w "\n__HTTP_STATUS:%{http_code}" -X DELETE "$API_URL/recipes/v2/$RECIPE_ID" \
  -H "Authorization: Bearer $TOKEN")

HTTP_STATUS=$(echo "$DEL_RESP" | grep "__HTTP_STATUS:" | cut -d: -f2)
echo "  HTTP Status: $HTTP_STATUS"
[ "$HTTP_STATUS" = "204" ] || [ "$HTTP_STATUS" = "200" ] && echo "✅ Рецепт удалён" || echo "⚠️  Статус удаления: $HTTP_STATUS"

echo ""
echo "========================================"
echo "✅ Тест завершён!"
