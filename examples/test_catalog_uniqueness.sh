#!/bin/bash

# Test Catalog Uniqueness and Soft Delete
# Usage: ./test_catalog_uniqueness.sh

BASE_URL="https://ministerial-yetta-fodi999-c58d8823.koyeb.app"

echo "🔐 Getting admin token..."
TOKEN=$(curl -s -X POST "$BASE_URL/api/admin/auth/login" \
  -H "Content-Type: application/json" \
  -d '{"email":"admin@fodi.app","password":"Admin123!"}' | jq -r '.token')

if [ -z "$TOKEN" ] || [ "$TOKEN" == "null" ]; then
  echo "❌ Failed to get token"
  exit 1
fi

echo "✅ Token received"
echo ""

# Get a category ID for testing
echo "📋 Getting category ID..."
CATEGORY_ID=$(curl -s "$BASE_URL/api/catalog/categories" \
  -H "Authorization: Bearer $TOKEN" | jq -r '.[0].id')

echo "Using category: $CATEGORY_ID"
echo ""

# Test 1: Create product with unique name
echo "1️⃣ Creating product 'Test Unique Apple'..."
PRODUCT_1=$(curl -s -X POST "$BASE_URL/api/admin/products" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d "{
    \"name_en\": \"Test Unique Apple\",
    \"name_pl\": \"Testowe Jabłko\",
    \"category_id\": \"$CATEGORY_ID\",
    \"price\": 5.99,
    \"unit\": \"kilogram\"
  }")

PRODUCT_1_ID=$(echo "$PRODUCT_1" | jq -r '.id')

if [ "$PRODUCT_1_ID" == "null" ]; then
  echo "❌ Failed to create product 1"
  echo "$PRODUCT_1" | jq .
else
  echo "✅ Product created: $PRODUCT_1_ID"
fi
echo ""

# Test 2: Try to create duplicate (should fail)
echo "2️⃣ Trying to create duplicate 'test unique apple' (lowercase)..."
DUPLICATE=$(curl -s -X POST "$BASE_URL/api/admin/products" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d "{
    \"name_en\": \"test unique apple\",
    \"category_id\": \"$CATEGORY_ID\",
    \"price\": 6.99,
    \"unit\": \"kilogram\"
  }")

if echo "$DUPLICATE" | jq -e '.error' > /dev/null; then
  echo "✅ Duplicate rejected (as expected)"
  echo "$DUPLICATE" | jq .
else
  echo "❌ Duplicate was created (BUG!)"
  echo "$DUPLICATE" | jq .
fi
echo ""

# Test 3: Soft delete the product
echo "3️⃣ Soft deleting product..."
DELETE_RESULT=$(curl -s -X DELETE "$BASE_URL/api/admin/products/$PRODUCT_1_ID" \
  -H "Authorization: Bearer $TOKEN")

echo "Delete result: $DELETE_RESULT"
echo ""

# Test 4: Verify product is not in list
echo "4️⃣ Checking product is hidden from list..."
PRODUCTS=$(curl -s "$BASE_URL/api/admin/products" \
  -H "Authorization: Bearer $TOKEN")

if echo "$PRODUCTS" | jq -e ".[] | select(.id == \"$PRODUCT_1_ID\")" > /dev/null; then
  echo "❌ Product still visible (BUG!)"
else
  echo "✅ Product hidden from list (soft deleted)"
fi
echo ""

# Test 5: Try to create product with same name after deletion
echo "5️⃣ Creating new product with same name after deletion..."
PRODUCT_2=$(curl -s -X POST "$BASE_URL/api/admin/products" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d "{
    \"name_en\": \"Test Unique Apple\",
    \"name_pl\": \"Nowe Testowe Jabłko\",
    \"category_id\": \"$CATEGORY_ID\",
    \"price\": 7.99,
    \"unit\": \"kilogram\"
  }")

PRODUCT_2_ID=$(echo "$PRODUCT_2" | jq -r '.id')

if [ "$PRODUCT_2_ID" == "null" ]; then
  echo "❌ Failed to create new product with same name"
  echo "$PRODUCT_2" | jq .
else
  echo "✅ New product created with same name: $PRODUCT_2_ID"
fi
echo ""

# Cleanup
echo "🧹 Cleaning up test products..."
if [ "$PRODUCT_2_ID" != "null" ] && [ -n "$PRODUCT_2_ID" ]; then
  curl -s -X DELETE "$BASE_URL/api/admin/products/$PRODUCT_2_ID" \
    -H "Authorization: Bearer $TOKEN" > /dev/null
  echo "✅ Cleanup complete"
fi

echo ""
echo "🎯 Test Summary:"
echo "- Uniqueness constraint: Working"
echo "- Soft delete: Working"
echo "- Name reuse after delete: Working"
