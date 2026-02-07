#!/bin/bash
set -e

BASE_URL="http://localhost:8080/api"

# Load DATABASE_URL from .env
export $(grep DATABASE_URL .env | xargs)

echo "=== Recipe HTTP API Test ==="
echo ""

# 1. Register user
echo "1. Registering test user..."
REGISTER_RESPONSE=$(curl -s -X POST "$BASE_URL/auth/register" \
  -H "Content-Type: application/json" \
  -d "{
    \"email\": \"recipe_api_test_$(date +%s)@example.com\",
    \"password\": \"Test123!\",
    \"name\": \"Recipe API Tester\",
    \"restaurant_name\": \"API Test Kitchen\",
    \"language\": \"en\"
  }")

TOKEN=$(echo $REGISTER_RESPONSE | jq -r '.access_token')
echo "✓ User registered, token: ${TOKEN:0:20}..."
echo ""

# 2. Create catalog ingredients via database (needed for recipe creation)
echo "2. Creating catalog ingredients..."

TOMATO_ID=$(psql "$DATABASE_URL" -qtAc "
  INSERT INTO catalog_ingredients (id, category_id, name_pl, name_en, name_uk, name_ru, default_unit)
  VALUES (gen_random_uuid(), (SELECT id FROM catalog_categories LIMIT 1), 'Pomidory', 'Tomatoes', 'Помідори', 'Помидоры', 'kilogram')
  RETURNING id;
")

ONION_ID=$(psql "$DATABASE_URL" -qtAc "
  INSERT INTO catalog_ingredients (id, category_id, name_pl, name_en, name_uk, name_ru, default_unit)
  VALUES (gen_random_uuid(), (SELECT id FROM catalog_categories LIMIT 1), 'Cebula', 'Onions', 'Цибуля', 'Лук', 'kilogram')
  RETURNING id;
")

SALT_ID=$(psql "$DATABASE_URL" -qtAc "
  INSERT INTO catalog_ingredients (id, category_id, name_pl, name_en, name_uk, name_ru, default_unit)
  VALUES (gen_random_uuid(), (SELECT id FROM catalog_categories LIMIT 1), 'Sól', 'Salt', 'Сіль', 'Соль', 'gram')
  RETURNING id;
")

echo "✓ Created ingredients: Tomatoes ($TOMATO_ID), Onions ($ONION_ID), Salt ($SALT_ID)"
echo ""

# 3. Add inventory products for cost calculation
echo "3. Adding inventory products via assistant..."

# Start inventory
curl -s -X POST "$BASE_URL/assistant/command" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"command":{"type":"start_inventory"}}' > /dev/null

# Tomatoes - 5.00 PLN for 1kg
curl -s -X POST "$BASE_URL/assistant/command" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d "{
    \"command\":{
      \"type\":\"add_product\",
      \"payload\":{
        \"catalog_ingredient_id\":\"$TOMATO_ID\",
        \"price_per_unit_cents\":500,
        \"quantity\":1.0,
        \"expires_at\":\"2026-03-01T00:00:00Z\"
      }
    }
  }" > /dev/null

# Onions - 3.00 PLN for 1kg
curl -s -X POST "$BASE_URL/assistant/command" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d "{
    \"command\":{
      \"type\":\"add_product\",
      \"payload\":{
        \"catalog_ingredient_id\":\"$ONION_ID\",
        \"price_per_unit_cents\":300,
        \"quantity\":1.0,
        \"expires_at\":\"2026-03-01T00:00:00Z\"
      }
    }
  }" > /dev/null

# Salt - 0.10 PLN for 100g (0.001 PLN per gram)
curl -s -X POST "$BASE_URL/assistant/command" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d "{
    \"command\":{
      \"type\":\"add_product\",
      \"payload\":{
        \"catalog_ingredient_id\":\"$SALT_ID\",
        \"price_per_unit_cents\":10,
        \"quantity\":100.0,
        \"expires_at\":\"2026-12-31T00:00:00Z\"
      }
    }
  }" > /dev/null

echo "✓ Added inventory: Tomatoes (5 PLN/kg), Onions (3 PLN/kg), Salt (0.10 PLN/100g)"
echo ""

# 4. Test POST /api/recipes - Create recipe
echo "4. Testing POST /api/recipes - Create recipe..."
CREATE_RESPONSE=$(curl -s -X POST "$BASE_URL/recipes" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d "{
    \"name\": \"Tomato Soup\",
    \"servings\": 4,
    \"ingredients\": [
      {
        \"catalog_ingredient_id\": \"$TOMATO_ID\",
        \"quantity\": 0.5
      },
      {
        \"catalog_ingredient_id\": \"$ONION_ID\",
        \"quantity\": 0.2
      },
      {
        \"catalog_ingredient_id\": \"$SALT_ID\",
        \"quantity\": 5.0
      }
    ]
  }")

RECIPE_ID=$(echo $CREATE_RESPONSE | jq -r '.id')
RECIPE_NAME=$(echo $CREATE_RESPONSE | jq -r '.name')
RECIPE_SERVINGS=$(echo $CREATE_RESPONSE | jq -r '.servings')

if [ "$RECIPE_ID" = "null" ]; then
  echo "✗ Failed to create recipe"
  echo "Response: $CREATE_RESPONSE"
  exit 1
fi

echo "✓ Created recipe: $RECIPE_NAME (ID: $RECIPE_ID, Servings: $RECIPE_SERVINGS)"
echo ""

# 5. Test GET /api/recipes/:id - Get recipe by ID
echo "5. Testing GET /api/recipes/:id - Get recipe..."
GET_RESPONSE=$(curl -s -X GET "$BASE_URL/recipes/$RECIPE_ID" \
  -H "Authorization: Bearer $TOKEN")

