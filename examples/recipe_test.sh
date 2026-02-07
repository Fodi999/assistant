#!/bin/bash
set -e

BASE_URL="http://localhost:8080/api"

# Load DATABASE_URL from .env
export $(grep DATABASE_URL .env | xargs)

echo "=== Recipe Domain Integration Test ==="
echo ""

# 1. Register user
echo "1. Registering test user..."
REGISTER_RESPONSE=$(curl -s -X POST "$BASE_URL/auth/register" \
  -H "Content-Type: application/json" \
  -d "{
    \"email\": \"recipe_test_$(date +%s)@example.com\",
    \"password\": \"Test123!\",
    \"name\": \"Recipe Tester\",
    \"restaurant_name\": \"Test Kitchen\",
    \"language\": \"en\"
  }")

TOKEN=$(echo $REGISTER_RESPONSE | jq -r '.access_token')
echo "✓ User registered, token: ${TOKEN:0:20}..."
echo ""

# 2. Create catalog ingredients first
echo "2. Creating catalog ingredients..."

# Tomatoes
TOMATO_ID=$(psql "$DATABASE_URL" -qtAc "
  INSERT INTO catalog_ingredients (id, category_id, name_pl, name_en, name_uk, name_ru, default_unit)
  VALUES (gen_random_uuid(), (SELECT id FROM catalog_categories LIMIT 1), 'Pomidory', 'Tomatoes', 'Помідори', 'Помидоры', 'kilogram')
  RETURNING id;
")

# Onions
ONION_ID=$(psql "$DATABASE_URL" -qtAc "
  INSERT INTO catalog_ingredients (id, category_id, name_pl, name_en, name_uk, name_ru, default_unit)
  VALUES (gen_random_uuid(), (SELECT id FROM catalog_categories LIMIT 1), 'Cebula', 'Onions', 'Цибуля', 'Лук', 'kilogram')
  RETURNING id;
")

echo "✓ Created ingredients: Tomatoes ($TOMATO_ID), Onions ($ONION_ID)"
echo ""

# 3. Add inventory products for cost calculation
echo "3. Adding inventory products..."
curl -s -X POST "$BASE_URL/assistant/command" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"command":{"type":"start_inventory"}}' > /dev/null

# Tomatoes - 5 PLN for 1kg
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

# Onions - 3 PLN for 1kg
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

curl -s -X POST "$BASE_URL/assistant/command" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"command":{"type":"finish_inventory"}}' > /dev/null

echo "✓ Added 2 inventory products (tomatoes and onions)"
echo ""

# 4. Create a recipe (direct SQL - API will be implemented later)
echo "4. Creating recipe 'Tomato Soup'..."
RECIPE_ID=$(uuidgen | tr '[:upper:]' '[:lower:]')
USER_ID=$(psql "$DATABASE_URL" -t -c "SELECT id FROM users ORDER BY created_at DESC LIMIT 1;" | xargs)
TENANT_ID=$(psql "$DATABASE_URL" -t -c "SELECT tenant_id FROM users WHERE id='$USER_ID';" | xargs)

psql "$DATABASE_URL" -c "
INSERT INTO recipes (id, user_id, tenant_id, name, servings)
VALUES ('$RECIPE_ID', '$USER_ID', '$TENANT_ID', 'Tomato Soup', 4);
" > /dev/null

# Add recipe ingredients using real IDs
psql "$DATABASE_URL" -c "
INSERT INTO recipe_ingredients (recipe_id, catalog_ingredient_id, quantity)
VALUES 
  ('$RECIPE_ID', '$TOMATO_ID', 0.5),
  ('$RECIPE_ID', '$ONION_ID', 0.2);
" > /dev/null

echo "✓ Recipe created with ID: $RECIPE_ID"
echo ""

