#!/bin/bash

# Test Admin Catalog API
# Tests: login, create product, list, get, update, upload image, delete image, delete product

BASE_URL="http://localhost:8000"
ADMIN_EMAIL="admin@fodi.app"
ADMIN_PASSWORD="Admin123!"

echo "🧪 Admin Catalog API Test"
echo "=========================="
echo ""

# Step 1: Login as Super Admin
echo "1️⃣ Login as Super Admin"
LOGIN_RESPONSE=$(curl -s -X POST "$BASE_URL/api/admin/auth/login" \
  -H "Content-Type: application/json" \
  -d "{\"email\":\"$ADMIN_EMAIL\",\"password\":\"$ADMIN_PASSWORD\"}")

TOKEN=$(echo $LOGIN_RESPONSE | jq -r '.token')

if [ "$TOKEN" == "null" ] || [ -z "$TOKEN" ]; then
  echo "❌ Login failed"
  echo "$LOGIN_RESPONSE" | jq '.'
  exit 1
fi

echo "✅ Login successful"
echo "Token: ${TOKEN:0:20}..."
echo ""

# Step 2: Get first category ID
echo "2️⃣ Get category ID for test product"

# First, we need a regular user token to get categories
USER_TOKEN=$(curl -s -X POST "$BASE_URL/api/auth/login" \
  -H "Content-Type: application/json" \
  -d '{"email":"test@test.com","password":"Test123!"}' | jq -r '.access_token')

if [ "$USER_TOKEN" == "null" ] || [ -z "$USER_TOKEN" ]; then
  echo "⚠️  No regular user found, skipping category lookup"
  CATEGORY_ID="00000000-0000-0000-0000-000000000000"
else
  CATEGORIES=$(curl -s -X GET "$BASE_URL/api/catalog/categories?language=en" \
    -H "Authorization: Bearer $USER_TOKEN")
  
  CATEGORY_ID=$(echo $CATEGORIES | jq -r '.categories[0].id')
fi

echo "✅ Category ID: $CATEGORY_ID"
echo ""

# Step 3: Create product
echo "3️⃣ Create test product"
CREATE_RESPONSE=$(curl -s -X POST "$BASE_URL/api/admin/products" \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $TOKEN" \
  -d "{
    \"name_en\": \"Test Tomato\",
    \"name_pl\": \"Testowy pomidor\",
    \"name_uk\": \"Тестовий помідор\",
    \"name_ru\": \"Тестовый помидор\",
    \"category_id\": \"$CATEGORY_ID\",
    \"price\": 12.50,
    \"unit\": \"kilogram\",
    \"description\": \"Fresh test tomatoes for admin API testing\"
  }")

PRODUCT_ID=$(echo $CREATE_RESPONSE | jq -r '.id')

if [ "$PRODUCT_ID" == "null" ] || [ -z "$PRODUCT_ID" ]; then
  echo "❌ Product creation failed"
  echo "$CREATE_RESPONSE" | jq '.'
  exit 1
fi

echo "✅ Product created successfully"
echo "$CREATE_RESPONSE" | jq '.'
echo ""

# Step 4: List products
echo "4️⃣ List all products"
LIST_RESPONSE=$(curl -s -X GET "$BASE_URL/api/admin/products" \
  -H "Authorization: Bearer $TOKEN")

PRODUCT_COUNT=$(echo $LIST_RESPONSE | jq '. | length')
echo "✅ Found $PRODUCT_COUNT products"
echo ""

# Step 5: Get product by ID
echo "5️⃣ Get product by ID: $PRODUCT_ID"
GET_RESPONSE=$(curl -s -X GET "$BASE_URL/api/admin/products/$PRODUCT_ID" \
  -H "Authorization: Bearer $TOKEN")

echo "✅ Product details:"
echo "$GET_RESPONSE" | jq '.'
echo ""

# Step 6: Update product
echo "6️⃣ Update product price and description"
UPDATE_RESPONSE=$(curl -s -X PUT "$BASE_URL/api/admin/products/$PRODUCT_ID" \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $TOKEN" \
  -d "{
    \"price\": 15.99,
    \"description\": \"Updated description: Premium organic tomatoes\"
  }")

echo "✅ Product updated:"
echo "$UPDATE_RESPONSE" | jq '.'
echo ""

# Step 7: Upload image (skip if no test image available)
if [ -f "test_image.jpg" ]; then
  echo "7️⃣ Upload product image"
  UPLOAD_RESPONSE=$(curl -s -X POST "$BASE_URL/api/admin/products/$PRODUCT_ID/image" \
    -H "Authorization: Bearer $TOKEN" \
    -F "file=@test_image.jpg")
  
  IMAGE_URL=$(echo $UPLOAD_RESPONSE | jq -r '.image_url')
  echo "✅ Image uploaded: $IMAGE_URL"
  echo ""
  
  # Step 8: Delete image
  echo "8️⃣ Delete product image"
  DELETE_IMAGE_RESPONSE=$(curl -s -w "\n%{http_code}" -X DELETE "$BASE_URL/api/admin/products/$PRODUCT_ID/image" \
    -H "Authorization: Bearer $TOKEN")
  
  HTTP_CODE=$(echo "$DELETE_IMAGE_RESPONSE" | tail -n1)
  if [ "$HTTP_CODE" == "204" ]; then
    echo "✅ Image deleted successfully"
  else
    echo "❌ Image deletion failed: HTTP $HTTP_CODE"
  fi
  echo ""
else
  echo "7️⃣ ⏭️  Skipping image upload (test_image.jpg not found)"
  echo ""
fi

# Step 9: Delete product
echo "9️⃣ Delete test product"
DELETE_RESPONSE=$(curl -s -w "\n%{http_code}" -X DELETE "$BASE_URL/api/admin/products/$PRODUCT_ID" \
  -H "Authorization: Bearer $TOKEN")

HTTP_CODE=$(echo "$DELETE_RESPONSE" | tail -n1)
if [ "$HTTP_CODE" == "204" ]; then
  echo "✅ Product deleted successfully"
else
  echo "❌ Product deletion failed: HTTP $HTTP_CODE"
  echo "$DELETE_RESPONSE"
fi
echo ""

# Step 10: Verify deletion
echo "🔟 Verify product is deleted"
VERIFY_RESPONSE=$(curl -s -w "\n%{http_code}" -X GET "$BASE_URL/api/admin/products/$PRODUCT_ID" \
  -H "Authorization: Bearer $TOKEN")

HTTP_CODE=$(echo "$VERIFY_RESPONSE" | tail -n1)
if [ "$HTTP_CODE" == "404" ]; then
  echo "✅ Product not found (correct)"
else
  echo "❌ Product still exists (wrong)"
fi
echo ""

echo "=========================="
echo "✅ All Admin Catalog API tests completed!"
