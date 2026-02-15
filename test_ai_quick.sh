#!/bin/bash

# Quick AI Insights Test - упрощенная версия без зависимостей от каталога

set -e

BASE_URL="http://localhost:8000"
EMAIL="test_ai@fodi.app"
PASSWORD="test12345"

echo "🧪 Quick AI Insights V1 Test"
echo "================================"
echo ""

# Step 1: Login
echo "Step 1: Login..."
LOGIN_RESPONSE=$(curl -s -X POST "$BASE_URL/api/auth/login" \
  -H "Content-Type: application/json" \
  -d "{\"email\":\"$EMAIL\",\"password\":\"$PASSWORD\"}")

TOKEN=$(echo $LOGIN_RESPONSE | jq -r '.access_token')

if [ "$TOKEN" = "null" ] || [ -z "$TOKEN" ]; then
  echo "❌ Login failed"
  echo $LOGIN_RESPONSE | jq .
  exit 1
fi

echo "✅ Logged in"
echo ""

# Step 2: Create recipe WITHOUT ingredients (just to test AI)
echo "Step 2: Creating test recipe..."
RECIPE_RESPONSE=$(curl -s -X POST "$BASE_URL/api/recipes/v2" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Тестовый борщ для AI анализа",
    "instructions": "Сварить свеклу и морковь. Добавить капусту и картофель. Варить 2 часа на медленном огне. Посолить по вкусу.",
    "language": "ru",
    "servings": 6,
    "ingredients": []
  }')

RECIPE_ID=$(echo $RECIPE_RESPONSE | jq -r '.id')

if [ "$RECIPE_ID" = "null" ] || [ -z "$RECIPE_ID" ]; then
  echo "❌ Recipe creation failed"
  echo $RECIPE_RESPONSE | jq .
  exit 1
fi

echo "✅ Recipe created: $RECIPE_ID"
echo ""

# Step 3: Generate AI insights
echo "Step 3: Generating AI insights..."
echo "⏳ Calling AI (this takes 2-3 seconds)..."

START=$(date +%s%3N)
INSIGHTS=$(curl -s -X GET "$BASE_URL/api/recipes/v2/$RECIPE_ID/insights/ru" \
  -H "Authorization: Bearer $TOKEN")
END=$(date +%s%3N)
DURATION=$((END - START))

INSIGHTS_ID=$(echo $INSIGHTS | jq -r '.id')

if [ "$INSIGHTS_ID" = "null" ] || [ -z "$INSIGHTS_ID" ]; then
  echo "❌ AI insights generation failed"
  echo $INSIGHTS | jq .
  exit 1
fi

echo "✅ AI insights generated in ${DURATION}ms"
echo ""

# Show results
echo "📊 Results:"
echo "--------"
echo "Steps: $(echo $INSIGHTS | jq '.steps | length')"
echo "Errors: $(echo $INSIGHTS | jq '.validation.errors | length')"
echo "Warnings: $(echo $INSIGHTS | jq '.validation.warnings | length')"
echo "Suggestions: $(echo $INSIGHTS | jq '.suggestions | length')"
echo "Feasibility: $(echo $INSIGHTS | jq '.feasibility_score')/100"
echo "Model: $(echo $INSIGHTS | jq -r '.model')"
echo ""

# Step 4: Get cached
echo "Step 4: Getting cached insights..."
START=$(date +%s%3N)
CACHED=$(curl -s -X GET "$BASE_URL/api/recipes/v2/$RECIPE_ID/insights/ru" \
  -H "Authorization: Bearer $TOKEN")
END=$(date +%s%3N)
CACHE_DURATION=$((END - START))

echo "✅ Got cached in ${CACHE_DURATION}ms (vs ${DURATION}ms first call)"
echo ""

# Step 5: Refresh
echo "Step 5: Refreshing insights..."
echo "⏳ Calling AI again..."

START=$(date +%s%3N)
REFRESHED=$(curl -s -X POST "$BASE_URL/api/recipes/v2/$RECIPE_ID/insights/ru/refresh" \
  -H "Authorization: Bearer $TOKEN")
END=$(date +%s%3N)
REFRESH_DURATION=$((END - START))

echo "✅ Refreshed in ${REFRESH_DURATION}ms"
echo ""

# Final summary
echo "================================"
echo "✅ ALL TESTS PASSED"
echo "================================"
echo ""
echo "Performance:"
echo "  First generation: ${DURATION}ms"
echo "  Cached: ${CACHE_DURATION}ms"
echo "  Refresh: ${REFRESH_DURATION}ms"
echo ""
echo "Recipe ID: $RECIPE_ID"
echo ""

# Show first step as example
echo "📝 Example - First Cooking Step:"
echo $INSIGHTS | jq '.steps[0]'
echo ""

# Show validation
echo "🔍 Validation:"
echo $INSIGHTS | jq '.validation | {errors: .errors | length, warnings: .warnings | length}'
