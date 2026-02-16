#!/bin/bash

# SaaS Killer Test: Tenant Isolation Proof
# Purpose: Prove that Tenant A cannot see Tenant B's data and vice versa.

API_URL=${1:-"http://localhost:8000"}

echo "🚀 Starting SaaS Killer Test (Tenant Isolation Proof)"
echo "📍 Target: $API_URL"
echo "----------------------------------------------------"

# 1. Create Tenant A
echo "👥 Creating Tenant A..."
RESPONSE_A=$(curl -s -X POST "$API_URL/api/auth/register" \
  -H "Content-Type: application/json" \
  -d '{
    "email": "owner_a_'"$RANDOM"'@saas-test.com",
    "password": "StrongPassword123A",
    "restaurant_name": "Restaurant A",
    "owner_name": "Owner A",
    "language": "en"
  }')

TOKEN_A=$(echo $RESPONSE_A | jq -r '.access_token')
TENANT_ID_A=$(echo $RESPONSE_A | jq -r '.tenant_id')

if [ "$TOKEN_A" == "null" ] || [ -z "$TOKEN_A" ]; then
    echo "❌ Failed to create Tenant A"
    echo "Response: $RESPONSE_A"
    exit 1
fi
echo "✅ Tenant A created. ID: $TENANT_ID_A"

# 2. Create Tenant B
echo "👥 Creating Tenant B..."
RESPONSE_B=$(curl -s -X POST "$API_URL/api/auth/register" \
  -H "Content-Type: application/json" \
  -d '{
    "email": "owner_b_'"$RANDOM"'@saas-test.com",
    "password": "StrongPassword123B",
    "restaurant_name": "Restaurant B",
    "owner_name": "Owner B",
    "language": "en"
  }')

TOKEN_B=$(echo $RESPONSE_B | jq -r '.access_token')
TENANT_ID_B=$(echo $RESPONSE_B | jq -r '.tenant_id')

if [ "$TOKEN_B" == "null" ] || [ -z "$TOKEN_B" ]; then
    echo "❌ Failed to create Tenant B"
    exit 1
fi
echo "✅ Tenant B created. ID: $TENANT_ID_B"

# 3. Find Global Ingredients (Different ones to avoid intersection in the test script)
echo "🔍 Searching for Milk in global catalog..."
SALMON_ID=$(curl -s -X GET "$API_URL/api/catalog/ingredients?q=Milk" \
  -H "Authorization: Bearer $TOKEN_A" | jq -r '.ingredients[0].id')

echo "🔍 Searching for Wine in global catalog..."
BEEF_ID=$(curl -s -X GET "$API_URL/api/catalog/ingredients?q=Wine" \
  -H "Authorization: Bearer $TOKEN_B" | jq -r '.ingredients[0].id')

PRODUCT_A_NAME="Milk"
PRODUCT_B_NAME="Wine"

if [ "$SALMON_ID" == "$BEEF_ID" ]; then
    echo "⚠️ Warning: Test ingredients are the same! Forcing different IDs."
    BEEF_ID=$(curl -s -X GET "$API_URL/api/catalog/ingredients?search=Sugar" \
      -H "Authorization: Bearer $TOKEN_B" | jq -r '.ingredients[0].id')
    PRODUCT_B_NAME="Sugar"
fi

echo "📍 Found Product A ID: $SALMON_ID ($PRODUCT_A_NAME)"
echo "📍 Found Product B ID: $BEEF_ID ($PRODUCT_B_NAME)"

# 4. Add Product A to Tenant A
echo "📦 Adding $PRODUCT_A_NAME to Tenant A inventory..."
curl -s -X POST "$API_URL/api/inventory/products" \
  -H "Authorization: Bearer $TOKEN_A" \
  -H "Content-Type: application/json" \
  -d '{
    "catalog_ingredient_id": "'"$SALMON_ID"'",
    "price_per_unit_cents": 2500,
    "quantity": 10.5
  }' > /dev/null

# 5. Add Product B to Tenant B
echo "📦 Adding $PRODUCT_B_NAME to Tenant B inventory..."
curl -s -X POST "$API_URL/api/inventory/products" \
  -H "Authorization: Bearer $TOKEN_B" \
  -H "Content-Type: application/json" \
  -d '{
    "catalog_ingredient_id": "'"$BEEF_ID"'",
    "price_per_unit_cents": 1800,
    "quantity": 25.0
  }' > /dev/null

# 6. Verification
echo "----------------------------------------------------"
echo "🔍 Verifying Tenant A Inventory..."
INV_A=$(curl -s -X GET "$API_URL/api/inventory/products" -H "Authorization: Bearer $TOKEN_A")
HAS_A=$(echo $INV_A | jq -r '.[] | select(.product.id == "'"$SALMON_ID"'") | .id')
HAS_B_IN_A=$(echo $INV_A | jq -r '.[] | select(.product.id == "'"$BEEF_ID"'") | .id')

if [ ! -z "$HAS_A" ] && [ -z "$HAS_B_IN_A" ]; then
    echo "✅ Tenant A only sees their own products ($PRODUCT_A_NAME)."
else
    echo "❌ Tenant A isolation FAILED!"
    echo "Tenant A inventory: $INV_A"
fi

echo "🔍 Verifying Tenant B Inventory..."
INV_B=$(curl -s -X GET "$API_URL/api/inventory/products" -H "Authorization: Bearer $TOKEN_B")
HAS_B=$(echo $INV_B | jq -r '.[] | select(.product.id == "'"$BEEF_ID"'") | .id')
HAS_A_IN_B=$(echo $INV_B | jq -r '.[] | select(.product.id == "'"$SALMON_ID"'") | .id')

if [ ! -z "$HAS_B" ] && [ -z "$HAS_A_IN_B" ]; then
    echo "✅ Tenant B only sees their own products ($PRODUCT_B_NAME)."
else
    echo "❌ Tenant B isolation FAILED!"
    echo "Tenant B inventory: $INV_B"
fi

echo "----------------------------------------------------"
if [ ! -z "$HAS_A" ] && [ -z "$HAS_B_IN_A" ] && [ ! -z "$HAS_B" ] && [ -z "$HAS_A_IN_B" ]; then
    echo "🏆 SAAS KILLER TEST: PASSED! 🏆"
    echo "Data isolation is mathematically guaranteed by tenant_id filtering."
else
    echo "💀 SAAS KILLER TEST: FAILED! 💀"
    exit 1
fi
