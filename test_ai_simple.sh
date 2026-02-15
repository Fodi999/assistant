#!/bin/bash

# Simple AI Test - just test AI generation without dependencies
# Минимальный тест: login → create recipe → AI insights

set -e

BASE_URL="http://localhost:8000"

echo "🧪 Simple AI Insights Test"
echo "=============================="
echo ""

# Step 1: Login or register
echo "Step 1: User login..."
LOGIN=$(curl -s -X POST "$BASE_URL/api/auth/login" \
  -H "Content-Type: application/json" \
  -d '{"email":"simple_test@fodi.app","password":"test12345"}')

TOKEN=$(echo $LOGIN | jq -r '.access_token')

if [ "$TOKEN" = "null" ] || [ -z "$TOKEN" ]; then
  echo "User not found, registering..."
  REGISTER=$(curl -s -X POST "$BASE_URL/api/auth/register" \
    -H "Content-Type: application/json" \
    -d '{
      "email": "simple_test@fodi.app",
      "password": "test12345",
      "name": "Simple Test",
      "restaurant_name": "Test Kitchen"
    }')
  
  TOKEN=$(echo $REGISTER | jq -r '.access_token')
  
  if [ "$TOKEN" = "null" ] || [ -z "$TOKEN" ]; then
    echo "❌ Login/register failed"
    echo $REGISTER | jq .
    exit 1
  fi
fi

echo "✅ Logged in (token: ${TOKEN:0:20}...)"
echo ""

# Step 2: Create recipe WITHOUT ingredients
echo "Step 2: Creating test recipe..."
RECIPE=$(curl -s -X POST "$BASE_URL/api/recipes/v2" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Борщ украинский классический",
    "instructions": "1. Сварить свеклу и морковь до мягкости (40 минут). 2. Нарезать капусту соломкой. 3. Добавить картофель кубиками. 4. Варить на медленном огне 2 часа. 5. Посолить, добавить лавровый лист и чеснок. 6. Подать со сметаной.",
    "language": "ru",
    "servings": 6,
    "ingredients": []
  }')

RECIPE_ID=$(echo $RECIPE | jq -r '.id')

if [ "$RECIPE_ID" = "null" ] || [ -z "$RECIPE_ID" ]; then
  echo "❌ Recipe creation failed"
  echo "$RECIPE"
  exit 1
fi

echo "✅ Recipe created: $RECIPE_ID"
echo ""

# Step 3: Generate AI insights
echo "Step 3: 🤖 Generating AI insights..."
echo "⏳ Calling Groq API (2-3 seconds)..."

START=$(date +%s%3N)
INSIGHTS=$(curl -s -X GET "$BASE_URL/api/recipes/v2/$RECIPE_ID/insights/ru" \
  -H "Authorization: Bearer $TOKEN")
END=$(date +%s%3N)
DURATION=$((END - START))

INSIGHTS_ID=$(echo $INSIGHTS | jq -r '.id')

if [ "$INSIGHTS_ID" = "null" ] || [ -z "$INSIGHTS_ID" ]; then
  echo "❌ AI insights failed"
  echo "$INSIGHTS"
  exit 1
fi

echo "✅ AI insights generated in ${DURATION}ms"
echo ""

# Show results
echo "=============================="
echo "📊 Results"
echo "=============================="
echo ""

STEPS=$(echo $INSIGHTS | jq '.steps | length')
ERRORS=$(echo $INSIGHTS | jq '.validation.errors | length')
WARNINGS=$(echo $INSIGHTS | jq '.validation.warnings | length')
SUGGESTIONS=$(echo $INSIGHTS | jq '.suggestions | length')
SCORE=$(echo $INSIGHTS | jq '.feasibility_score')
MODEL=$(echo $INSIGHTS | jq -r '.model')

echo "Steps: $STEPS"
echo "Errors: $ERRORS"
echo "Warnings: $WARNINGS"
echo "Suggestions: $SUGGESTIONS"
echo "Feasibility: $SCORE/100"
echo "Model: $MODEL"
echo ""

# Test cache
echo "Step 4: Testing cache..."
START=$(date +%s%3N)
curl -s -X GET "$BASE_URL/api/recipes/v2/$RECIPE_ID/insights/ru" \
  -H "Authorization: Bearer $TOKEN" > /dev/null
END=$(date +%s%3N)
CACHE_TIME=$((END - START))

echo "✅ Cached in ${CACHE_TIME}ms (vs ${DURATION}ms first call)"
echo ""

# Show first step
echo "=============================="
echo "🍳 First Cooking Step"
echo "=============================="
echo $INSIGHTS | jq '.steps[0]'
echo ""

echo "✅ TEST PASSED"
echo ""
echo "Recipe ID: $RECIPE_ID"
echo "Insights ID: $INSIGHTS_ID"
