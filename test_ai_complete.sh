#!/bin/bash

# AI Insights V1 Complete Test
# Полный тест: admin login → add ingredient → create recipe → AI insights

set -e

BASE_URL="http://localhost:8000"
ADMIN_EMAIL="admin@fodi.app"
ADMIN_PASSWORD="Admin123!"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

echo "═══════════════════════════════════════════════════════"
echo "🧪 AI Insights V1 Complete Test"
echo "═══════════════════════════════════════════════════════"
echo ""

# Step 1: Admin Login
echo -e "${BLUE}Step 1:${NC} Admin login..."
ADMIN_LOGIN=$(curl -s -X POST "$BASE_URL/api/admin/auth/login" \
  -H "Content-Type: application/json" \
  -d "{\"email\":\"$ADMIN_EMAIL\",\"password\":\"$ADMIN_PASSWORD\"}")

ADMIN_TOKEN=$(echo $ADMIN_LOGIN | jq -r '.token // .access_token')

if [ "$ADMIN_TOKEN" = "null" ] || [ -z "$ADMIN_TOKEN" ]; then
  echo -e "${RED}❌ Admin login failed${NC}"
  echo $ADMIN_LOGIN | jq .
  exit 1
fi

echo -e "${GREEN}✅ Admin logged in${NC}"
echo ""

# Step 2: Check existing ingredients
echo -e "${BLUE}Step 2:${NC} Checking catalog ingredients..."
INGREDIENTS=$(curl -s -X GET "$BASE_URL/api/admin/catalog/ingredients?limit=5" \
  -H "Authorization: Bearer $ADMIN_TOKEN")

INGREDIENT_COUNT=$(echo $INGREDIENTS | jq '.data | length')

if [ "$INGREDIENT_COUNT" = "0" ]; then
  echo -e "${YELLOW}⚠️  No ingredients found, creating test ingredients...${NC}"
  
  # Create test ingredients
  BEET=$(curl -s -X POST "$BASE_URL/api/admin/catalog/ingredients" \
    -H "Authorization: Bearer $ADMIN_TOKEN" \
    -H "Content-Type: application/json" \
    -d '{
      "name": "Свекла",
      "language": "ru",
      "default_unit": "kg",
      "category": "vegetables"
    }')
  
  CARROT=$(curl -s -X POST "$BASE_URL/api/admin/catalog/ingredients" \
    -H "Authorization: Bearer $ADMIN_TOKEN" \
    -H "Content-Type: application/json" \
    -d '{
      "name": "Морковь",
      "language": "ru",
      "default_unit": "kg",
      "category": "vegetables"
    }')
  
  CABBAGE=$(curl -s -X POST "$BASE_URL/api/admin/catalog/ingredients" \
    -H "Authorization: Bearer $ADMIN_TOKEN" \
    -H "Content-Type: application/json" \
    -d '{
      "name": "Капуста",
      "language": "ru",
      "default_unit": "kg",
      "category": "vegetables"
    }')
  
  echo -e "${GREEN}✅ Created 3 test ingredients${NC}"
  
  # Re-fetch ingredients
  INGREDIENTS=$(curl -s -X GET "$BASE_URL/api/admin/catalog/ingredients?limit=5" \
    -H "Authorization: Bearer $ADMIN_TOKEN")
else
  echo -e "${GREEN}✅ Found $INGREDIENT_COUNT ingredients${NC}"
fi

INGREDIENT_ID=$(echo $INGREDIENTS | jq -r '.data[0].id')
echo "Using ingredient: $(echo $INGREDIENTS | jq -r '.data[0].name') ($INGREDIENT_ID)"
echo ""

# Step 3: Register regular user (for recipe creation)
echo -e "${BLUE}Step 3:${NC} Creating test user..."
USER_REGISTER=$(curl -s -X POST "$BASE_URL/api/auth/register" \
  -H "Content-Type: application/json" \
  -d '{
    "email": "chef_ai_test@fodi.app",
    "password": "test12345",
    "name": "AI Test Chef",
    "restaurant_name": "AI Test Kitchen"
  }')

USER_TOKEN=$(echo $USER_REGISTER | jq -r '.access_token')

