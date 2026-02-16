#!/bin/bash

# Business Flow Killer Test: Production & Inventory Deduction
# Purpose: Prove that selling a dish automatically deducts ingredients from inventory.

API_URL=${1:-"http://localhost:8000"}

echo "🚀 Starting Business Flow Killer Test (Production Deduction)"
echo "📍 Target: $API_URL"
echo "----------------------------------------------------"

# 1. Create Tenant
echo "👥 Creating Tenant..."
RESPONSE=$(curl -s -X POST "$API_URL/api/auth/register" \
  -H "Content-Type: application/json" \
  -d '{
    "email": "chef_production_'"$RANDOM"'@saas-test.com",
    "password": "StrongPassword123",
    "restaurant_name": "Sushi Bar",
    "owner_name": "Chef",
    "language": "en"
  }')

TOKEN=$(echo $RESPONSE | jq -r '.access_token')
if [ "$TOKEN" == "null" ] || [ -z "$TOKEN" ]; then echo "❌ Auth Failed"; exit 1; fi

# 2. Find Salmon ID
echo "🔍 Finding Salmon in catalog..."
SALMON_ID=$(curl -s -X GET "$API_URL/api/catalog/ingredients?q=Salmon" -H "Authorization: Bearer $TOKEN" | jq -r '.ingredients[0].id')
if [ "$SALMON_ID" == "null" ]; then
    # Fallback to Milk if Salmon not found
    SALMON_ID=$(curl -s -X GET "$API_URL/api/catalog/ingredients?q=Milk" -H "Authorization: Bearer $TOKEN" | jq -r '.ingredients[0].id')
    PRODUCT_NAME="Milk"
else
    PRODUCT_NAME="Salmon"
fi
echo "✅ Found $PRODUCT_NAME ID: $SALMON_ID"

# 3. Add 10 kg to Inventory
echo "📦 Adding 10.0 kg of $PRODUCT_NAME to inventory..."
curl -s -X POST "$API_URL/api/inventory/products" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "catalog_ingredient_id": "'"$SALMON_ID"'",
    "price_per_unit_cents": 2500,
    "quantity": 10.0
  }' > /dev/null

# 4. Create Recipe "Philadelphia" (200g per portion)
echo "📜 Creating Recipe: Philadelphia Roll (200g per portion)..."
RECIPE_RESPONSE=$(curl -s -X POST "$API_URL/api/recipes" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Philadelphia Roll",
    "servings": 1,
    "ingredients": [
      {
        "catalog_ingredient_id": "'"$SALMON_ID"'",
        "quantity": 0.2
      }
    ]
  }')
RECIPE_ID=$(echo $RECIPE_RESPONSE | jq -r '.id')
echo "✅ Recipe Created: $RECIPE_ID"

# 5. Create Dish from Recipe (Menu Item)
echo "🍽️ Adding 'Philadelphia Roll' to Menu..."
DISH_RESPONSE=$(curl -s -X POST "$API_URL/api/dishes" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "recipe_id": "'"$RECIPE_ID"'",
    "name": "Philadelphia Roll",
    "selling_price_cents": 1500
  }')
DISH_ID=$(echo $DISH_RESPONSE | jq -r '.id')
echo "✅ Dish Added to Menu: $DISH_ID"

# 6. Record Sale of 5 rolls
echo "💰 SELLING 5 PORTIONS..."
curl -s -X POST "$API_URL/api/menu-engineering/sales" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "dish_id": "'"$DISH_ID"'",
    "quantity": 5,
    "selling_price_cents": 1500,
    "recipe_cost_cents": 500
  }' > /dev/null
echo "✅ Sale Recorded."

# 7. Verification: Remaining Inventory should be 10 - (5 * 0.2) = 9.0 kg
echo "----------------------------------------------------"
echo "🔍 Verifying Inventory Deduction..."
REMAINING_QTY=$(curl -s -X GET "$API_URL/api/inventory/products" \
  -H "Authorization: Bearer $TOKEN" | jq -r '.[] | select(.product.id == "'"$SALMON_ID"'") | .quantity')

echo "📊 INITIAL: 10.0 kg"
echo "📊 USED:    1.0 kg (5 portions x 0.2 kg)"
echo "📊 ACTUAL REMAINING: $REMAINING_QTY kg"

if (( $(echo "$REMAINING_QTY == 9.0" | bc -l) )); then
    echo "----------------------------------------------------"
    echo "🏆 BUSINESS FLOW KILLER TEST: PASSED! 🏆"
    echo "The system successfully deducted ingredients from inventory."
else
    echo "----------------------------------------------------"
    echo "💀 BUSINESS FLOW KILLER TEST: FAILED! 💀"
    exit 1
fi
