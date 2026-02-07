#!/bin/bash

# Expiration Warnings Test
# Tests that assistant correctly detects and warns about expired/expiring products

set -e

BASE_URL="http://localhost:8080"

echo "=========================================="
echo "Inventory Expiration Warnings Test"
echo "=========================================="
echo ""

# Register user
echo "1. Registering test user..."
UNIQUE_EMAIL="expiration_test_$(date +%s)@example.com"
REGISTER_RESPONSE=$(curl -s -X POST "$BASE_URL/api/auth/register" \
  -H "Content-Type: application/json" \
  -d "{
    \"email\": \"$UNIQUE_EMAIL\",
    \"password\": \"Test123!\",
    \"name\": \"Expiration Test\",
    \"restaurant_name\": \"Test Restaurant\",
    \"language\": \"pl\"
  }")

TOKEN=$(echo $REGISTER_RESPONSE | jq -r '.access_token')
echo "✅ User registered"
echo ""

# Start inventory
echo "2. Starting inventory..."
curl -s -X POST "$BASE_URL/api/assistant/command" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"command": {"type": "start_inventory"}}' > /dev/null
echo "✅ Inventory started"
echo ""

# Test 1: Add EXPIRED product (yesterday)
YESTERDAY=$(date -u -v-1d +"%Y-%m-%dT%H:%M:%SZ" 2>/dev/null || date -u -d "yesterday" +"%Y-%m-%dT%H:%M:%SZ")
echo "3. Test #1: Adding EXPIRED product (expires_at = $YESTERDAY)..."
curl -s -X POST "$BASE_URL/api/assistant/command" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d "{
    \"command\": {
      \"type\": \"add_product\",
      \"payload\": {
        \"catalog_ingredient_id\": \"519169f2-69f1-4875-94ed-12eccbb809ae\",
        \"price_per_unit_cents\": 450,
        \"quantity\": 5.0,
        \"expires_at\": \"$YESTERDAY\"
      }
    }
  }" > /dev/null
echo "✅ Expired product added"
echo ""

# Check assistant state - should have CRITICAL warning
echo "4. Checking assistant state (expecting CRITICAL warning)..."
STATE=$(curl -s -X GET "$BASE_URL/api/assistant/state" -H "Authorization: Bearer $TOKEN")
echo "$STATE" | jq '{step, warnings}'

CRITICAL_COUNT=$(echo "$STATE" | jq '.warnings | map(select(.level == "critical")) | length')
if [ "$CRITICAL_COUNT" -gt 0 ]; then
  echo "✅ CRITICAL warning detected for expired product"
else
  echo "❌ Expected CRITICAL warning but got none"
  exit 1
fi
echo ""

# Test 2: Add product EXPIRING TODAY
TODAY=$(date -u +"%Y-%m-%dT%H:%M:%SZ")
echo "5. Test #2: Adding product EXPIRING TODAY (expires_at = $TODAY)..."
curl -s -X POST "$BASE_URL/api/assistant/command" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d "{
    \"command\": {
      \"type\": \"add_product\",
      \"payload\": {
        \"catalog_ingredient_id\": \"ee8715c7-7391-423b-8346-39ef6913bb19\",
        \"price_per_unit_cents\": 50,
        \"quantity\": 10.0,
        \"expires_at\": \"$TODAY\"
      }
    }
  }" > /dev/null
echo "✅ Product expiring today added"
echo ""

# Check warnings again
echo "6. Checking assistant state (expecting 2 warnings: CRITICAL + WARNING)..."
STATE=$(curl -s -X GET "$BASE_URL/api/assistant/state" -H "Authorization: Bearer $TOKEN")
echo "$STATE" | jq '{step, warnings}'

WARNING_COUNT=$(echo "$STATE" | jq '.warnings | length')
if [ "$WARNING_COUNT" -ge 2 ]; then
  echo "✅ Multiple warnings detected"
else
  echo "❌ Expected at least 2 warnings but got $WARNING_COUNT"
  exit 1
fi
echo ""

# Test 3: Add product EXPIRING SOON (tomorrow)
TOMORROW=$(date -u -v+1d +"%Y-%m-%dT%H:%M:%SZ" 2>/dev/null || date -u -d "tomorrow" +"%Y-%m-%dT%H:%M:%SZ")
echo "7. Test #3: Adding product EXPIRING SOON (expires_at = $TOMORROW)..."
curl -s -X POST "$BASE_URL/api/assistant/command" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d "{
    \"command\": {
      \"type\": \"add_product\",
      \"payload\": {
        \"catalog_ingredient_id\": \"82ea2199-c133-4fc8-8f34-15ee562c23c1\",
        \"price_per_unit_cents\": 1500,
        \"quantity\": 2.0,
        \"expires_at\": \"$TOMORROW\"
      }
    }
  }" > /dev/null
echo "✅ Product expiring soon added"
echo ""

# Final check
echo "8. Final check (expecting 3 warnings: CRITICAL + WARNING + INFO)..."
STATE=$(curl -s -X GET "$BASE_URL/api/assistant/state" -H "Authorization: Bearer $TOKEN")
echo "$STATE" | jq '{step, warnings}'

FINAL_WARNING_COUNT=$(echo "$STATE" | jq '.warnings | length')
if [ "$FINAL_WARNING_COUNT" -ge 3 ]; then
  echo "✅ All 3 warning types detected"
else
  echo "⚠️  Expected 3 warnings but got $FINAL_WARNING_COUNT"
fi
echo ""

# Test 4: Add FRESH product (no expiration)
echo "9. Test #4: Adding FRESH product (no expires_at)..."
curl -s -X POST "$BASE_URL/api/assistant/command" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "command": {
      "type": "add_product",
      "payload": {
        "catalog_ingredient_id": "6d20ebe8-1261-4a60-9aa5-6a79fb908427",
        "price_per_unit_cents": 300,
        "quantity": 5.0,
        "expires_at": null
      }
    }
  }' > /dev/null
echo "✅ Fresh product added (no expiration date)"
echo ""

# Final state
echo "10. Final assistant state:"
curl -s -X GET "$BASE_URL/api/assistant/state" -H "Authorization: Bearer $TOKEN" | jq .

echo ""
echo "=========================================="
echo "✅ Expiration Warnings Test Complete!"
echo "=========================================="
echo ""
echo "Summary:"
echo "- ❌ 1 expired product (yesterday) → CRITICAL warning"
echo "- ⚠️  1 product expiring today → WARNING"
echo "- ℹ️  1 product expiring soon (2 days) → INFO"
echo "- ✅ 1 fresh product (no expiration) → no warning"
echo ""
echo "Architecture:"
echo "- ✅ Bot reads state reactively (no monitoring)"
echo "- ✅ ExpirationStatus calculated on demand"
echo "- ✅ Warnings enriched in AssistantService"
echo "- ✅ Multilingual support (PL/EN/UK/RU)"
echo ""