if [ "$USER_TOKEN" = "null" ] || [ -z "$USER_TOKEN" ]; then
  # User might already exist, try login
  echo -e "${YELLOW}⚠️  User exists, trying login...${NC}"
  USER_LOGIN=$(curl -s -X POST "$BASE_URL/api/auth/login" \
    -H "Content-Type: application/json" \
    -d '{"email":"chef_ai_test@fodi.app","password":"test12345"}')
  
  USER_TOKEN=$(echo $USER_LOGIN | jq -r '.access_token')
  
  if [ "$USER_TOKEN" = "null" ] || [ -z "$USER_TOKEN" ]; then
    echo -e "${RED}❌ User login failed${NC}"
    echo $USER_LOGIN | jq .
    exit 1
  fi
fi

echo -e "${GREEN}✅ User ready${NC}"
echo ""

# Step 4: Create recipe
echo -e "${BLUE}Step 4:${NC} Creating test recipe..."
RECIPE=$(curl -s -X POST "$BASE_URL/api/recipes/v2" \
  -H "Authorization: Bearer $USER_TOKEN" \
  -H "Content-Type: application/json" \
  -d "{
    \"name\": \"Борщ украинский классический\",
    \"instructions\": \"1. Сварить свеклу и морковь до мягкости (40 минут). 2. Нарезать капусту соломкой и добавить в кастрюлю. 3. Добавить картофель кубиками. 4. Варить на медленном огне 2 часа. 5. Посолить, добавить лавровый лист и чеснок.\",
    \"language\": \"ru\",
    \"servings\": 6,
    \"ingredients\": [{
      \"catalog_ingredient_id\": \"$INGREDIENT_ID\",
      \"quantity\": 0.5,
      \"unit\": \"kg\"
    }]
  }")

RECIPE_ID=$(echo $RECIPE | jq -r '.id')

if [ "$RECIPE_ID" = "null" ] || [ -z "$RECIPE_ID" ]; then
  echo -e "${RED}❌ Recipe creation failed${NC}"
  echo $RECIPE | jq .
  exit 1
fi

echo -e "${GREEN}✅ Recipe created: $RECIPE_ID${NC}"
echo "Name: $(echo $RECIPE | jq -r '.name')"
echo ""

# Step 5: Generate AI insights (MAIN TEST)
echo -e "${BLUE}Step 5:${NC} 🤖 Generating AI insights (first time)..."
echo -e "${YELLOW}⏳ Calling Groq AI... (this takes 2-3 seconds)${NC}"

START=$(date +%s%3N)
INSIGHTS=$(curl -s -X GET "$BASE_URL/api/recipes/v2/$RECIPE_ID/insights/ru" \
  -H "Authorization: Bearer $USER_TOKEN")
END=$(date +%s%3N)
DURATION=$((END - START))

INSIGHTS_ID=$(echo $INSIGHTS | jq -r '.id')

if [ "$INSIGHTS_ID" = "null" ] || [ -z "$INSIGHTS_ID" ]; then
  echo -e "${RED}❌ AI insights generation failed${NC}"
  echo $INSIGHTS | jq .
  exit 1
fi

echo -e "${GREEN}✅ AI insights generated in ${DURATION}ms${NC}"
echo ""

# Step 6: Show insights structure
echo "═══════════════════════════════════════════════════════"
echo -e "${GREEN}📊 AI Insights Results${NC}"
echo "═══════════════════════════════════════════════════════"
echo ""

STEPS_COUNT=$(echo $INSIGHTS | jq '.steps | length')
ERRORS_COUNT=$(echo $INSIGHTS | jq '.validation.errors | length')
WARNINGS_COUNT=$(echo $INSIGHTS | jq '.validation.warnings | length')
SUGGESTIONS_COUNT=$(echo $INSIGHTS | jq '.suggestions | length')
FEASIBILITY=$(echo $INSIGHTS | jq '.feasibility_score')
MODEL=$(echo $INSIGHTS | jq -r '.model')

echo "📈 Statistics:"
echo "  Steps: $STEPS_COUNT"
echo "  Validation Errors: $ERRORS_COUNT"
echo "  Validation Warnings: $WARNINGS_COUNT"
echo "  Suggestions: $SUGGESTIONS_COUNT"
echo "  Feasibility Score: $FEASIBILITY/100"
echo "  Model: $MODEL"
echo ""

