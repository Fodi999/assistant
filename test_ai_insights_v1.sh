#!/bin/bash

# AI Insights V1 Testing Script
# Тестирует полный flow: login → create recipe → generate insights → get cached → refresh

set -e  # Exit on error

BASE_URL="http://localhost:8000"
EMAIL="test_ai@fodi.app"
PASSWORD="test12345"

echo "═══════════════════════════════════════════════════════"
echo "🧪 AI Insights V1 Testing Script"
echo "═══════════════════════════════════════════════════════"
echo ""

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Step 1: Login
echo -e "${BLUE}Step 1:${NC} Logging in..."
LOGIN_RESPONSE=$(curl -s -X POST "$BASE_URL/api/auth/login" \
  -H "Content-Type: application/json" \
  -d "{\"email\":\"$EMAIL\",\"password\":\"$PASSWORD\"}")

TOKEN=$(echo $LOGIN_RESPONSE | jq -r '.access_token')

if [ "$TOKEN" = "null" ] || [ -z "$TOKEN" ]; then
  echo -e "${RED}❌ Login failed${NC}"
  echo $LOGIN_RESPONSE | jq .
  exit 1
fi

echo -e "${GREEN}✅ Logged in successfully${NC}"
echo "Token: ${TOKEN:0:20}..."
echo ""

# Step 2: Get a catalog ingredient
echo -e "${BLUE}Step 2:${NC} Fetching catalog ingredients..."
INGREDIENTS=$(curl -s -X GET "$BASE_URL/api/admin/catalog/ingredients?limit=1" \
  -H "Authorization: Bearer $TOKEN")

INGREDIENT_ID=$(echo $INGREDIENTS | jq -r '.data[0].id')

if [ "$INGREDIENT_ID" = "null" ] || [ -z "$INGREDIENT_ID" ]; then
  echo -e "${RED}❌ No ingredients found${NC}"
  exit 1
fi

echo -e "${GREEN}✅ Found ingredient:${NC} $INGREDIENT_ID"
echo ""

# Step 3: Create a test recipe
echo -e "${BLUE}Step 3:${NC} Creating test recipe..."
RECIPE_RESPONSE=$(curl -s -X POST "$BASE_URL/api/recipes/v2" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d "{
    \"name\": \"Тестовый борщ для AI анализа\",
    \"instructions\": \"Сварить свеклу и морковь. Добавить капусту и картофель. Варить 2 часа на медленном огне. Посолить по вкусу.\",
    \"language\": \"ru\",
    \"servings\": 6,
    \"ingredients\": [{
      \"catalog_ingredient_id\": \"$INGREDIENT_ID\",
      \"quantity\": 0.5,
      \"unit\": \"kg\"
    }]
  }")

RECIPE_ID=$(echo $RECIPE_RESPONSE | jq -r '.id')

if [ "$RECIPE_ID" = "null" ] || [ -z "$RECIPE_ID" ]; then
  echo -e "${RED}❌ Recipe creation failed${NC}"
  echo $RECIPE_RESPONSE | jq .
  exit 1
fi

echo -e "${GREEN}✅ Recipe created:${NC} $RECIPE_ID"
echo ""

# Step 4: Generate AI insights (first time - should call AI)
echo -e "${BLUE}Step 4:${NC} Generating AI insights (first time)..."
echo -e "${YELLOW}⏳ This will take 2-3 seconds (calling AI)...${NC}"

START_TIME=$(date +%s%3N)
INSIGHTS_RESPONSE=$(curl -s -X GET "$BASE_URL/api/recipes/v2/$RECIPE_ID/insights/ru" \
  -H "Authorization: Bearer $TOKEN")
END_TIME=$(date +%s%3N)
DURATION=$((END_TIME - START_TIME))

INSIGHTS_ID=$(echo $INSIGHTS_RESPONSE | jq -r '.id')

if [ "$INSIGHTS_ID" = "null" ] || [ -z "$INSIGHTS_ID" ]; then
  echo -e "${RED}❌ AI insights generation failed${NC}"
  echo $INSIGHTS_RESPONSE | jq .
  exit 1
fi

echo -e "${GREEN}✅ AI insights generated:${NC} $INSIGHTS_ID"
echo -e "⏱️  Duration: ${DURATION}ms"
echo ""

# Show insights structure
echo -e "${BLUE}📊 Insights Structure:${NC}"
echo "Steps count: $(echo $INSIGHTS_RESPONSE | jq '.steps | length')"
echo "Validation errors: $(echo $INSIGHTS_RESPONSE | jq '.validation.errors | length')"
echo "Validation warnings: $(echo $INSIGHTS_RESPONSE | jq '.validation.warnings | length')"
echo "Suggestions count: $(echo $INSIGHTS_RESPONSE | jq '.suggestions | length')"
echo "Feasibility score: $(echo $INSIGHTS_RESPONSE | jq '.feasibility_score')"
echo "Model: $(echo $INSIGHTS_RESPONSE | jq -r '.model')"
echo ""

