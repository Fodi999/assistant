#!/bin/bash

# Restaurant Backend API Examples
# Make sure the server is running on http://localhost:8080

BASE_URL="http://localhost:8080"

echo "=== Restaurant Backend API Examples ==="
echo ""

# 1. Register a new user (creates tenant + owner)
echo "1. Register new user and restaurant"
echo "POST $BASE_URL/api/auth/register"
REGISTER_RESPONSE=$(curl -s -X POST "$BASE_URL/api/auth/register" \
  -H "Content-Type: application/json" \
  -d '{
    "email": "owner@restaurant.com",
    "password": "SecurePass123!",
    "restaurant_name": "The Best Restaurant",
    "owner_name": "John Doe"
  }')

echo "$REGISTER_RESPONSE" | jq .
echo ""

# Extract tokens
ACCESS_TOKEN=$(echo "$REGISTER_RESPONSE" | jq -r '.access_token')
REFRESH_TOKEN=$(echo "$REGISTER_RESPONSE" | jq -r '.refresh_token')

# 2. Get current user info (protected endpoint)
echo "2. Get current user info (using access token)"
echo "GET $BASE_URL/api/me"
curl -s -X GET "$BASE_URL/api/me" \
  -H "Authorization: Bearer $ACCESS_TOKEN" | jq .
echo ""

# 3. Login with existing user
echo "3. Login with existing user"
echo "POST $BASE_URL/api/auth/login"
LOGIN_RESPONSE=$(curl -s -X POST "$BASE_URL/api/auth/login" \
  -H "Content-Type: application/json" \
  -d '{
    "email": "owner@restaurant.com",
    "password": "SecurePass123!"
  }')

echo "$LOGIN_RESPONSE" | jq .
echo ""

# Update tokens from login response
ACCESS_TOKEN=$(echo "$LOGIN_RESPONSE" | jq -r '.access_token')
REFRESH_TOKEN=$(echo "$LOGIN_RESPONSE" | jq -r '.refresh_token')

# 4. Refresh access token
echo "4. Refresh access token"
echo "POST $BASE_URL/api/auth/refresh"
REFRESH_RESPONSE=$(curl -s -X POST "$BASE_URL/api/auth/refresh" \
  -H "Content-Type: application/json" \
  -d "{
    \"refresh_token\": \"$REFRESH_TOKEN\"
  }")

echo "$REFRESH_RESPONSE" | jq .
echo ""

# Update access token
ACCESS_TOKEN=$(echo "$REFRESH_RESPONSE" | jq -r '.access_token')

# 5. Use new access token
echo "5. Get user info with refreshed token"
echo "GET $BASE_URL/api/me"
curl -s -X GET "$BASE_URL/api/me" \
  -H "Authorization: Bearer $ACCESS_TOKEN" | jq .
echo ""

# 6. Test validation errors
echo "6. Test validation - invalid email"
echo "POST $BASE_URL/api/auth/register"
curl -s -X POST "$BASE_URL/api/auth/register" \
  -H "Content-Type: application/json" \
  -d '{
    "email": "invalid-email",
    "password": "SecurePass123!",
    "restaurant_name": "Test Restaurant",
    "owner_name": "Test User"
  }' | jq .
echo ""

# 7. Test validation errors - short password
echo "7. Test validation - short password"
echo "POST $BASE_URL/api/auth/register"
curl -s -X POST "$BASE_URL/api/auth/register" \
  -H "Content-Type: application/json" \
  -d '{
    "email": "test@example.com",
    "password": "short",
    "restaurant_name": "Test Restaurant",
    "owner_name": "Test User"
  }' | jq .
echo ""

# 8. Test unauthorized access
echo "8. Test unauthorized access (no token)"
echo "GET $BASE_URL/api/me"
curl -s -X GET "$BASE_URL/api/me" | jq .
echo ""

# 9. Test invalid token
echo "9. Test invalid token"
echo "GET $BASE_URL/api/me"
curl -s -X GET "$BASE_URL/api/me" \
  -H "Authorization: Bearer invalid-token" | jq .
echo ""

echo "=== All examples completed ==="
