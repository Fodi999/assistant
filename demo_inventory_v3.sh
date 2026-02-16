#!/bin/bash

# Конфигурация
API_URL="https://ministerial-yetta-fodi999-c58d8823.koyeb.app/api"
EMAIL="test@fodi.app"
PASSWORD="password123"

echo "🚀 Начинаем комплексный тест Инвентаризации: FIFO + Waste KPI + Health Score"

# 1. Логин для получения токена
echo "🔑 Авторизация..."
LOGIN_RES=$(curl -s -X POST "$API_URL/auth/login" -H "Content-Type: application/json" -d "{\"email\":\"$EMAIL\", \"password\":\"$PASSWORD\"}")
TOKEN=$(echo $LOGIN_RES | grep -oE '"access_token":"[^"]+"' | cut -d'"' -f4)

if [ -z "$TOKEN" ]; then
    echo "❌ Ошибка авторизации. Проверь логин/пароль."
    exit 1
fi

# 2. Получаем ID какого-нибудь ингредиента из каталога
echo "📦 Поиск ингредиента в каталоге..."
ING_ID=$(curl -s -H "Authorization: Bearer $TOKEN" "$API_URL/catalog/ingredients" | grep -oE '"id":"[^"]+"' | head -1 | cut -d'"' -f4)

if [ -z "$ING_ID" ]; then
    echo "⚠️ Каталог пуст. Добавь ингредиент через админку."
    exit 1
fi

# 3. Добавляем СВЕЖУЮ партию (10 единиц по 100 центов)
echo "📥 Добавляем нормальную партию..."
curl -s -X POST "$API_URL/inventory/products" \
     -H "Authorization: Bearer $TOKEN" \
     -H "Content-Type: application/json" \
     -d "{
        \"catalog_ingredient_id\": \"$ING_ID\",
        \"price_per_unit_cents\": 100,
        \"quantity\": 10,
        \"received_at\": \"$(date -u +"%Y-%m-%dT%H:%M:%SZ")\"
     }" > /dev/null

# 4. Добавляем ПРОСРОЧЕННУЮ партию (5 единиц по 200 центов)
echo "💀 Добавляем просроченную партию (намеренно)..."
# Используем дату в прошлом
PAST_DATE="2026-01-01T12:00:00Z"
curl -s -X POST "$API_URL/inventory/products" \
     -H "Authorization: Bearer $TOKEN" \
     -H "Content-Type: application/json" \
     -d "{
        \"catalog_ingredient_id\": \"$ING_ID\",
        \"price_per_unit_cents\": 200,
        \"quantity\": 5,
        \"received_at\": \"$PAST_DATE\",
        \"expires_at\": \"$PAST_DATE\"
     }" > /dev/null

# 5. Проверяем Health Score (должен упасть из-за просрочки)
echo "📊 Проверка Health Score..."
HEALTH_BEFORE=$(curl -s -H "Authorization: Bearer $TOKEN" "$API_URL/inventory/health")
echo "Health Score до очистки: $(echo $HEALTH_BEFORE | grep -oE '"health_score":[0-9]+' | cut -d: -f2)"

# 6. Запускаем магию: Автоматическое списание просрочки
echo "🧹 Запуск process-expirations..."
PROCESS_RES=$(curl -s -X POST "$API_URL/inventory/process-expirations" -H "Authorization: Bearer $TOKEN")
echo "Результат: $PROCESS_RES"

# 7. Проверяем финансовый отчет Loss Report
echo "📈 Генерация Loss Report (Waste KPI)..."
REPORT=$(curl -s -H "Authorization: Bearer $TOKEN" "$API_URL/inventory/reports/loss?days=30")
echo "Total Loss: $(echo $REPORT | grep -oE '"total_loss_cents":[0-9]+' | cut -d: -f2) cents"
echo "Waste Percentage: $(echo $REPORT | grep -oE '"waste_percentage":[0-9.]+' | cut -d: -f2)%"

# 8. Финальный Health Score (должен вырасти после удаления мусора)
echo "✨ Итоговый статус..."
HEALTH_AFTER=$(curl -s -H "Authorization: Bearer $TOKEN" "$API_URL/inventory/health")
echo "Health Score после очистки: $(echo $HEALTH_AFTER | grep -oE '"health_score":[0-9]+' | cut -d: -f2)"

echo "✅ Тест завершен!"