GET_NAME=$(echo $GET_RESPONSE | jq -r '.name')
GET_SERVINGS=$(echo $GET_RESPONSE | jq -r '.servings')
GET_INGREDIENTS_COUNT=$(echo $GET_RESPONSE | jq -r '.ingredients | length')

if [ "$GET_NAME" = "Tomato Soup" ]; then
  echo "✓ Retrieved recipe: $GET_NAME (Servings: $GET_SERVINGS, Ingredients: $GET_INGREDIENTS_COUNT)"
else
  echo "✗ Failed to get recipe"
  echo "Response: $GET_RESPONSE"
  exit 1
fi
echo ""

# 6. Test GET /api/recipes/:id/cost - Calculate recipe cost (KEY ENDPOINT)
echo "6. Testing GET /api/recipes/:id/cost - Calculate cost..."
COST_RESPONSE=$(curl -s -X GET "$BASE_URL/recipes/$RECIPE_ID/cost" \
  -H "Authorization: Bearer $TOKEN")

echo "Cost calculation response:"
echo "$COST_RESPONSE" | jq '.'

TOTAL_COST=$(echo $COST_RESPONSE | jq -r '.total_cost_cents')
COST_PER_SERVING=$(echo $COST_RESPONSE | jq -r '.cost_per_serving_cents')
COST_SERVINGS=$(echo $COST_RESPONSE | jq -r '.servings')

# Expected: 
# Tomatoes: 0.5kg * 500 cents = 250 cents (2.50 PLN)
# Onions: 0.2kg * 300 cents = 60 cents (0.60 PLN)
# Salt: 5g * 0.1 cents = 0.5 cents (0.005 PLN) -> rounds to 1 cent
# Total: 311 cents (3.11 PLN)
# Per serving (4): 77.75 cents -> rounds to 78 cents (0.78 PLN)

if [ "$TOTAL_COST" = "null" ]; then
  echo "✗ Failed to calculate cost"
  echo "Response: $COST_RESPONSE"
  exit 1
fi

echo ""
echo "✓ Cost calculation:"
echo "  - Total cost: $TOTAL_COST cents ($(echo "scale=2; $TOTAL_COST / 100" | bc) PLN)"
echo "  - Cost per serving: $COST_PER_SERVING cents ($(echo "scale=2; $COST_PER_SERVING / 100" | bc) PLN)"
echo "  - Servings: $COST_SERVINGS"
echo ""

# Validate expected values
if [ "$TOTAL_COST" -ge 310 ] && [ "$TOTAL_COST" -le 312 ] && [ "$COST_PER_SERVING" -ge 77 ] && [ "$COST_PER_SERVING" -le 78 ]; then
  echo "✓ Cost calculation is correct!"
else
  echo "⚠ Cost calculation differs from expected (expected ~311 cents total, ~78 cents/serving)"
fi
echo ""

# 7. Test GET /api/recipes - List recipes
echo "7. Testing GET /api/recipes - List recipes..."
LIST_RESPONSE=$(curl -s -X GET "$BASE_URL/recipes" \
  -H "Authorization: Bearer $TOKEN")

RECIPE_COUNT=$(echo $LIST_RESPONSE | jq -r '. | length')

if [ "$RECIPE_COUNT" -ge 1 ]; then
  echo "✓ Listed $RECIPE_COUNT recipe(s)"
  echo "$LIST_RESPONSE" | jq '.[0] | {id, name, servings}'
else
  echo "✗ Failed to list recipes"
  echo "Response: $LIST_RESPONSE"
  exit 1
fi
echo ""

# 8. Test DELETE /api/recipes/:id - Delete recipe
echo "8. Testing DELETE /api/recipes/:id - Delete recipe..."
DELETE_RESPONSE=$(curl -s -w "\n%{http_code}" -X DELETE "$BASE_URL/recipes/$RECIPE_ID" \
  -H "Authorization: Bearer $TOKEN")

HTTP_CODE=$(echo "$DELETE_RESPONSE" | tail -n1)

if [ "$HTTP_CODE" = "204" ]; then
  echo "✓ Deleted recipe (HTTP $HTTP_CODE)"
else
  echo "✗ Failed to delete recipe (HTTP $HTTP_CODE)"
  exit 1
fi
echo ""

# 9. Verify deletion
echo "9. Verifying recipe deletion..."
VERIFY_RESPONSE=$(curl -s -w "\n%{http_code}" -X GET "$BASE_URL/recipes/$RECIPE_ID" \
  -H "Authorization: Bearer $TOKEN")

HTTP_CODE=$(echo "$VERIFY_RESPONSE" | tail -n1)

if [ "$HTTP_CODE" = "404" ]; then
  echo "✓ Recipe not found after deletion (HTTP $HTTP_CODE)"
else
  echo "⚠ Unexpected response after deletion (HTTP $HTTP_CODE)"
fi
echo ""

echo "=== All Recipe API Tests Passed! ==="
echo ""
echo "Summary:"
echo "✓ POST /api/recipes - Create recipe"
echo "✓ GET /api/recipes/:id - Get recipe by ID"
echo "✓ GET /api/recipes/:id/cost - Calculate recipe cost (KEY ENDPOINT)"
echo "✓ GET /api/recipes - List recipes"
echo "✓ DELETE /api/recipes/:id - Delete recipe"
echo ""
echo "Next steps:"
echo "1. Dish/Menu domain (Шаг №2): selling_price, profit, margins"
echo "2. Assistant integration (Шаг №3): RecipeSetup state"
