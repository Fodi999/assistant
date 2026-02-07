#!/bin/bash

# Menu Engineering API Test Script
# Tests Star/Plowhorse/Puzzle/Dog classification

set -e

API_URL="http://localhost:8080/api"

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

print_success() {
    echo -e "${GREEN}✓ $1${NC}"
}

print_error() {
    echo -e "${RED}✗ $1${NC}"
}

print_info() {
    echo -e "${BLUE}ℹ $1${NC}"
}

print_warning() {
    echo -e "${YELLOW}⚠ $1${NC}"
}

echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${BLUE}   MENU ENGINEERING TEST (ШАГИ 1)${NC}"
echo -e "${BLUE}   Star / Plowhorse / Puzzle / Dog Classification${NC}"
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""

# Step 1: Register and login
print_info "Step 1: Creating test user..."
TIMESTAMP=$(date +%s)
EMAIL="menu_test_${TIMESTAMP}@test.com"
PASSWORD="Test123!"

REGISTER_RESPONSE=$(curl -s -X POST "$API_URL/auth/register" \
    -H "Content-Type: application/json" \
    -d "{
        \"email\": \"$EMAIL\",
        \"password\": \"$PASSWORD\",
        \"display_name\": \"Menu Test User\",
        \"restaurant_name\": \"Menu Engineering Test Restaurant\"
    }")

echo "$REGISTER_RESPONSE" | jq . > /dev/null || {
    print_error "Registration failed"
    echo "$REGISTER_RESPONSE"
    exit 1
}

print_success "User registered: $EMAIL"

# Login
LOGIN_RESPONSE=$(curl -s -X POST "$API_URL/auth/login" \
    -H "Content-Type: application/json" \
    -d "{
        \"email\": \"$EMAIL\",
        \"password\": \"$PASSWORD\"
    }")

ACCESS_TOKEN=$(echo "$LOGIN_RESPONSE" | jq -r '.access_token')

if [ "$ACCESS_TOKEN" = "null" ] || [ -z "$ACCESS_TOKEN" ]; then
    print_error "Login failed"
    echo "$LOGIN_RESPONSE"
    exit 1
fi

print_success "Logged in successfully"
echo ""

# Step 2: Get ingredient IDs from catalog
print_info "Step 2: Getting catalog ingredients..."

# Use exact IDs from database
CHICKEN_ID="56066c9b-73ee-4dc4-8490-f5ee9b0da452"  # Chicken breast
RICE_ID="39d2b784-87fb-48b1-a680-1f4f426f8440"     # Rice
TOMATO_ID="410b66ee-c19c-4971-86ba-678207cf14d9"   # Tomato

print_success "Chicken ID: $CHICKEN_ID"
print_success "Rice ID: $RICE_ID"
print_success "Tomato ID: $TOMATO_ID"
echo ""

# Step 3: Setup inventory
print_info "Step 3: Adding inventory products..."

curl -s -X POST "$API_URL/assistant/command" \
    -H "Authorization: Bearer $ACCESS_TOKEN" \
    -H "Content-Type: application/json" \
    -d '{"command": {"type": "start_inventory"}}' > /dev/null

# Add Chicken - expensive (100 PLN)
curl -s -X POST "$API_URL/assistant/command" \
    -H "Authorization: Bearer $ACCESS_TOKEN" \
    -H "Content-Type: application/json" \
    -d "{
        \"command\": {
            \"type\": \"add_product\",
            \"catalog_ingredient_id\": \"$CHICKEN_ID\",
            \"quantity\": 10.0,
            \"price_cents\": 10000
        }
    }" > /dev/null

# Add Rice - cheap (5 PLN)
curl -s -X POST "$API_URL/assistant/command" \
    -H "Authorization: Bearer $ACCESS_TOKEN" \
    -H "Content-Type: application/json" \
    -d "{
        \"command\": {
            \"type\": \"add_product\",
            \"catalog_ingredient_id\": \"$RICE_ID\",
            \"quantity\": 5.0,
            \"price_cents\": 500
        }
    }" > /dev/null

# Add Tomato - medium (3 PLN)
curl -s -X POST "$API_URL/assistant/command" \
    -H "Authorization: Bearer $ACCESS_TOKEN" \
    -H "Content-Type: application/json" \
    -d "{
        \"command\": {
            \"type\": \"add_product\",
            \"catalog_ingredient_id\": \"$TOMATO_ID\",
            \"quantity\": 2.0,
            \"price_cents\": 300
        }
    }" > /dev/null

print_success "Inventory setup complete"
echo ""

# Step 4: Create recipes
print_info "Step 4: Creating recipes..."

curl -s -X POST "$API_URL/assistant/command" \
    -H "Authorization: Bearer $ACCESS_TOKEN" \
    -H "Content-Type: application/json" \
    -d '{"command": {"type": "start_recipe"}}' > /dev/null