# 5. Verify recipe in database
echo "5. Verifying recipe in database..."
RECIPE_DATA=$(psql "$DATABASE_URL" -t -c "
SELECT r.name, r.servings, COUNT(ri.id) as ingredients_count
FROM recipes r
LEFT JOIN recipe_ingredients ri ON r.id = ri.recipe_id
WHERE r.id = '$RECIPE_ID'
GROUP BY r.name, r.servings;
")

echo "Recipe data: $RECIPE_DATA"
echo ""

# 6. Verify ingredients
echo "6. Verifying recipe ingredients..."
psql "$DATABASE_URL" -c "
SELECT 
  ci.name_en as ingredient,
  ri.quantity as qty
FROM recipe_ingredients ri
JOIN catalog_ingredients ci ON ri.catalog_ingredient_id = ci.id
WHERE ri.recipe_id = '$RECIPE_ID';
"
echo ""

# 7. Calculate expected cost
echo "7. Expected cost calculation:"
echo "  - Tomatoes: 0.5 kg × 5.00 PLN/kg = 2.50 PLN"
echo "  - Onions:   0.2 kg × 3.00 PLN/kg = 0.60 PLN"
echo "  - Total:                           3.10 PLN"
echo "  - Per serving (4 servings):        0.78 PLN"
echo ""

# 8. Test constraints
echo "8. Testing database constraints..."

# Try to create recipe with zero servings (should fail)
echo -n "  - Testing servings > 0 constraint... "
if psql "$DATABASE_URL" -c "
INSERT INTO recipes (id, user_id, tenant_id, name, servings)
VALUES (gen_random_uuid(), '$USER_ID', '$TENANT_ID', 'Bad Recipe', 0);
" 2>&1 | grep -q "violates check constraint"; then
  echo "✓ Constraint works"
else
  echo "✗ Constraint failed"
fi

# Try to add duplicate ingredient (should fail)
echo -n "  - Testing unique ingredient constraint... "
if psql "$DATABASE_URL" -c "
INSERT INTO recipe_ingredients (recipe_id, catalog_ingredient_id, quantity)
VALUES ('$RECIPE_ID', '$TOMATO_ID', 1.0);
" 2>&1 | grep -q "duplicate key value"; then
  echo "✓ Constraint works"
else
  echo "✗ Constraint failed"
fi
echo ""

# 9. Test CASCADE delete
echo "9. Testing CASCADE delete..."
TEMP_RECIPE_ID=$(uuidgen | tr '[:upper:]' '[:lower:]')
psql "$DATABASE_URL" -c "
INSERT INTO recipes (id, user_id, tenant_id, name, servings)
VALUES ('$TEMP_RECIPE_ID', '$USER_ID', '$TENANT_ID', 'Temp Recipe', 2);

INSERT INTO recipe_ingredients (recipe_id, catalog_ingredient_id, quantity)
VALUES ('$TEMP_RECIPE_ID', '$TOMATO_ID', 1.0);
" > /dev/null

BEFORE_COUNT=$(psql "$DATABASE_URL" -t -c "SELECT COUNT(*) FROM recipe_ingredients WHERE recipe_id='$TEMP_RECIPE_ID';" | xargs)
echo "  Ingredients before delete: $BEFORE_COUNT"

psql "$DATABASE_URL" -c "DELETE FROM recipes WHERE id='$TEMP_RECIPE_ID';" > /dev/null

AFTER_COUNT=$(psql "$DATABASE_URL" -t -c "SELECT COUNT(*) FROM recipe_ingredients WHERE recipe_id='$TEMP_RECIPE_ID';" | xargs)
echo "  Ingredients after delete: $AFTER_COUNT"

if [ "$AFTER_COUNT" = "0" ]; then
  echo "✓ CASCADE delete works correctly"
else
  echo "✗ CASCADE delete failed"
fi
echo ""

# 10. Test updated_at trigger
echo "10. Testing updated_at trigger..."
BEFORE_UPDATE=$(psql "$DATABASE_URL" -t -c "SELECT updated_at FROM recipes WHERE id='$RECIPE_ID';" | xargs)
sleep 2

# Update recipe name
psql "$DATABASE_URL" -c "UPDATE recipes SET name='Tomato Soup v2' WHERE id='$RECIPE_ID';" > /dev/null

AFTER_UPDATE=$(psql "$DATABASE_URL" -t -c "SELECT updated_at FROM recipes WHERE id='$RECIPE_ID';" | xargs)

if [ "$BEFORE_UPDATE" != "$AFTER_UPDATE" ]; then
  echo "✓ updated_at trigger works"
  echo "  Before: $BEFORE_UPDATE"
  echo "  After:  $AFTER_UPDATE"
else
  echo "✗ updated_at trigger failed"
fi
echo ""

echo "=== Recipe Domain Test Complete ==="