# Show first cooking step
if [ "$STEPS_COUNT" != "0" ]; then
  echo -e "${BLUE}🍳 First Cooking Step:${NC}"
  echo $INSIGHTS | jq '.steps[0]'
  echo ""
fi

# Show validation
echo -e "${BLUE}✅ Validation Summary:${NC}"
echo $INSIGHTS | jq '.validation | {
  errors: .errors | length,
  warnings: .warnings | length,
  is_valid: (.errors | length == 0)
}'
echo ""

# Show first suggestion if exists
if [ "$SUGGESTIONS_COUNT" != "0" ]; then
  echo -e "${BLUE}💡 First Suggestion:${NC}"
  echo $INSIGHTS | jq '.suggestions[0]'
  echo ""
fi

# Step 7: Test caching (should be instant)
echo -e "${BLUE}Step 6:${NC} Testing cache (should be <100ms)..."

START=$(date +%s%3N)
CACHED=$(curl -s -X GET "$BASE_URL/api/recipes/v2/$RECIPE_ID/insights/ru" \
  -H "Authorization: Bearer $USER_TOKEN")
END=$(date +%s%3N)
CACHE_DURATION=$((END - START))

CACHED_ID=$(echo $CACHED | jq -r '.id')

if [ "$CACHED_ID" != "$INSIGHTS_ID" ]; then
  echo -e "${RED}❌ Cache test failed - different ID${NC}"
  exit 1
fi

echo -e "${GREEN}✅ Got cached insights in ${CACHE_DURATION}ms${NC}"
echo -e "⚡ Speedup: ${DURATION}ms → ${CACHE_DURATION}ms ($(echo "scale=1; $DURATION / $CACHE_DURATION" | bc)x faster)"
echo ""

# Step 8: Test refresh (force regeneration)
echo -e "${BLUE}Step 7:${NC} Testing refresh (force regeneration)..."
echo -e "${YELLOW}⏳ Calling AI again...${NC}"

START=$(date +%s%3N)
REFRESHED=$(curl -s -X POST "$BASE_URL/api/recipes/v2/$RECIPE_ID/insights/ru/refresh" \
  -H "Authorization: Bearer $USER_TOKEN")
END=$(date +%s%3N)
REFRESH_DURATION=$((END - START))

REFRESH_ID=$(echo $REFRESHED | jq -r '.id')

if [ "$REFRESH_ID" = "null" ] || [ -z "$REFRESH_ID" ]; then
  echo -e "${RED}❌ Refresh failed${NC}"
  echo $REFRESHED | jq .
  exit 1
fi

echo -e "${GREEN}✅ Insights refreshed in ${REFRESH_DURATION}ms${NC}"
echo ""

# Step 9: Test get all insights (all languages)
echo -e "${BLUE}Step 8:${NC} Getting all insights (all languages)..."
ALL_INSIGHTS=$(curl -s -X GET "$BASE_URL/api/recipes/v2/$RECIPE_ID/insights" \
  -H "Authorization: Bearer $USER_TOKEN")

INSIGHTS_COUNT=$(echo $ALL_INSIGHTS | jq '. | length')

echo -e "${GREEN}✅ Found $INSIGHTS_COUNT language(s):${NC}"
echo $ALL_INSIGHTS | jq -r '.[] | "  - \(.language): \(.feasibility_score)/100 (\(.model))"'
echo ""

# Final Summary
echo "═══════════════════════════════════════════════════════"
echo -e "${GREEN}✅ ALL TESTS PASSED!${NC}"
echo "═══════════════════════════════════════════════════════"
echo ""
echo "Performance Summary:"
echo "  First generation: ${DURATION}ms"
echo "  Cached retrieval: ${CACHE_DURATION}ms"
echo "  Refresh: ${REFRESH_DURATION}ms"
echo ""
echo "Quality Summary:"
echo "  Steps generated: $STEPS_COUNT"
echo "  Feasibility score: $FEASIBILITY/100"
echo "  Errors: $ERRORS_COUNT"
echo "  Warnings: $WARNINGS_COUNT"
echo "  Suggestions: $SUGGESTIONS_COUNT"
echo ""
echo "IDs:"
echo "  Recipe: $RECIPE_ID"
echo "  Insights: $REFRESH_ID"
echo ""

# Optional: Show full JSON
read -p "Show full AI insights JSON? (y/n) " -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]]; then
  echo $REFRESHED | jq .
fi
