#!/bin/bash

# 🎯 Test: Registered User Search & Add to Inventory
# Flow: 1) Login → 2) Search catalog → 3) Add to inventory
# Target: https://koyeb.app (production)

BACKEND_URL="https://ministerial-yetta-fodi999-c58d8823.koyeb.app"
# BACKEND_URL="http://localhost:8080"  # For local testing

echo "================================="
echo "🧪 User Search & Add to Inventory Test"
echo "================================="
echo "Backend: $BACKEND_URL"
echo ""

# Step 1: Register or Login
echo "📝 Step 1: Register new user"
echo "---"

REGISTER_RESPONSE=$(curl -s -X POST "$BACKEND_URL/api/auth/register" \
  -H "Content-Type: application/json" \
  -d "{
    \"email\": \"user_test_$(date +%s)@example.com\",
    \"password\": \"TestPass123!\",
    \"restaurant_name\": \"Test Restaurant $(date +%s)\",
    \"owner_name\": \"Test User\"
  }")

echo "Response:"
echo "$REGISTER_RESPONSE" | jq .
echo ""

ACCESS_TOKEN=$(echo "$REGISTER_RESPONSE" | jq -r '.access_token // empty')

if [ -z "$ACCESS_TOKEN" ]; then
  echo "❌ Registration failed! No token received."
  echo "Full response:"
  echo "$REGISTER_RESPONSE" | jq .
  exit 1
fi

echo "✅ Registration successful!"
echo "Token (first 50 chars): ${ACCESS_TOKEN:0:50}..."
echo ""

# Step 2: Get user info (to confirm language setting)
echo "👤 Step 2: Get user info"
echo "---"

USER_INFO=$(curl -s -X GET "$BACKEND_URL/api/me" \
  -H "Authorization: Bearer $ACCESS_TOKEN")

echo "$USER_INFO" | jq .
USER_ID=$(echo "$USER_INFO" | jq -r '.id // empty')
LANGUAGE=$(echo "$USER_INFO" | jq -r '.language // "en"')

echo "✅ User ID: $USER_ID"
echo "✅ Language: $LANGUAGE"
echo ""

# Step 3: Search catalog for "яблоко" (apple in Russian)
echo "🔍 Step 3: Search catalog for 'яблоко' (apple)"
echo "---"
echo "GET /api/catalog/ingredients?q=яблоко"
echo ""

SEARCH_RESPONSE=$(curl -s -X GET "$BACKEND_URL/api/catalog/ingredients?q=яблоко" \
  -H "Authorization: Bearer $ACCESS_TOKEN")

echo "Response:"
echo "$SEARCH_RESPONSE" | jq .
echo ""

# Extract first result
INGREDIENT_ID=$(echo "$SEARCH_RESPONSE" | jq -r '.ingredients[0].id // empty')
INGREDIENT_NAME=$(echo "$SEARCH_RESPONSE" | jq -r '.ingredients[0].name // "Unknown"')

if [ -z "$INGREDIENT_ID" ]; then
  echo "❌ No ingredients found for 'яблоко'"
  echo "Trying English search for 'apple'..."
  echo ""
  
  SEARCH_RESPONSE=$(curl -s -X GET "$BACKEND_URL/api/catalog/ingredients?q=apple" \
    -H "Authorization: Bearer $ACCESS_TOKEN")
  
  echo "$SEARCH_RESPONSE" | jq .
  INGREDIENT_ID=$(echo "$SEARCH_RESPONSE" | jq -r '.ingredients[0].id // empty')
  INGREDIENT_NAME=$(echo "$SEARCH_RESPONSE" | jq -r '.ingredients[0].name // "Unknown"')
  
  if [ -z "$INGREDIENT_ID" ]; then
    echo "❌ No ingredients found for 'apple' either"
    exit 1
  fi
fi

echo "✅ Found: $INGREDIENT_NAME"
echo "✅ Ingredient ID: $INGREDIENT_ID"
echo ""

# Step 4: Add product to inventory
echo "➕ Step 4: Add product to inventory"
echo "---"
echo "POST /api/inventory/products"
echo ""

ADD_RESPONSE=$(curl -s -X POST "$BACKEND_URL/api/inventory/products" \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $ACCESS_TOKEN" \
  -d "{
    \"catalog_ingredient_id\": \"$INGREDIENT_ID\",
    \"price_per_unit_cents\": 15000,
    \"quantity\": 10.5,
    \"received_at\": \"$(date -u +'%Y-%m-%dT%H:%M:%SZ')\",
    \"expires_at\": \"$(date -u -d '+30 days' +'%Y-%m-%dT%H:%M:%SZ' 2>/dev/null || date -u -v+30d +'%Y-%m-%dT%H:%M:%SZ')\"
  }")

echo "Response:"
echo "$ADD_RESPONSE" | jq .
echo ""

# Check if successful
PRODUCT_ID=$(echo "$ADD_RESPONSE" | jq -r '.id // empty')

if [ -n "$PRODUCT_ID" ]; then
  echo "✅ Product added successfully!"
  echo "✅ Product ID: $PRODUCT_ID"
  echo ""
  
  # Step 5: List all inventory products
  echo "📋 Step 5: List all inventory products"
  echo "---"
  echo "GET /api/inventory/products"
  echo ""
  
  LIST_RESPONSE=$(curl -s -X GET "$BACKEND_URL/api/inventory/products" \
    -H "Authorization: Bearer $ACCESS_TOKEN")
  
  echo "$LIST_RESPONSE" | jq .
  
  TOTAL_PRODUCTS=$(echo "$LIST_RESPONSE" | jq '.[] | length' 2>/dev/null || echo "0")
  echo ""
  echo "✅ Total products in inventory: $TOTAL_PRODUCTS"
else
  echo "❌ Failed to add product!"
  exit 1
fi

echo ""
echo "================================="
echo "✅ All tests passed!"
echo "================================="
