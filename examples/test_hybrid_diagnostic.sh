#!/bin/bash

# 🎯 Прямой тест гибридного перевода - диагностика

API_URL="https://ministerial-yetta-fodi999-c58d8823.koyeb.app"
ADMIN_EMAIL="admin@fodi.app"
ADMIN_PASSWORD="Admin123!"

CYAN='\033[0;36m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

echo -e "${CYAN}════════════════════════════════════════════════════════════${NC}"
echo -e "${CYAN}🔍 ДИАГНОСТИКА: Гибридный перевод (Groq + Dictionary)${NC}"
echo -e "${CYAN}════════════════════════════════════════════════════════════${NC}"
echo ""

# 1. Вход
echo -e "${YELLOW}1️⃣ Вход администратора...${NC}"
LOGIN=$(curl -s -X POST "$API_URL/api/admin/auth/login" \
  -H "Content-Type: application/json" \
  -d "{\"email\":\"$ADMIN_EMAIL\",\"password\":\"$ADMIN_PASSWORD\"}")

TOKEN=$(echo "$LOGIN" | jq -r '.token')
echo "✅ Токен: ${TOKEN:0:30}..."
echo ""

# 2. Получить категорию
echo -e "${YELLOW}2️⃣ Получение категории...${NC}"
CATS=$(curl -s -X GET "$API_URL/api/admin/categories" \
  -H "Authorization: Bearer $TOKEN")

CATEGORY_ID=$(echo "$CATS" | jq -r '.categories[0].id')
echo "✅ Категория: $CATEGORY_ID"
echo ""

# 3. Создать продукт с auto_translate=true
echo -e "${YELLOW}3️⃣ Создание продукта 'Pineapple' с auto_translate=true...${NC}"
echo "   (только name_en заполнен, ИИ должен заполнить остальные)"
echo ""

RESPONSE=$(curl -s -X POST "$API_URL/api/admin/products" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d "{
    \"category_id\": \"$CATEGORY_ID\",
    \"name_en\": \"Pineapple\",
    \"auto_translate\": true,
    \"unit\": \"kilogram\",
    \"description\": \"Спелый ананас\"
  }")

echo -e "${CYAN}📤 ОТПРАВЛЕНО:${NC}"
echo "  name_en: Orange"
echo "  name_pl: [пусто - должен заполнить ИИ]"
echo "  name_ru: [пусто - должен заполнить ИИ]"
echo "  name_uk: [пусто - должен заполнить ИИ]"
echo "  auto_translate: true"
echo ""

echo -e "${CYAN}📥 ПОЛУЧЕНО:${NC}"
echo "$RESPONSE" | jq '{
  name_en: .name_en,
  name_pl: .name_pl,
  name_ru: .name_ru,
  name_uk: .name_uk
}'
echo ""

# Проверяем содержимое
NAME_EN=$(echo "$RESPONSE" | jq -r '.name_en // "null"')
NAME_PL=$(echo "$RESPONSE" | jq -r '.name_pl // "null"')
NAME_RU=$(echo "$RESPONSE" | jq -r '.name_ru // "null"')
NAME_UK=$(echo "$RESPONSE" | jq -r '.name_uk // "null"')

if [ "$NAME_PL" = "null" ] || [ -z "$NAME_PL" ]; then
  echo -e "${RED}❌ ИИ не переводит!${NC}"
  echo ""
  echo "Полный ответ сервера:"
  echo "$RESPONSE" | jq '.'
  echo ""
  echo -e "${YELLOW}Возможные причины:${NC}"
  echo "1. GROQ_API_KEY не установлен на сервере"
  echo "2. Limit Groq API достигнут"
  echo "3. auto_translate флаг не обрабатывается правильно"
  echo "4. DictionaryService/GroqService не инициализирован"
  exit 1
fi

echo -e "${GREEN}✅ ИИ успешно перевёл!${NC}"
echo ""
echo -e "${CYAN}📊 РЕЗУЛЬТАТ ПЕРЕВОДА:${NC}"
printf "┌───────┬──────────────┐\n"
printf "│ Язык  │ Перевод      │\n"
printf "├───────┼──────────────┤\n"
printf "│ 🇬🇧 EN │ %-12s │\n" "$NAME_EN"
printf "│ 🇵🇱 PL │ %-12s │\n" "$NAME_PL"
printf "│ 🇷🇺 RU │ %-12s │\n" "$NAME_RU"
printf "│ 🇺🇦 UK │ %-12s │\n" "$NAME_UK"
printf "└───────┴──────────────┘\n"
echo ""

# 4. Повторный запрос - проверяем кеш
PRODUCT_ID=$(echo "$RESPONSE" | jq -r '.id')

echo -e "${YELLOW}4️⃣ Повторный запрос того же слова 'Pineapple' (должно быть из кеша)...${NC}"

RESPONSE2=$(curl -s -X PUT "$API_URL/api/admin/products/$PRODUCT_ID" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name_en": "Pineapple",
    "auto_translate": true,
    "unit": "kilogram",
    "description": "Спелый ананас #2"
  }')

NAME_PL2=$(echo "$RESPONSE2" | jq -r '.name_pl // "null"')
NAME_RU2=$(echo "$RESPONSE2" | jq -r '.name_ru // "null"')

printf "┌───────┬──────────────┐\n"
printf "│ Язык  │ Перевод (2)  │\n"
printf "├───────┼──────────────┤\n"
printf "│ 🇬🇧 EN │ %-12s │\n" "$NAME_EN"
printf "│ 🇵🇱 PL │ %-12s │\n" "$NAME_PL2"
printf "│ 🇷🇺 RU │ %-12s │\n" "$NAME_RU2"
printf "│ 🇺🇦 UK │ %-12s │\n" "$(echo "$RESPONSE2" | jq -r '.name_uk // "null"')"
printf "└───────┴──────────────┘\n"
echo ""

if [ "$NAME_PL" = "$NAME_PL2" ] && [ "$NAME_RU" = "$NAME_RU2" ]; then
  echo -e "${GREEN}✅ Кеш работает! Переводы совпадают.${NC}"
else
  echo -e "${YELLOW}⚠️ Переводы отличаются (или второй раз ИИ дал новый результат)${NC}"
fi

echo ""
echo -e "${CYAN}════════════════════════════════════════════════════════════${NC}"
echo -e "${GREEN}✅ ДИАГНОСТИКА ЗАВЕРШЕНА${NC}"
echo -e "${CYAN}════════════════════════════════════════════════════════════${NC}"
