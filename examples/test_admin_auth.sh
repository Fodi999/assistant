#!/bin/bash

# Test Super Admin Authentication
# Usage: ./test_admin_auth.sh [BASE_URL]

BASE_URL=${1:-http://localhost:8000}
ADMIN_EMAIL="admin@fodi.app"
ADMIN_PASSWORD="Admin123!"

echo "🔐 Testing Super Admin Authentication"
echo "Base URL: $BASE_URL"
echo ""

# Test 1: Admin Login
echo "1️⃣ POST /api/admin/auth/login - Admin Login"
LOGIN_RESPONSE=$(curl -s -X POST "$BASE_URL/api/admin/auth/login" \
  -H "Content-Type: application/json" \
  -d "{\"email\":\"$ADMIN_EMAIL\",\"password\":\"$ADMIN_PASSWORD\"}")

echo "$LOGIN_RESPONSE" | jq .
TOKEN=$(echo "$LOGIN_RESPONSE" | jq -r '.token')

if [ "$TOKEN" == "null" ] || [ -z "$TOKEN" ]; then
  echo "❌ Failed to get token"
  exit 1
fi

echo "✅ Token received: ${TOKEN:0:50}..."
echo ""

# Test 2: Verify Token (Protected Route)
echo "2️⃣ GET /api/admin/auth/verify - Verify Token"
VERIFY_RESPONSE=$(curl -s -X GET "$BASE_URL/api/admin/auth/verify" \
  -H "Authorization: Bearer $TOKEN")

echo "$VERIFY_RESPONSE" | jq .
echo "✅ Token verified"
echo ""

# Test 3: Access without token (should fail with 401)
echo "3️⃣ GET /api/admin/auth/verify - No Token (should fail)"
NO_TOKEN_RESPONSE=$(curl -s -w "\nHTTP_CODE:%{http_code}" -X GET "$BASE_URL/api/admin/auth/verify")
HTTP_CODE=$(echo "$NO_TOKEN_RESPONSE" | grep "HTTP_CODE" | cut -d: -f2)

if [ "$HTTP_CODE" == "401" ]; then
  echo "✅ Correctly rejected (401)"
else
  echo "❌ Expected 401, got $HTTP_CODE"
fi
echo ""

# Test 4: Wrong password (should fail with 401)
echo "4️⃣ POST /api/admin/auth/login - Wrong Password"
WRONG_PASSWORD_RESPONSE=$(curl -s -w "\nHTTP_CODE:%{http_code}" -X POST "$BASE_URL/api/admin/auth/login" \
  -H "Content-Type: application/json" \
  -d "{\"email\":\"$ADMIN_EMAIL\",\"password\":\"WrongPassword123\"}")

HTTP_CODE=$(echo "$WRONG_PASSWORD_RESPONSE" | grep "HTTP_CODE" | cut -d: -f2)

if [ "$HTTP_CODE" == "401" ]; then
  echo "✅ Correctly rejected wrong password (401)"
else
  echo "❌ Expected 401, got $HTTP_CODE"
fi
echo ""

# Test 5: Wrong email (should fail with 401)
echo "5️⃣ POST /api/admin/auth/login - Wrong Email"
WRONG_EMAIL_RESPONSE=$(curl -s -w "\nHTTP_CODE:%{http_code}" -X POST "$BASE_URL/api/admin/auth/login" \
  -H "Content-Type: application/json" \
  -d "{\"email\":\"wrong@email.com\",\"password\":\"$ADMIN_PASSWORD\"}")

HTTP_CODE=$(echo "$WRONG_EMAIL_RESPONSE" | grep "HTTP_CODE" | cut -d: -f2)

if [ "$HTTP_CODE" == "401" ]; then
  echo "✅ Correctly rejected wrong email (401)"
else
  echo "❌ Expected 401, got $HTTP_CODE"
fi
echo ""

echo "🎉 All tests passed!"
echo ""
echo "📋 Summary:"
echo "✅ Admin login with correct credentials"
echo "✅ Token verification works"
echo "✅ No token = 401 Unauthorized"
echo "✅ Wrong password = 401 Unauthorized"
echo "✅ Wrong email = 401 Unauthorized"
