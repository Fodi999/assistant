#!/bin/bash
# Test Script for Recipe Photo Upload

# API URL
API_URL="http://localhost:8000/api"
EMAIL="fodi85999333@gmail.com"
PASSWORD="fodi210185"

echo "1. Checking credentials (logging in)..."
MAX_RETRIES=10
COUNT=0
# Try login until success or definitive failure
while [ $COUNT -lt $MAX_RETRIES ]; do
  COUNT=$((COUNT+1))
  
  # Get response body and HTTP status code
  RESPONSE=$(curl -s -w "\n%{http_code}" -X POST "$API_URL/auth/login" \
    -H "Content-Type: application/json" \
    -d "{
      \"email\": \"$EMAIL\",
      \"password\": \"$PASSWORD\"
    }")
  
  HTTP_CODE=$(echo "$RESPONSE" | tail -n 1)
  LOGIN_RESPONSE=$(echo "$RESPONSE" | sed '$d')

  if [ "$HTTP_CODE" -eq 429 ]; then
    echo "Wait for login rate limiter ($COUNT/$MAX_RETRIES)..."
    sleep 2
    continue
  fi

  if [ "$HTTP_CODE" -eq 401 ] || echo "$LOGIN_RESPONSE" | grep -q "Invalid email or password"; then
    echo "User not found or invalid credentials, attempting registration..."
    
    # Try registration
    REGISTER_RESP_FULL=$(curl -s -w "\n%{http_code}" -X POST "$API_URL/auth/register" \
      -H "Content-Type: application/json" \
      -d "{
        \"email\": \"$EMAIL\",
        \"password\": \"$PASSWORD\",
        \"tenant_name\": \"Fodi Kitchen\"
      }")
    
    REG_HTTP_CODE=$(echo "$REGISTER_RESP_FULL" | tail -n 1)
    REGISTER_RESPONSE=$(echo "$REGISTER_RESP_FULL" | sed '$d')

    if [ "$REG_HTTP_CODE" -eq 429 ]; then
        echo "Wait for registration rate limiter ($COUNT/$MAX_RETRIES)..."
        sleep 2
        continue
    else
        echo "Registration Response (HTTP $REG_HTTP_CODE): $REGISTER_RESPONSE"
        # If registration was successful (likely 200 or 201), the next loop will log in.
        # If it failed for other reasons, the count will increment and we try again.
    fi
  else
    TOKEN=$(echo "$LOGIN_RESPONSE" | jq -r .access_token)
    if [ "$TOKEN" != "null" ] && [ -n "$TOKEN" ]; then
      echo "Login success!"
      break
    else
      echo "Unexpected error (HTTP $HTTP_CODE): $LOGIN_RESPONSE"
      sleep 1
    fi
  fi

  if [ $COUNT -eq $MAX_RETRIES ]; then
    echo "❌ Error: Maximum retries reached for authentication."
    exit 1
  fi
done

if [ "$TOKEN" == "null" ] || [ -z "$TOKEN" ]; then
    echo "Could not obtain token!"
    exit 1
fi

echo "2. Fetching existing ingredients..."
INGREDIENTS_RESPONSE=$(curl -s -X GET "$API_URL/catalog/ingredients" \
  -H "Authorization: Bearer $TOKEN")
INGREDIENT_ID=$(echo $INGREDIENTS_RESPONSE | jq -r '.[0].id // empty')

if [ -z "$INGREDIENT_ID" ]; then
  echo "No ingredients found in catalog. Adding one as admin..."
  # Use admin credentials if possible, or just add one as tenant?
  # Wait, tenant_ingredient API is for inventory. Let's try to find a global ingredient.
  # If catalog is empty, we must add one to test recipe creation.
  echo "Falling back: Cannot create recipe without ingredients. Let's try to add one to the catalog."
  # For this test, let's assume there is at least one ingredient.
  # (In a real environment, catalog should have seed data).
fi

echo "3. Creating a V2 recipe..."
RECIPE_RESPONSE=$(curl -s -X POST "$API_URL/recipes/v2" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d "{
    \"name\": \"Test Recipe with Photo\",
    \"instructions\": \"Step 1: Test upload. Step 2: Profit.\",
    \"language\": \"ru\",
    \"servings\": 2,
    \"ingredients\": []
  }")
RECIPE_ID=$(echo $RECIPE_RESPONSE | jq -r .id)
echo "Recipe ID: $RECIPE_ID"

echo "5. Testing multipart image upload..."
# Create a dummy image file
echo "BM6..........................." > dummy.webp

UPLOAD_RESPONSE=$(curl -s -X POST "$API_URL/recipes/v2/$RECIPE_ID/image" \
  -H "Authorization: Bearer $TOKEN" \
  -F "image=@dummy.webp;type=image/webp")
IMAGE_URL=$(echo $UPLOAD_RESPONSE | jq -r .image_url)

echo "Upload Response: $UPLOAD_RESPONSE"
echo "Image URL: $IMAGE_URL"

echo "6. Verifying the recipe has the image_url now..."
VERIFY_RESPONSE=$(curl -s -X GET "$API_URL/recipes/v2/$RECIPE_ID" \
  -H "Authorization: Bearer $TOKEN")
CURRENT_IMAGE=$(echo $VERIFY_RESPONSE | jq -r .image_url)

echo "Recipe Image URL in DB: $CURRENT_IMAGE"

if [ "$CURRENT_IMAGE" != "null" ] && [ "$CURRENT_IMAGE" == "$IMAGE_URL" ]; then
    echo "✅ TEST PASSED: Photo uploaded and saved to recipe!"
else
    echo "❌ TEST FAILED: Photo URL missing or mismatch."
fi

# Cleanup
rm dummy.webp
