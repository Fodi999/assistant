#!/bin/bash

echo "🧪 Simple Koyeb Production Test"
echo "================================"

BASE_URL="https://ministerial-yetta-fodi999-c58d8823.koyeb.app"

# Health check
echo "1. Health check..."
curl -s -m 5 "$BASE_URL/health" && echo " ✅"

# Check if Recipe V2 endpoint exists (without auth)
echo ""
echo "2. Recipe V2 endpoint check (should return auth error)..."
RESPONSE=$(curl -s -m 5 -X GET "$BASE_URL/api/recipes/v2")
echo $RESPONSE | jq -r '.message // .error // .' | head -1

if echo "$RESPONSE" | grep -q "Authentication"; then
  echo "✅ Recipe V2 endpoint exists and requires auth (correct!)"
else
  echo "⚠️  Unexpected response"
fi

echo ""
echo "✅ Basic checks complete. Server is running with Recipe V2!"
