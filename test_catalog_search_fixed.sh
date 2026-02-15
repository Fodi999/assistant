#!/bin/bash

echo "🧪 Testing Catalog Search (Fixed)"
echo "=================================="
echo ""

BASE_URL="https://ministerial-yetta-fodi999-c58d8823.koyeb.app"

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m'

# Helper function to test API with HTTP code check
test_api() {
    local url="$1"
    local token="$2"
    local description="$3"
    
    echo "🔍 $description"
    echo "URL: $url"
    
    # Get response + HTTP code
    resp=$(curl -sS -w "\n%{http_code}" "$url" -H "Authorization: Bearer $token")
    body=$(echo "$resp" | sed '$d')
    http_code=$(echo "$resp" | tail -n1)
    
    echo "HTTP Code: $http_code"
    
    # Check if valid JSON
    if ! echo "$body" | jq . >/dev/null 2>&1; then
        echo -e "${RED}❌ Response is not valid JSON${NC}"
        echo "Response:"
        echo "$body"
        echo ""
        return 1
    fi
    
    # Check HTTP code
    if [ "$http_code" != "200" ]; then
        echo -e "${RED}❌ HTTP error: $http_code${NC}"
        echo "$body" | jq .
        echo ""
        return 1
    fi
    
    # Parse ingredients array
    count=$(echo "$body" | jq -r '.ingredients | length // 0')
    
    echo "Response:"
    echo "$body" | jq .
    echo ""
    echo "Found: $count results"
    echo ""
    
    if [ "$count" -gt 0 ]; then
        echo -e "${GREEN}✅ SUCCESS: Found $count results${NC}"
        return 0
    else
        echo -e "${YELLOW}⚠️  WARNING: 0 results${NC}"
        return 1
    fi
}

echo "📝 Step 1: Register test user"
echo "------------------------------"
register_resp=$(curl -sS -w "\n%{http_code}" -X POST "$BASE_URL/api/auth/register" \
  -H "Content-Type: application/json" \
  -d "{
    \"email\": \"test_$(date +%s)@test.com\",
    \"password\": \"Test123!\",
    \"restaurant_name\": \"Test Restaurant\",
    \"owner_name\": \"Test Owner\"
  }")

register_body=$(echo "$register_resp" | sed '$d')
register_code=$(echo "$register_resp" | tail -n1)

if [ "$register_code" != "201" ] && [ "$register_code" != "200" ]; then
    echo -e "${RED}❌ Registration failed: HTTP $register_code${NC}"
    echo "$register_body"
    exit 1
fi

echo "$register_body" | jq .

TOKEN=$(echo "$register_body" | jq -r '.access_token // .token // empty')

if [ -z "$TOKEN" ]; then
    echo -e "${RED}❌ No token received${NC}"
    exit 1
fi

echo -e "${GREEN}✅ User registered successfully${NC}"
echo "Token: ${TOKEN:0:50}..."
echo ""

echo "=================================================="
echo "🔍 TESTING CATALOG SEARCH"
echo "=================================================="
echo ""

# Test 1: English search
test_api "$BASE_URL/api/catalog/ingredients?q=cocoa" "$TOKEN" "Test 1: English search (cocoa)"
EN_RESULT=$?

# Test 2: Russian search (full word)
test_api "$BASE_URL/api/catalog/ingredients?q=какао" "$TOKEN" "Test 2: Russian search (какао)"
RU_FULL=$?

# Test 3: Russian search (partial)
test_api "$BASE_URL/api/catalog/ingredients?q=како" "$TOKEN" "Test 3: Russian search partial (како)"
RU_PARTIAL=$?

# Test 4: English milk
test_api "$BASE_URL/api/catalog/ingredients?q=milk" "$TOKEN" "Test 4: English search (milk)"
MILK=$?

# Test 5: Russian milk
test_api "$BASE_URL/api/catalog/ingredients?q=молоко" "$TOKEN" "Test 5: Russian search (молоко)"
MOLOKO=$?

echo "=================================================="
echo "📊 SUMMARY"
echo "=================================================="
echo ""

if [ $EN_RESULT -eq 0 ]; then
    echo -e "✅ English 'cocoa':  ${GREEN}PASS${NC}"
else
    echo -e "❌ English 'cocoa':  ${RED}FAIL${NC}"
fi

if [ $RU_FULL -eq 0 ]; then
    echo -e "✅ Russian 'какао':  ${GREEN}PASS${NC}"
else
    echo -e "❌ Russian 'какао':  ${RED}FAIL${NC}"
fi

if [ $RU_PARTIAL -eq 0 ]; then
    echo -e "✅ Russian 'како':   ${GREEN}PASS${NC}"
else
    echo -e "❌ Russian 'како':   ${YELLOW}FAIL (partial match might not work)${NC}"
fi

if [ $MILK -eq 0 ]; then
    echo -e "✅ English 'milk':   ${GREEN}PASS${NC}"
else
    echo -e "❌ English 'milk':   ${YELLOW}FAIL (no milk in catalog?)${NC}"
fi

if [ $MOLOKO -eq 0 ]; then
    echo -e "✅ Russian 'молоко': ${GREEN}PASS${NC}"
else
    echo -e "❌ Russian 'молоко': ${RED}FAIL${NC}"
fi

echo ""
echo "=================================================="
echo ""

if [ $EN_RESULT -eq 0 ] && [ $RU_FULL -eq 0 ]; then
    echo -e "${GREEN}🎉 CRITICAL TESTS PASSED!${NC}"
    echo -e "${GREEN}✅ Multilingual search is working${NC}"
else
    echo -e "${RED}❌ CRITICAL TESTS FAILED${NC}"
    echo ""
    echo "Possible reasons:"
    echo "1. Koyeb hasn't deployed new code yet (check commit b88f1c7)"
    echo "2. Database doesn't have Russian translations"
    echo "3. Backend search still using old SQL (check logs)"
    echo ""
    echo "Next steps:"
    echo "- Check Koyeb deployment status"
    echo "- Verify commit b88f1c7 is deployed"
    echo "- Check database: SELECT name_en, name_ru FROM catalog_ingredients LIMIT 5;"
fi

echo ""
