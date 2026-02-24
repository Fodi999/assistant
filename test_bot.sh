#!/bin/bash

# test_bot.sh - Скрипт для тестирования Assistant Bot (State Machine)
# Запускать при работающем сервере (cargo run)

API_URL="http://localhost:8000/api"
EMAIL="bot_tester_$(date +%s)@example.com"
PASSWORD="Password123!"

echo "🤖 Тестируем Assistant Bot локально..."
echo "=================================================="

# 1. Регистрация пользователя
echo "1️⃣ Регистрация нового пользователя: $EMAIL"
AUTH_RESPONSE=$(curl -s -X POST "$API_URL/auth/register" \
  -H "Content-Type: application/json" \
  -d "{\"email\":\"$EMAIL\",\"password\":\"$PASSWORD\",\"first_name\":\"Bot\",\"last_name\":\"Tester\",\"language\":\"ru\",\"restaurant_name\":\"Test Restaurant\"}")

TOKEN=$(echo $AUTH_RESPONSE | grep -o '"access_token":"[^"]*' | cut -d'"' -f4)

if [ -z "$TOKEN" ]; then
  echo "❌ Ошибка регистрации. Ответ: $AUTH_RESPONSE"
  exit 1
fi
echo "✅ Успешно зарегистрирован. Получен токен."

# 2. Получение начального состояния бота
echo -e "\n2️⃣ Запрашиваем текущее состояние бота (GET /assistant/state)"
STATE_RESPONSE=$(curl -s -X GET "$API_URL/assistant/state" \
  -H "Authorization: Bearer $TOKEN")

echo "Ответ бота:"
echo $STATE_RESPONSE | jq .

# 3. Отправляем команду StartInventory
echo -e "\n3️⃣ Отправляем команду: start_inventory"
CMD_RESPONSE=$(curl -s -X POST "$API_URL/assistant/command" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"command": {"type": "start_inventory"}}')

echo "Ответ бота:"
echo $CMD_RESPONSE | jq .

# 4. Пытаемся завершить инвентаризацию без продуктов (проверка валидации)
echo -e "\n4️⃣ Пытаемся завершить инвентаризацию без продуктов (ожидаем ошибку)"
CMD_RESPONSE=$(curl -s -X POST "$API_URL/assistant/command" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"command": {"type": "finish_inventory"}}')

echo "Ответ бота:"
echo $CMD_RESPONSE | jq .

# 5. Ищем ингредиент 'milk' в каталоге
echo -e "\n5️⃣ Ищем ингредиент 'milk' в каталоге"
CATALOG_RESPONSE=$(curl -s -X GET "$API_URL/catalog/ingredients?query=milk" \
  -H "Authorization: Bearer $TOKEN")

INGREDIENT_ID=$(echo $CATALOG_RESPONSE | jq -r '.ingredients[0].id // .data[0].id // .[0].id')

if [ "$INGREDIENT_ID" == "null" ] || [ -z "$INGREDIENT_ID" ]; then
  echo "❌ Ингредиент не найден. Пропускаем добавление."
else
  echo "✅ Найден ингредиент ID: $INGREDIENT_ID"
  
  # 6. Добавляем продукт в инвентаризацию через бота
  echo -e "\n6️⃣ Добавляем продукт через бота (add_product)"
  
  NOW=$(date -u +"%Y-%m-%dT%H:%M:%SZ")
  EXPIRES=$(date -v+7d -u +"%Y-%m-%dT%H:%M:%SZ" 2>/dev/null || date -d "+7 days" -u +"%Y-%m-%dT%H:%M:%SZ") # macOS/Linux compat
  
  CMD_RESPONSE=$(curl -s -X POST "$API_URL/assistant/command" \
    -H "Authorization: Bearer $TOKEN" \
    -H "Content-Type: application/json" \
    -d "{
      \"command\": {
        \"type\": \"add_product\",
        \"payload\": {
          \"catalog_ingredient_id\": \"$INGREDIENT_ID\",
          \"price_per_unit_cents\": 500,
          \"quantity\": 10.0,
          \"received_at\": \"$NOW\",
          \"expires_at\": \"$EXPIRES\"
        }
      }
    }")
    
  echo "Ответ бота:"
  echo $CMD_RESPONSE | jq .
  
  # 7. Завершаем инвентаризацию
  echo -e "\n7️⃣ Завершаем инвентаризацию (finish_inventory)"
  CMD_RESPONSE=$(curl -s -X POST "$API_URL/assistant/command" \
    -H "Authorization: Bearer $TOKEN" \
    -H "Content-Type: application/json" \
    -d '{"command": {"type": "finish_inventory"}}')
    
  echo "Ответ бота:"
  echo $CMD_RESPONSE | jq .
fi

echo -e "\n🎉 Тестирование базового флоу бота завершено!"