# Recipe 1: Chicken Steak (expensive, will be low-margin if priced wrong)
RECIPE1=$(curl -s -X POST "$API_URL/recipes" \
    -H "Authorization: Bearer $ACCESS_TOKEN" \
    -H "Content-Type: application/json" \
    -d "{
        \"name\": \"Chicken Steak\",
        \"servings\": 1,
        \"ingredients\": [
            {\"catalog_ingredient_id\": \"$CHICKEN_ID\", \"quantity\": 0.3}
        ]
    }")

DISH1_ID=$(echo "$RECIPE1" | jq -r '.id')
print_success "Recipe 1 created: Chicken Steak (Cost: 3 PLN)"

# Recipe 2: Rice with Tomato (cheap)
RECIPE2=$(curl -s -X POST "$API_URL/recipes" \
    -H "Authorization: Bearer $ACCESS_TOKEN" \
    -H "Content-Type: application/json" \
    -d "{
        \"name\": \"Rice with Tomato\",
        \"servings\": 1,
        \"ingredients\": [
            {\"catalog_ingredient_id\": \"$RICE_ID\", \"quantity\": 0.2},
            {\"catalog_ingredient_id\": \"$TOMATO_ID\", \"quantity\": 0.1}
        ]
    }")

DISH2_ID=$(echo "$RECIPE2" | jq -r '.id')
print_success "Recipe 2 created: Rice with Tomato (Cost: 0.25 PLN)"

# Recipe 3: Full Meal (moderate cost)
RECIPE3=$(curl -s -X POST "$API_URL/recipes" \
    -H "Authorization: Bearer $ACCESS_TOKEN" \
    -H "Content-Type: application/json" \
    -d "{
        \"name\": \"Full Meal\",
        \"servings\": 1,
        \"ingredients\": [
            {\"catalog_ingredient_id\": \"$CHICKEN_ID\", \"quantity\": 0.2},
            {\"catalog_ingredient_id\": \"$RICE_ID\", \"quantity\": 0.3},
            {\"catalog_ingredient_id\": \"$TOMATO_ID\", \"quantity\": 0.15}
        ]
    }")

DISH3_ID=$(echo "$RECIPE3" | jq -r '.id')
print_success "Recipe 3 created: Full Meal (Cost: 2.49 PLN)"
echo ""

# Step 5: Create dishes with different margins
print_info "Step 5: Creating dishes via API..."

# Dish 1: Chicken Steak - 65 PLN (will be STAR if popular)
DISH1_RESPONSE=$(curl -s -X POST "$API_URL/dishes" \
    -H "Authorization: Bearer $ACCESS_TOKEN" \
    -H "Content-Type: application/json" \
    -d "{
        \"recipe_id\": \"$DISH1_ID\",
        \"name\": \"Premium Chicken Steak\",
        \"description\": \"Grilled chicken breast\",
        \"selling_price_cents\": 6500
    }")

DISH1_REAL_ID=$(echo "$DISH1_RESPONSE" | jq -r '.id')
print_success "Dish 1 created: $DISH1_REAL_ID"

# Dish 2: Rice with Tomato - 5 PLN (will be PUZZLE if not popular)
DISH2_RESPONSE=$(curl -s -X POST "$API_URL/dishes" \
    -H "Authorization: Bearer $ACCESS_TOKEN" \
    -H "Content-Type: application/json" \
    -d "{
        \"recipe_id\": \"$DISH2_ID\",
        \"name\": \"Vegetarian Rice Bowl\",
        \"description\": \"Healthy rice with fresh tomatoes\",
        \"selling_price_cents\": 500
    }")

DISH2_REAL_ID=$(echo "$DISH2_RESPONSE" | jq -r '.id')
print_success "Dish 2 created: $DISH2_REAL_ID"

# Dish 3: Full Meal - 15 PLN (will depend on sales)
DISH3_RESPONSE=$(curl -s -X POST "$API_URL/dishes" \
    -H "Authorization: Bearer $ACCESS_TOKEN" \
    -H "Content-Type: application/json" \
    -d "{
        \"recipe_id\": \"$DISH3_ID\",
        \"name\": \"Chef's Special\",
        \"description\": \"Complete meal with chicken, rice, and vegetables\",
        \"selling_price_cents\": 1500
    }")

DISH3_REAL_ID=$(echo "$DISH3_RESPONSE" | jq -r '.id')
print_success "Dish 3 created: $DISH3_REAL_ID"

print_success "3 dishes created with different margins"
echo ""

# Step 6: Simulate sales (record dish_sales)
print_info "Step 6: Simulating sales data..."

# Chicken Steak - HIGH sales (popular) → STAR (high margin + high sales)
for i in {1..20}; do
    curl -s -X POST "$API_URL/menu-engineering/sales" \
        -H "Authorization: Bearer $ACCESS_TOKEN" \
        -H "Content-Type: application/json" \
        -d "{
            \"dish_id\": \"$DISH1_REAL_ID\",
            \"quantity\": 1,
            \"selling_price_cents\": 6500,
            \"recipe_cost_cents\": 300
        }" > /dev/null
done

# Rice Bowl - LOW sales (unpopular) → PUZZLE (high margin + low sales)
for i in {1..2}; do
    curl -s -X POST "$API_URL/menu-engineering/sales" \
        -H "Authorization: Bearer $ACCESS_TOKEN" \
        -H "Content-Type: application/json" \
        -d "{
            \"dish_id\": \"$DISH2_REAL_ID\",
            \"quantity\": 1,
            \"selling_price_cents\": 500,
            \"recipe_cost_cents\": 25
        }" > /dev/null
done

# Full Meal - MEDIUM sales → Could be PLOWHORSE or STAR
for i in {1..10}; do
    curl -s -X POST "$API_URL/menu-engineering/sales" \
        -H "Authorization: Bearer $ACCESS_TOKEN" \
        -H "Content-Type: application/json" \
        -d "{
            \"dish_id\": \"$DISH3_REAL_ID\",
            \"quantity\": 1,
            \"selling_price_cents\": 1500,
            \"recipe_cost_cents\": 249
        }" > /dev/null