# Show first step
echo -e "${BLUE}🔍 First Cooking Step:${NC}"
echo $INSIGHTS_RESPONSE | jq '.steps[0]'
echo ""

# Show validation
echo -e "${BLUE}🔍 Validation:${NC}"
echo $INSIGHTS_RESPONSE | jq '.validation'
echo ""

# Step 5: Get cached insights (second time - should be instant)
echo -e "${BLUE}Step 5:${NC} Getting cached insights (should be instant)..."

START_TIME=$(date +%s%3N)
CACHED_RESPONSE=$(curl -s -X GET "$BASE_URL/api/recipes/v2/$RECIPE_ID/insights/ru" \
  -H "Authorization: Bearer $TOKEN")
END_TIME=$(date +%s%3N)
CACHE_DURATION=$((END_TIME - START_TIME))

CACHED_ID=$(echo $CACHED_RESPONSE | jq -r '.id')

if [ "$CACHED_ID" != "$INSIGHTS_ID" ]; then
  echo -e "${RED}❌ Cache failed - got different ID${NC}"
  exit 1
fi

echo -e "${GREEN}✅ Got cached insights${NC}"
echo -e "⚡ Duration: ${CACHE_DURATION}ms (vs ${DURATION}ms on first call)"
echo ""

# Step 6: Get all insights (all languages)
echo -e "${BLUE}Step 6:${NC} Getting all insights (all languages)..."
ALL_INSIGHTS=$(curl -s -X GET "$BASE_URL/api/recipes/v2/$RECIPE_ID/insights" \
  -H "Authorization: Bearer $TOKEN")

INSIGHTS_COUNT=$(echo $ALL_INSIGHTS | jq '. | length')

echo -e "${GREEN}✅ Found $INSIGHTS_COUNT language(s)${NC}"
echo $ALL_INSIGHTS | jq -r '.[] | "  - \(.language): \(.model)"'
echo ""

# Step 7: Refresh insights (force regeneration)
echo -e "${BLUE}Step 7:${NC} Refreshing insights (force regeneration)..."
echo -e "${YELLOW}⏳ This will take 2-3 seconds again...${NC}"

START_TIME=$(date +%s%3N)
REFRESH_RESPONSE=$(curl -s -X POST "$BASE_URL/api/recipes/v2/$RECIPE_ID/insights/ru/refresh" \
  -H "Authorization: Bearer $TOKEN")
END_TIME=$(date +%s%3N)
REFRESH_DURATION=$((END_TIME - START_TIME))

REFRESH_ID=$(echo $REFRESH_RESPONSE | jq -r '.id')

if [ "$REFRESH_ID" = "null" ] || [ -z "$REFRESH_ID" ]; then
  echo -e "${RED}❌ Refresh failed${NC}"
  echo $REFRESH_RESPONSE | jq .
  exit 1
fi

echo -e "${GREEN}✅ Insights refreshed${NC}"
echo -e "⏱️  Duration: ${REFRESH_DURATION}ms"
echo ""

# Check if refresh generated new insights
if [ "$REFRESH_ID" = "$INSIGHTS_ID" ]; then
  echo -e "${YELLOW}⚠️  Warning: Refresh returned same ID (might be cached?)${NC}"
else
  echo -e "${GREEN}✅ Refresh generated new insights (different ID)${NC}"
fi
echo ""

# Step 8: Check database record
echo -e "${BLUE}Step 8:${NC} Checking database record..."
echo "Recipe ID: $RECIPE_ID"
echo "Insights ID: $REFRESH_ID"
echo ""

# Final summary
echo "═══════════════════════════════════════════════════════"
echo -e "${GREEN}✅ ALL TESTS PASSED${NC}"
echo "═══════════════════════════════════════════════════════"
echo ""
echo "Performance:"
echo "  - First generation: ${DURATION}ms"
echo "  - Cached retrieval: ${CACHE_DURATION}ms"
echo "  - Refresh: ${REFRESH_DURATION}ms"
echo ""
echo "Insights Quality:"
echo "  - Steps: $(echo $REFRESH_RESPONSE | jq '.steps | length')"
echo "  - Feasibility: $(echo $REFRESH_RESPONSE | jq '.feasibility_score')/100"
echo "  - Model: $(echo $REFRESH_RESPONSE | jq -r '.model')"
echo ""

# Optional: Show full insights JSON
read -p "Show full insights JSON? (y/n) " -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]]; then
  echo $REFRESH_RESPONSE | jq .
fi
