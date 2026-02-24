#!/bin/bash

# load_test_bot.sh - Нагрузочное тестирование Assistant Bot (1000 продуктов)
# Проверяет скорость работы и эффективность Rule Engine / AI Cache

API_URL="http://localhost:8000/api"
EMAIL="load_tester_$(date +%s)@example.com"
PASSWORD="Password123!"

echo "🚀 Запуск нагрузочного тестирования (1000 продуктов)..."
echo "=================================================="

# 1. Регистрация
echo "1️⃣ Регистрация пользователя: $EMAIL"
AUTH_RESPONSE=$(curl -s -X POST "$API_URL/auth/register" \
  -H "Content-Type: application/json" \
  -d "{\"email\":\"$EMAIL\",\"password\":\"$PASSWORD\",\"first_name\":\"Load\",\"last_name\":\"Tester\",\"language\":\"ru\",\"restaurant_name\":\"Load Test Resto\"}")

TOKEN=$(echo $AUTH_RESPONSE | grep -o '"access_token":"[^"]*' | cut -d'"' -f4)

if [ -z "$TOKEN" ]; then
  echo "❌ Ошибка регистрации."
  exit 1
fi

# 2. Start Inventory
curl -s -X POST "$API_URL/assistant/command" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"command": {"type": "start_inventory"}}' > /dev/null

# 3. Получаем список ингредиентов для теста
echo "2️⃣ Получаем ингредиенты из каталога..."
CATALOG_RESPONSE=$(curl -s -X GET "$API_URL/catalog/ingredients?limit=50" \
  -H "Authorization: Bearer $TOKEN")

# Извлекаем массив ID ингредиентов
INGREDIENT_IDS=($(echo $CATALOG_RESPONSE | jq -r '.ingredients[].id // .data[].id // .[].id'))

if [ ${#INGREDIENT_IDS[@]} -eq 0 ]; then
  echo "❌ Не удалось получить ингредиенты из каталога."
  exit 1
fi

echo "✅ Найдено ${#INGREDIENT_IDS[@]} ингредиентов для теста."

# 4. Нагрузочный тест (1000 запросов)
echo "3️⃣ Запускаем добавление 1000 продуктов..."

START_TIME=$(date +%s)
SUCCESS_COUNT=0
ERROR_COUNT=0

NOW=$(date -u +"%Y-%m-%dT%H:%M:%SZ")
EXPIRES=$(date -v+7d -u +"%Y-%m-%dT%H:%M:%SZ" 2>/dev/null || date -d "+7 days" -u +"%Y-%m-%dT%H:%M:%SZ")

for i in {1..1000}; do
  # Выбираем случайный ингредиент из массива
  RANDOM_INDEX=$((RANDOM % ${#INGREDIENT_IDS[@]}))
  INGREDIENT_ID=${INGREDIENT_IDS[$RANDOM_INDEX]}
  
  # Отправляем запрос и сохраняем HTTP статус
  RESPONSE=$(curl -s -o /dev/null -w "%{http_code}" -X POST "$API_URL/assistant/command" \
    -H "Authorization: Bearer $TOKEN" \
    -H "Content-Type: application/json" \
    -d "{
      \"command\": {
        \"type\": \"add_product\",
        \"payload\": {
          \"catalog_ingredient_id\": \"$INGREDIENT_ID\",
          \"price_per_unit_cents\": $((RANDOM % 1000 + 100)),
          \"quantity\": 10.0,
          \"received_at\": \"$NOW\",
          \"expires_at\": \"$EXPIRES\"
        }
      }
    }")
    
  if [ "$RESPONSE" == "200" ]; then
    ((SUCCESS_COUNT++))
  else
    ((ERROR_COUNT++))
  fi
  
  # Прогресс-бар каждые 10 запросов (чтобы видеть, что скрипт не висит)
  if [ $((i % 10)) -eq 0 ]; then
    echo "⏳ Прогресс: $i / 1000 (Успешно: $SUCCESS_COUNT, Ошибок: $ERROR_COUNT)"
  fi
done

END_TIME=$(date +%s)
DURATION=$((END_TIME - START_TIME))

echo "=================================================="
echo "🎉 Тест завершен за $DURATION секунд!"
echo "✅ Успешно: $SUCCESS_COUNT"
echo "❌ Ошибок: $ERROR_COUNT"
echo "⚡ Средняя скорость: $(echo "scale=2; 1000 / $DURATION" | bc) запросов в секунду"

# 5. Проверка статистики AI
echo -e "\n📊 Статистика использования AI (из базы данных):"
psql -U postgres -d assistant_db -c "
SELECT 
  COUNT(*) as total_ai_calls,
  SUM(prompt_tokens) as total_prompt_tokens,
  SUM(completion_tokens) as total_completion_tokens,
  AVG(duration_ms) as avg_duration_ms
FROM ai_usage_stats;
"

echo -e "\n🧠 Статистика AI Cache:"
psql -U postgres -d assistant_db -c "
SELECT COUNT(*) as cached_items FROM ai_cache;
"