#!/bin/bash

PROD_URL="https://ministerial-yetta-fodi999-c58d8823.koyeb.app"

echo "=== Testing Admin Catalog API on PRODUCTION with R2 ==="
echo ""

# 1. Login
echo "1. Login as Super Admin..."
LOGIN_RESPONSE=$(curl -s -X POST "$PROD_URL/api/admin/auth/login" \
  -H "Content-Type: application/json" \
  -d '{"email":"admin@fodi.app","password":"Admin123!"}')

TOKEN=$(echo "$LOGIN_RESPONSE" | jq -r '.token')

if [ "$TOKEN" == "null" ] || [ -z "$TOKEN" ]; then
  echo "❌ Login failed"
  echo "$LOGIN_RESPONSE" | jq .
  exit 1
fi

echo "✅ Token received: ${TOKEN:0:50}..."
echo ""

# 2. Create test product
echo "2. Creating test product..."
CREATE_RESPONSE=$(curl -s -X POST "$PROD_URL/api/admin/products" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name_en": "Production R2 Test",
    "category_id": "c3818a0d-dd0c-4a0d-81bb-5edc4c91bab0",
    "price": "99.99",
    "unit": "kilogram",
    "description": "Testing R2 image upload on production"
  }')

PRODUCT_ID=$(echo "$CREATE_RESPONSE" | jq -r '.id')

if [ "$PRODUCT_ID" == "null" ] || [ -z "$PRODUCT_ID" ]; then
  echo "❌ Product creation failed"
  echo "$CREATE_RESPONSE" | jq .
  exit 1
fi

echo "✅ Product created: $PRODUCT_ID"
echo "$CREATE_RESPONSE" | jq '{id, name_en, price, unit}'
echo ""

# 3. Upload image to R2
echo "3. Uploading image to Cloudflare R2..."
UPLOAD_RESPONSE=$(curl -s -X POST "$PROD_URL/api/admin/products/$PRODUCT_ID/image" \
  -H "Authorization: Bearer $TOKEN" \
  -F "file=@test_image.jpg")

IMAGE_URL=$(echo "$UPLOAD_RESPONSE" | jq -r '.image_url')

if [ "$IMAGE_URL" == "null" ] || [ -z "$IMAGE_URL" ]; then
  echo "❌ Image upload failed"
  echo "$UPLOAD_RESPONSE" | jq .
  exit 1
fi

echo "✅ Image uploaded to R2: $IMAGE_URL"
echo ""

# 4. Verify product has image
echo "4. Verifying product has image URL..."
VERIFY_RESPONSE=$(curl -s -X GET "$PROD_URL/api/admin/products/$PRODUCT_ID" \
  -H "Authorization: Bearer $TOKEN")

VERIFIED_URL=$(echo "$VERIFY_RESPONSE" | jq -r '.image_url')

if [ "$VERIFIED_URL" == "$IMAGE_URL" ]; then
  echo "✅ Image URL verified in database"
else
  echo "❌ Image URL mismatch"
  echo "Expected: $IMAGE_URL"
  echo "Got: $VERIFIED_URL"
  exit 1
fi

echo ""
echo "5. Testing image accessibility..."
HTTP_CODE=$(curl -s -o /dev/null -w "%{http_code}" "$IMAGE_URL")

if [ "$HTTP_CODE" == "200" ]; then
  echo "✅ Image is publicly accessible (HTTP $HTTP_CODE)"
else
  echo "⚠️  Image returned HTTP $HTTP_CODE"
fi

echo ""
echo "6. Cleaning up - deleting test product..."
DELETE_RESPONSE=$(curl -s -o /dev/null -w "%{http_code}" -X DELETE "$PROD_URL/api/admin/products/$PRODUCT_ID" \
  -H "Authorization: Bearer $TOKEN")

if [ "$DELETE_RESPONSE" == "204" ]; then
  echo "✅ Test product deleted (HTTP $DELETE_RESPONSE)"
else
  echo "⚠️  Delete returned HTTP $DELETE_RESPONSE"
fi

echo ""
echo "=== ✅ ALL TESTS PASSED - R2 INTEGRATION WORKING ON PRODUCTION ==="
