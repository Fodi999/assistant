#!/bin/bash

# Test Improved AI with Validator
# Tests: impossible recipe, dangerous recipe, valid recipe

BASE_URL="http://localhost:8000"

echo "🧪 Testing AI Insights with Rule-Based Validator"
echo "=================================================="
echo ""

# Login
echo "Step 1: Login..."
LOGIN=$(curl -s -X POST "$BASE_URL/api/auth/login" \
  -H "Content-Type: application/json" \
  -d '{"email":"simple_test@fodi.app","password":"test12345"}')

TOKEN=$(echo $LOGIN | jq -r '.access_token')

if [ "$TOKEN" = "null" ] || [ -z "$TOKEN" ]; then
  echo "❌ Login failed"
  exit 1
fi

echo "✅ Logged in"
echo ""

# TEST 1: Impossible Recipe (Cake from vegetables)
echo "════════════════════════════════════════════════"
echo "TEST 1: Невозможный рецепт (Торт из овощей)"
echo "════════════════════════════════════════════════"

RECIPE_1=$(curl -s -X POST "$BASE_URL/api/recipes/v2" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Торт шоколадный",
    "instructions": "Нарезать свеклу и капусту кубиками. Добавить картофель. Запечь 30 минут.",
    "language": "ru",
    "servings": 6,
    "ingredients": []
  }')

RECIPE_1_ID=$(echo $RECIPE_1 | jq -r '.id')

if [ "$RECIPE_1_ID" != "null" ]; then
  echo "Recipe ID: $RECIPE_1_ID"
  
  INSIGHTS_1=$(curl -s -X GET "$BASE_URL/api/recipes/v2/$RECIPE_1_ID/insights/ru" \
    -H "Authorization: Bearer $TOKEN")
  
  SCORE_1=$(echo $INSIGHTS_1 | jq '.insights.feasibility_score')
  ERRORS_1=$(echo $INSIGHTS_1 | jq '.insights.validation.errors | length')
  
  echo ""
  echo "Feasibility Score: $SCORE_1/100"
  echo "Validation Errors: $ERRORS_1"
  
  if [ "$SCORE_1" -lt 30 ]; then
    echo "✅ PASS: AI correctly detected impossible recipe (score < 30)"
  else
    echo "⚠️  WARNING: Score too high for impossible recipe"
  fi
  
  echo ""
  echo "Validation:"
  echo $INSIGHTS_1 | jq '.insights.validation'
fi

echo ""
echo ""

# TEST 2: Dangerous Recipe (Raw meat)
echo "════════════════════════════════════════════════"
echo "TEST 2: Опасный рецепт (Сырое мясо)"
echo "════════════════════════════════════════════════"

RECIPE_2=$(curl -s -X POST "$BASE_URL/api/recipes/v2" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Салат с мясом",
    "instructions": "Нарезать сырое мясо кубиками. Добавить овощи. Подать свежим.",
    "language": "ru",
    "servings": 4,
    "ingredients": []
  }')

RECIPE_2_ID=$(echo $RECIPE_2 | jq -r '.id')

if [ "$RECIPE_2_ID" != "null" ]; then
  echo "Recipe ID: $RECIPE_2_ID"
  
  INSIGHTS_2=$(curl -s -X GET "$BASE_URL/api/recipes/v2/$RECIPE_2_ID/insights/ru" \
    -H "Authorization: Bearer $TOKEN")
  
  SCORE_2=$(echo $INSIGHTS_2 | jq '.insights.feasibility_score')
  ERRORS_2=$(echo $INSIGHTS_2 | jq '.insights.validation.errors | length')
  
  echo ""
  echo "Feasibility Score: $SCORE_2/100"
  echo "Validation Errors: $ERRORS_2"
  
  if [ "$ERRORS_2" -gt 0 ]; then
    echo "✅ PASS: Validator detected safety issue"
  else
    echo "⚠️  WARNING: No errors for dangerous recipe"
  fi
  
  echo ""
  echo "Errors:"
  echo $INSIGHTS_2 | jq '.insights.validation.errors'
fi

echo ""
echo ""

# TEST 3: Valid Recipe (Good borscht)
echo "════════════════════════════════════════════════"
echo "TEST 3: Правильный рецепт (Борщ)"
echo "════════════════════════════════════════════════"

RECIPE_3=$(curl -s -X POST "$BASE_URL/api/recipes/v2" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Борщ украинский классический",
    "instructions": "1. Сварить свеклу и морковь в воде до мягкости (40 минут). 2. Нарезать капусту соломкой и добавить в кастрюлю. 3. Добавить картофель кубиками. 4. Варить на медленном огне 2 часа. 5. Посолить, добавить лавровый лист и чеснок. 6. Подать со сметаной.",
    "language": "ru",
    "servings": 6,
    "ingredients": []
  }')

RECIPE_3_ID=$(echo $RECIPE_3 | jq -r '.id')

if [ "$RECIPE_3_ID" != "null" ]; then
  echo "Recipe ID: $RECIPE_3_ID"
  
  INSIGHTS_3=$(curl -s -X GET "$BASE_URL/api/recipes/v2/$RECIPE_3_ID/insights/ru" \
    -H "Authorization: Bearer $TOKEN")
  
  SCORE_3=$(echo $INSIGHTS_3 | jq '.insights.feasibility_score')
  ERRORS_3=$(echo $INSIGHTS_3 | jq '.insights.validation.errors | length')
  WARNINGS_3=$(echo $INSIGHTS_3 | jq '.insights.validation.warnings | length')
  
  echo ""
  echo "Feasibility Score: $SCORE_3/100"
  echo "Validation Errors: $ERRORS_3"
  echo "Validation Warnings: $WARNINGS_3"
  
  if [ "$SCORE_3" -gt 70 ]; then
    echo "✅ PASS: Good recipe has high feasibility score"
  else
    echo "⚠️  WARNING: Score too low for good recipe"
  fi
  
  echo ""
  echo "Steps:"
  echo $INSIGHTS_3 | jq '.insights.steps | length'
  
  echo ""
  echo "Suggestions:"
  echo $INSIGHTS_3 | jq '.insights.suggestions[0]'
fi

echo ""
echo "=================================================="
echo "✅ Validator Testing Complete"
echo "=================================================="