done

print_success "Sales data recorded: 20 steaks, 2 rice bowls, 10 full meals"
echo ""

# Step 7: Analyze Menu Engineering
print_info "Step 7: Getting Menu Engineering analysis..."
echo ""

ANALYSIS=$(curl -s -X GET "$API_URL/menu-engineering/analysis?period_days=30&language=en" \
    -H "Authorization: Bearer $ACCESS_TOKEN")

echo "$ANALYSIS" | jq . > /dev/null || {
    print_error "Analysis failed"
    echo "$ANALYSIS"
    exit 1
}

echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${BLUE}   MENU ENGINEERING MATRIX${NC}"
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""

# Extract summary
TOTAL_DISHES=$(echo "$ANALYSIS" | jq '.total_dishes')
STARS=$(echo "$ANALYSIS" | jq '.stars')
PLOWHORSES=$(echo "$ANALYSIS" | jq '.plowhorses')
PUZZLES=$(echo "$ANALYSIS" | jq '.puzzles')
DOGS=$(echo "$ANALYSIS" | jq '.dogs')
AVG_MARGIN=$(echo "$ANALYSIS" | jq '.avg_profit_margin')

print_info "Total Dishes: $TOTAL_DISHES"
echo -e "  ⭐ Stars: ${GREEN}$STARS${NC}"
echo -e "  🐴 Plowhorses: ${YELLOW}$PLOWHORSES${NC}"
echo -e "  ❓ Puzzles: ${YELLOW}$PUZZLES${NC}"
echo -e "  🐶 Dogs: ${RED}$DOGS${NC}"
echo -e "  📊 Avg Margin: ${BLUE}${AVG_MARGIN}%${NC}"
echo ""

# Show each dish
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${BLUE}   DISH PERFORMANCE${NC}"
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""

DISH_COUNT=$(echo "$ANALYSIS" | jq '.dishes | length')

for i in $(seq 0 $((DISH_COUNT - 1))); do
    DISH=$(echo "$ANALYSIS" | jq ".dishes[$i]")
    
    NAME=$(echo "$DISH" | jq -r '.dish_name')
    CATEGORY=$(echo "$DISH" | jq -r '.category')
    ABC_CLASS=$(echo "$DISH" | jq -r '.abc_class')
    MARGIN=$(echo "$DISH" | jq -r '.profit_margin_percent')
    POPULARITY=$(echo "$DISH" | jq -r '.popularity_score')
    SALES=$(echo "$DISH" | jq -r '.sales_volume')
    REVENUE=$(echo "$DISH" | jq -r '.total_revenue_cents')
    RECOMMENDATION=$(echo "$DISH" | jq -r '.recommendation')
    STRATEGY=$(echo "$DISH" | jq -r '.strategy')
    
    EMOJI=""
    COLOR=""
    case "$CATEGORY" in
        "star")
            EMOJI="⭐"
            COLOR="${GREEN}"
            ;;
        "plowhorse")
            EMOJI="🐴"
            COLOR="${YELLOW}"
            ;;
        "puzzle")
            EMOJI="❓"
            COLOR="${YELLOW}"
            ;;
        "dog")
            EMOJI="🐶"
            COLOR="${RED}"
            ;;
    esac
    
    # ABC emoji
    ABC_EMOJI=""
    case "$ABC_CLASS" in
        "A")
            ABC_EMOJI="🥇"
            ;;
        "B")
            ABC_EMOJI="🥈"
            ;;
        "C")
            ABC_EMOJI="🥉"
            ;;
    esac
    
    echo -e "${COLOR}${EMOJI} $NAME ${ABC_EMOJI} [ABC: ${ABC_CLASS}]${NC}"
    echo -e "  Category: ${COLOR}${CATEGORY}${NC} | ABC Class: ${ABC_CLASS}"
    echo -e "  Margin: ${MARGIN}% | Popularity: ${POPULARITY} | Sales: ${SALES}"
    echo -e "  Revenue: $((REVENUE / 100)) PLN"
    echo -e "  ${BLUE}→ Strategy: ${STRATEGY}${NC}"
    echo ""
done

echo -e "${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${GREEN}   ✓ MENU ENGINEERING TEST COMPLETED${NC}"
echo -e "${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
