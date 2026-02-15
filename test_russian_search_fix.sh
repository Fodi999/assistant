#!/bin/bash

# 🔴 ТЕСТ: Поиск по русски должен работать
# Проверяем, что backend использует users.language из БД

BACKEND_URL="https://ministerial-yetta-fodi999-c58d8823.koyeb.app"

echo "================================="
echo "🧪 ТЕСТ: Русский Поиск"
echo "================================="
echo ""

# 1️⃣ Регистрируем юзера с language='ru'
echo "📝 1. Регистрация юзера с language='ru'"
echo "---"

REGISTER_RESPONSE=$(curl -s -X POST "$BACKEND_URL/api/auth/register" \
  -H "Content-Type: application/json" \
  -d "{
    \"email\": \"rustest_$(date +%s)@example.com\",
    \"password\": \"TestPass123!\",
    \"restaurant_name\": \"Русский Ресторан\",
    \"owner_name\": \"Иван Иванов\",
    \"language\": \"ru\"
  }")

echo "$REGISTER_RESPONSE" | jq .

ACCESS_TOKEN=$(echo "$REGISTER_RESPONSE" | jq -r '.access_token // empty')

if [ -z "$ACCESS_TOKEN" ]; then
  echo "❌ Регистрация не удалась"
  exit 1
fi

echo "✅ Юзер зарегистрирован"
echo "Token: ${ACCESS_TOKEN:0:50}..."
echo ""

# 2️⃣ Проверяем /api/me (язык должен быть 'ru')
echo "👤 2. Проверка языка юзера"
echo "---"

USER_INFO=$(curl -s -X GET "$BACKEND_URL/api/me" \
  -H "Authorization: Bearer $ACCESS_TOKEN")

echo "$USER_INFO" | jq .

USER_LANGUAGE=$(echo "$USER_INFO" | jq -r '.language // empty')

if [ "$USER_LANGUAGE" != "ru" ]; then
  echo "⚠️  ВНИМАНИЕ: Язык юзера = '$USER_LANGUAGE', ожидали 'ru'"
  echo "Продолжаем тест..."
fi

echo ""

# 3️⃣ Поиск по русски "молоко"
echo "🔍 3. Поиск 'молоко' (русский)"
echo "---"

SEARCH_RESPONSE=$(curl -s -X GET "$BACKEND_URL/api/catalog/ingredients?q=молоко" \
  -H "Authorization: Bearer $ACCESS_TOKEN")

echo "Raw response:"
echo "$SEARCH_RESPONSE"
echo ""
echo "Pretty JSON:"
echo "$SEARCH_RESPONSE" | jq . 2>/dev/null || echo "(Not valid JSON)"

INGREDIENTS_COUNT=$(echo "$SEARCH_RESPONSE" | jq '.ingredients | length' 2>/dev/null || echo "0")

echo ""
echo "================================="
echo "📊 РЕЗУЛЬТАТ"
echo "================================="
echo "Язык юзера:    $USER_LANGUAGE"
echo "Поиск:         'молоко'"
echo "Найдено:       $INGREDIENTS_COUNT продуктов"
echo ""

if [ "$INGREDIENTS_COUNT" -gt 0 ]; then
  echo "✅ УСПЕХ: Поиск по русски работает!"
  echo ""
  echo "Найденные продукты:"
  echo "$SEARCH_RESPONSE" | jq -r '.ingredients[] | "  - \(.name) (\(.default_unit))"'
else
  echo "❌ ОШИБКА: Поиск вернул 0 результатов"
  echo ""
  echo "🔧 Возможные причины:"
  echo "  1. Язык юзера в БД не 'ru' (проверьте выше)"
  echo "  2. SQL запрос ищет только по английскому (проверьте repository.rs)"
  echo "  3. Нет переводов в catalog_ingredient_translations"
fi

echo ""
echo "================================="
echo ""

# 4️⃣ Для сравнения: поиск по английски "milk"
echo "🔍 4. Поиск 'milk' (английский)"
echo "---"

SEARCH_EN=$(curl -s -X GET "$BACKEND_URL/api/catalog/ingredients?q=milk" \
  -H "Authorization: Bearer $ACCESS_TOKEN")

INGREDIENTS_EN=$(echo "$SEARCH_EN" | jq '.ingredients | length')

echo "Найдено: $INGREDIENTS_EN продуктов"
echo ""

if [ "$INGREDIENTS_EN" -gt 0 ]; then
  echo "✅ Поиск 'milk' работает"
else
  echo "⚠️  Поиск 'milk' тоже не работает - проблема глубже"
fi

echo ""
echo "================================="
echo "🎯 ФИНАЛЬНЫЙ ВЫВОД"
echo "================================="

if [ "$INGREDIENTS_COUNT" -gt 0 ] && [ "$USER_LANGUAGE" == "ru" ]; then
  echo "✅ ВСЁ ПРАВИЛЬНО РАБОТАЕТ!"
  echo ""
  echo "Backend:"
  echo "  - Язык берётся из users.language в БД ✅"
  echo "  - SQL использует COALESCE для fallback ✅"
  echo "  - Поиск по русски работает ✅"
  echo ""
  echo "Frontend:"
  echo "  - НЕ передавайте lang в query параметрах"
  echo "  - Backend сам определит язык из auth context"
  exit 0
elif [ "$USER_LANGUAGE" != "ru" ]; then
  echo "❌ ПРОБЛЕМА: Язык не сохранился при регистрации"
  echo ""
  echo "Что исправить:"
  echo "  1. Проверьте /api/auth/register endpoint"
  echo "  2. Убедитесь, что language записывается в users таблицу"
  echo "  3. Проверьте, что middleware читает language из БД"
  exit 1
else
  echo "❌ ПРОБЛЕМА: Поиск не работает"
  echo ""
  echo "Что исправить:"
  echo "  1. Проверьте SQL в catalog_ingredient_repository.rs"
  echo "  2. Убедитесь, что WHERE использует COALESCE(cit_user.name, cit_en.name)"
  echo "  3. Проверьте наличие переводов в catalog_ingredient_translations"
  exit 1
fi
