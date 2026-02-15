#!/bin/bash
set -e

echo "🧪 Testing Recipe V2 on Koyeb Production"
echo "=========================================="

BASE_URL="https://ministerial-yetta-fodi999-c58d8823.koyeb.app"

# 1. Register test user
echo ""
echo "1️⃣ Registering test user..."
REGISTER_RESPONSE=$(curl -s -X POST "$BASE_URL/api/auth/register" \
  -H "Content-Type: application/json" \
  -d '{
    "email":"recipe-test@test.com",
    "password":"testpass123",
    "name":"Recipe Tester",
    "restaurant_name":"Test Kitchen"
  }')

TOKEN=$(echo $REGISTER_RESPONSE | jq -r '.access_token // empty')

if [ -z "$TOKEN" ]; then
  echo "❌ Registration failed. Trying login instead..."
  LOGIN_RESPONSE=$(curl -s -X POST "$BASE_URL/api/auth/login" \
    -H "Content-Type: application/json" \
    -d '{"email":"recipe-test@test.com","password":"testpass123"}')
  TOKEN=$(echo $LOGIN_RESPONSE | jq -r '.access_token // empty')
fi

if [ -z "$TOKEN" ]; then
  echo "❌ Failed to get token"
  exit 1
fi

echo "✅ Got auth token: ${TOKEN:0:20}..."

# 2. Create catalog ingredient
echo ""
echo "2️⃣ Creating catalog ingredient (Beets)..."
INGREDIENT=$(curl -s -X POST "$BASE_URL/api/admin/catalog" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"name_input":"Beets"}')

INGREDIENT_ID=$(echo $INGREDIENT | jq -r '.id // empty')

if [ -z "$INGREDIENT_ID" ]; then
  echo "⚠️  Ingredient creation failed or already exists. Using existing..."
  # Try to get first ingredient from catalog
  CATALOG=$(curl -s -X GET "$BASE_URL/api/catalog?page=1&limit=1" \
    -H "Authorization: Bearer $TOKEN")
  INGREDIENT_ID=$(echo $CATALOG | jq -r '.items[0].id // empty')
fi

if [ -z "$INGREDIENT_ID" ]; then
  echo "❌ No ingredients available"
  exit 1
fi

echo "✅ Using ingredient ID: $INGREDIENT_ID"
echo "   Name: $(echo $INGREDIENT | jq -r '.name_en // "N/A"')"

# 3. Create Recipe V2
echo ""
echo "3️⃣ Creating Recipe V2 (Ukrainian Borscht)..."
RECIPE=$(curl -s -X POST "$BASE_URL/api/recipes/v2" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d "{
    \"name\": \"Борщ украинский\",
    \"instructions\": \"Сварить свеклу, морковь и капусту. Добавить мясо и картофель. Варить 2 часа на слабом огне.\",
    \"language\": \"ru\",
    \"servings\": 6,
    \"ingredients\": [{
      \"catalog_ingredient_id\": \"$INGREDIENT_ID\",
      \"quantity\": 0.5,
      \"unit\": \"kg\"
    }]
  }")

RECIPE_ID=$(echo $RECIPE | jq -r '.id // empty')

if [ -z "$RECIPE_ID" ]; then
  echo "❌ Recipe creation failed:"
  echo $RECIPE | jq '.'
  exit 1
fi

echo "✅ Recipe created!"
echo "   ID: $RECIPE_ID"
echo "   Name (RU): $(echo $RECIPE | jq -r '.name')"
echo "   Status: $(echo $RECIPE | jq -r '.status')"

# 4. Wait for translations
echo ""
echo "4️⃣ Waiting for AI translations (10 seconds)..."
sleep 10

# 5. Get recipe with translations
echo ""
echo "5️⃣ Fetching recipe with translations..."
RECIPE_FULL=$(curl -s -X GET "$BASE_URL/api/recipes/v2/$RECIPE_ID" \
  -H "Authorization: Bearer $TOKEN")

echo ""
echo "📊 Translation Results:"
echo "======================"
echo $RECIPE_FULL | jq '{
  id,
  name,
  status,
  translations: .translations | map({
    language,
    name,
    instructions: (.instructions[:80] + "...")
  })
}'

# Check if translations exist
TRANSLATION_COUNT=$(echo $RECIPE_FULL | jq '.translations | length')

if [ "$TRANSLATION_COUNT" -gt 0 ]; then
  echo ""
  echo "🎉 SUCCESS! Recipe V2 with AI translations working on production!"
  echo "   Translations found: $TRANSLATION_COUNT"
else
  echo ""
  echo "⚠️  Recipe created but no translations yet. May need more time."
fi

echo ""
echo "✅ Test complete!"
