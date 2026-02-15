#!/bin/bash

################################################################################
# Universal Input Architecture - Complete Test Suite
# Tests the full flow: Admin inputs "Молоко" → normalize → classify → translate → save
# 
# Usage:
#   export KOYEB_ADMIN_TOKEN='your-jwt-token'
#   bash test_universal_input.sh
################################################################################

set -e

# Configuration
API_URL="${API_URL:-https://api.fodi.app}"
ADMIN_TOKEN="${KOYEB_ADMIN_TOKEN:-}"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Test counters
TESTS_PASSED=0
TESTS_FAILED=0
TESTS_TOTAL=0

################################################################################
# Utility Functions
################################################################################

print_header() {
    echo ""
    echo -e "${BLUE}════════════════════════════════════════════════════════════${NC}"
    echo -e "${BLUE}$1${NC}"
    echo -e "${BLUE}════════════════════════════════════════════════════════════${NC}"
    echo ""
}

print_test() {
    TESTS_TOTAL=$((TESTS_TOTAL + 1))
    echo -e "${CYAN}[TEST $TESTS_TOTAL] $1${NC}"
}

print_success() {
    TESTS_PASSED=$((TESTS_PASSED + 1))
    echo -e "${GREEN}✅ $1${NC}"
}

print_failure() {
    TESTS_FAILED=$((TESTS_FAILED + 1))
    echo -e "${RED}❌ $1${NC}"
}

print_info() {
    echo -e "${YELLOW}ℹ️  $1${NC}"
}

print_step() {
    echo -e "${YELLOW}→ $1${NC}"
}

# Make API request
api_request() {
    local method="$1"
    local endpoint="$2"
    local data="$3"
    
    if [ -z "$data" ]; then
        curl -s -X "$method" "$API_URL$endpoint" \
            -H "Content-Type: application/json" \
            -H "Authorization: Bearer $ADMIN_TOKEN"
    else
        curl -s -X "$method" "$API_URL$endpoint" \
            -H "Content-Type: application/json" \
            -H "Authorization: Bearer $ADMIN_TOKEN" \
            -d "$data"
    fi
}

################################################################################
# Test Suite
################################################################################

test_auth() {
    print_test "Authentication Token Validation"
    
    if [ -z "$ADMIN_TOKEN" ]; then
        print_failure "No ADMIN_TOKEN provided"
        echo ""
        echo "Get token with:"
        echo "  curl -X POST $API_URL/api/auth/login \\"
        echo "    -H 'Content-Type: application/json' \\"
        echo "    -d '{\"email\":\"admin@fodi.app\",\"password\":\"YOUR_PASSWORD\"}' \\"
        echo "    | jq '.data.access_token'"
        echo ""
        echo "Then export:"
        echo "  export KOYEB_ADMIN_TOKEN='token-here'"
        exit 1
    fi
    
    # Quick health check
    RESPONSE=$(api_request "GET" "/api/health")
    if echo "$RESPONSE" | jq . > /dev/null 2>&1; then
        print_success "API is responding"
    else
        print_failure "API not responding"
        exit 1
    fi
}

test_russian_input() {
    print_test "Russian Input: 'Молоко' (Milk)"
    print_step "normalize_to_english('Молоко') → 'Milk'"
    print_step "classify_product('Milk') → dairy_and_eggs + liter"
    print_step "translate('Milk') → PL/RU/UK"
    print_step "Save to database"
    
    RESPONSE=$(api_request "POST" "/api/admin/catalog/products" \
        '{"name_input":"Молоко","auto_translate":true}')
    
    SUCCESS=$(echo "$RESPONSE" | jq -r '.success // false')
    
    if [ "$SUCCESS" = "true" ]; then
        NAME_EN=$(echo "$RESPONSE" | jq -r '.data.name_en // "N/A"')
        CATEGORY=$(echo "$RESPONSE" | jq -r '.data.category.name // "N/A"')
        UNIT=$(echo "$RESPONSE" | jq -r '.data.unit // "N/A"')
        
        if [ "$NAME_EN" = "Milk" ]; then
            print_success "Normalized to 'Milk'"
        else
            print_failure "Expected 'Milk', got '$NAME_EN'"
        fi
        
        if [ "$CATEGORY" = "Dairy & Eggs" ] || [ "$CATEGORY" = "dairy_and_eggs" ]; then
            print_success "Classified as '$CATEGORY'"
        else
            print_failure "Expected 'Dairy & Eggs', got '$CATEGORY'"
        fi
        
        if [ "$UNIT" = "liter" ]; then
            print_success "Unit set to 'liter'"
        else
            print_info "Unit is '$UNIT' (expected 'liter')"
        fi
        
        # Check translations
        NAME_PL=$(echo "$RESPONSE" | jq -r '.data.name_pl // "N/A"')
        NAME_RU=$(echo "$RESPONSE" | jq -r '.data.name_ru // "N/A"')
        NAME_UK=$(echo "$RESPONSE" | jq -r '.data.name_uk // "N/A"')
        
        if [ "$NAME_PL" != "N/A" ] && [ "$NAME_PL" != "" ]; then
            print_success "Polish translation: '$NAME_PL'"
        else
            print_failure "Missing Polish translation"
        fi
        
    else
        print_failure "Product creation failed"
        echo "$RESPONSE" | jq .
    fi
}

test_polish_input() {
    print_test "Polish Input: 'Mleko' (Milk)"
    
    RESPONSE=$(api_request "POST" "/api/admin/catalog/products" \
        '{"name_input":"Mleko","auto_translate":true}')
    
    SUCCESS=$(echo "$RESPONSE" | jq -r '.success // false')
    
    if [ "$SUCCESS" = "true" ]; then
        NAME_EN=$(echo "$RESPONSE" | jq -r '.data.name_en // "N/A"')
        
        if [ "$NAME_EN" = "Milk" ]; then
            print_success "Polish 'Mleko' normalized to 'Milk'"
        else
            print_failure "Expected 'Milk', got '$NAME_EN'"
        fi
    else
        ERROR=$(echo "$RESPONSE" | jq -r '.error // "Unknown error"')
        if [[ "$ERROR" == *"already exists"* ]]; then
            print_success "Duplicate detection working (expected for Milk)"
        else
            print_failure "Unexpected error: $ERROR"
        fi
    fi
}

test_english_multiword() {
    print_test "English Input: 'Green Apple' (Multi-word)"
    print_step "Should NOT call AI for normalization (ASCII-only optimization)"
    
    RESPONSE=$(api_request "POST" "/api/admin/catalog/products" \
        '{"name_input":"Green Apple","auto_translate":true}')
    
    SUCCESS=$(echo "$RESPONSE" | jq -r '.success // false')
    
    if [ "$SUCCESS" = "true" ]; then
        NAME_EN=$(echo "$RESPONSE" | jq -r '.data.name_en // "N/A"')
        NAME_PL=$(echo "$RESPONSE" | jq -r '.data.name_pl // "N/A"')
        CATEGORY=$(echo "$RESPONSE" | jq -r '.data.category.name // "N/A"')
        
        if [ "$NAME_EN" = "Green Apple" ]; then
            print_success "Kept as 'Green Apple' (no truncation)"
        else
            print_failure "Expected 'Green Apple', got '$NAME_EN'"
        fi
        
        if [[ "$NAME_PL" != "N/A" ]] && [[ "$NAME_PL" != *"Jabłko"* ]]; then
            print_success "Multi-word translation preserved: '$NAME_PL'"
        else
            print_info "Polish translation: '$NAME_PL'"
        fi
        
        if [ "$CATEGORY" = "Fruits" ] || [ "$CATEGORY" = "fruits" ]; then
            print_success "Classified as Fruits"
        else
            print_info "Category is '$CATEGORY'"
        fi
    else
        print_failure "Product creation failed"
    fi
}

test_ukrainian_input() {
    print_test "Ukrainian Input: 'Яйце' (Egg)"
    
    RESPONSE=$(api_request "POST" "/api/admin/catalog/products" \
        '{"name_input":"Яйце","auto_translate":true}')
    
    SUCCESS=$(echo "$RESPONSE" | jq -r '.success // false')
    
    if [ "$SUCCESS" = "true" ]; then
        NAME_EN=$(echo "$RESPONSE" | jq -r '.data.name_en // "N/A"')
        CATEGORY=$(echo "$RESPONSE" | jq -r '.data.category.name // "N/A"')
        
        # Egg could normalize to "Egg", "Eggs", etc.
        if [[ "$NAME_EN" == *"Egg"* ]]; then
            print_success "Ukrainian normalized to '$NAME_EN'"
        else
            print_info "Got '$NAME_EN'"
        fi
        
    else
        ERROR=$(echo "$RESPONSE" | jq -r '.error // "Unknown error"')
        print_info "Response: $ERROR"
    fi
}

test_duplicate_detection() {
    print_test "Duplicate Detection"
    print_step "Creating product 'Milk' (if not exists)"
    
    # Try to create
    RESPONSE=$(api_request "POST" "/api/admin/catalog/products" \
        '{"name_input":"Milk","auto_translate":true}')
    
    SUCCESS=$(echo "$RESPONSE" | jq -r '.success // false')
    
    if [ "$SUCCESS" = "true" ]; then
        print_success "First 'Milk' created"
        
        print_step "Attempting duplicate creation"
        RESPONSE2=$(api_request "POST" "/api/admin/catalog/products" \
            '{"name_input":"Milk","auto_translate":true}')
        
        SUCCESS2=$(echo "$RESPONSE2" | jq -r '.success // false')
        
        if [ "$SUCCESS2" = "false" ]; then
            ERROR=$(echo "$RESPONSE2" | jq -r '.error // ""')
            if [[ "$ERROR" == *"already exists"* ]]; then
                print_success "Duplicate correctly rejected"
            else
                print_info "Error: $ERROR"
            fi
        else
            print_failure "Duplicate should be rejected"
        fi
    else
        ERROR=$(echo "$RESPONSE" | jq -r '.error // "Unknown error"')
        if [[ "$ERROR" == *"already exists"* ]]; then
            print_success "Milk already exists (duplicate detection working)"
        else
            print_info "Response: $ERROR"
        fi
    fi
}

test_category_override() {
    print_test "Explicit Category Override"
    print_step "Getting first category ID"
    
    CATEGORIES=$(api_request "GET" "/api/admin/catalog/categories")
    CATEGORY_ID=$(echo "$CATEGORIES" | jq -r '.data[0].id // "N/A"')
    
    if [ "$CATEGORY_ID" = "N/A" ] || [ -z "$CATEGORY_ID" ]; then
        print_failure "Could not get category ID"
        return
    fi
    
    print_step "Creating 'Cheese' with explicit category"
    RESPONSE=$(api_request "POST" "/api/admin/catalog/products" \
        "{\"name_input\":\"Cheese\",\"category_id\":\"$CATEGORY_ID\",\"auto_translate\":true}")
    
    SUCCESS=$(echo "$RESPONSE" | jq -r '.success // false')
    
    if [ "$SUCCESS" = "true" ]; then
        RETURNED_CATEGORY_ID=$(echo "$RESPONSE" | jq -r '.data.category_id // "N/A"')
        if [ "$RETURNED_CATEGORY_ID" = "$CATEGORY_ID" ]; then
            print_success "Used provided category override"
        else
            print_info "Category ID: $RETURNED_CATEGORY_ID"
        fi
    else
        ERROR=$(echo "$RESPONSE" | jq -r '.error // "Unknown error"')
        print_info "Response: $ERROR"
    fi
}

test_empty_input() {
    print_test "Edge Case: Empty Input"
    
    RESPONSE=$(api_request "POST" "/api/admin/catalog/products" \
        '{"name_input":"","auto_translate":true}')
    
    SUCCESS=$(echo "$RESPONSE" | jq -r '.success // false')
    
    if [ "$SUCCESS" = "false" ]; then
        print_success "Empty input properly rejected"
    else
        print_failure "Should reject empty input"
    fi
}

test_long_input() {
    print_test "Edge Case: Very Long Input"
    
    LONG_NAME=$(python3 -c "print('A' * 200)")
    
    RESPONSE=$(api_request "POST" "/api/admin/catalog/products" \
        "{\"name_input\":\"$LONG_NAME\",\"auto_translate\":true}")
    
    SUCCESS=$(echo "$RESPONSE" | jq -r '.success // false')
    
    if [ "$SUCCESS" = "false" ]; then
        print_success "Long input properly rejected"
    else
        print_failure "Should reject very long input"
    fi
}

################################################################################
# Main Test Execution
################################################################################

main() {
    print_header "🧪 Universal Input Architecture - Test Suite"
    
    print_info "API URL: $API_URL"
    print_info "Test Date: $(date)"
    
    # Run all tests
    test_auth
    echo ""
    
    test_russian_input
    echo ""
    
    test_polish_input
    echo ""
    
    test_english_multiword
    echo ""
    
    test_ukrainian_input
    echo ""
    
    test_duplicate_detection
    echo ""
    
    test_category_override
    echo ""
    
    test_empty_input
    echo ""
    
    test_long_input
    echo ""
    
    # Summary
    print_header "📊 Test Summary"
    
    echo -e "Total Tests:  ${CYAN}$TESTS_TOTAL${NC}"
    echo -e "Passed:       ${GREEN}$TESTS_PASSED${NC}"
    echo -e "Failed:       ${RED}$TESTS_FAILED${NC}"
    
    if [ $TESTS_FAILED -eq 0 ]; then
        echo ""
        echo -e "${GREEN}🎉 All tests passed!${NC}"
        return 0
    else
        echo ""
        echo -e "${RED}⚠️  Some tests failed${NC}"
        return 1
    fi
}

# Run main function
main
