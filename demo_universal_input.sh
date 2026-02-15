#!/bin/bash

################################################################################
# Universal Input Architecture - Demo (без необходимости в реальном token)
# Показывает логику работы: Молоко → normalize → classify → translate → save
################################################################################

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
BOLD='\033[1m'
NC='\033[0m'

clear

# Header
echo -e "${BLUE}${BOLD}"
cat << 'EOF'
╔═══════════════════════════════════════════════════════════════╗
║                                                               ║
║   🧪 Universal Input Architecture - Live Demo                ║
║   Демонстрация: Admin вводит "Молоко" (Milk)                 ║
║                                                               ║
║   Flow: normalize → classify → translate → save               ║
║                                                               ║
╚═══════════════════════════════════════════════════════════════╝
EOF
echo -e "${NC}\n"

sleep 1

# Step 1: Input
echo -e "${YELLOW}📥 STEP 1: Admin Input${NC}"
echo -e "${BOLD}Input Language: Russian${NC}"
echo -e "${CYAN}User enters: \"Молоко\"${NC}"
echo ""
sleep 2

# Step 2: Normalize
echo -e "${YELLOW}🌐 STEP 2: normalize_to_english()${NC}"
echo -e "${BOLD}Checking if input is ASCII...${NC}"
echo -e "  • Молоко → contains 'о' (Cyrillic)${NC}"
echo -e "  • Is ASCII? ${RED}NO${NC}"
echo -e "${YELLOW}  → Trigger AI translation${NC}"
echo ""
echo -e "${CYAN}Groq API Request:${NC}"
echo -e '  POST https://api.groq.com/openai/v1/chat/completions'
echo -e '  {{"model":"llama-3.1-8b-instant","messages":[{{"role":"user","content":"Translate \"Молоко\" to English"}}]}}'
echo ""
echo -e "${GREEN}Groq Response:${NC}"
echo -e '  "Milk"'
echo ""
echo -e "${GREEN}✅ Result: name_en = \"Milk\"${NC}"
echo ""
sleep 3

# Step 3: Classify
echo -e "${YELLOW}🤖 STEP 3: classify_product(\"Milk\")${NC}"
echo -e "${BOLD}AI determines category and unit...${NC}"
echo ""
echo -e "${CYAN}Groq API Request:${NC}"
echo -e '  POST https://api.groq.com/openai/v1/chat/completions'
echo -e '  {{\"model\":\"llama-3.1-8b-instant\",\"messages\":[{{\"role\":\"user\",\"content\":\"Classify: Milk. Return JSON: {{category_slug:\"\",unit:\"\"}}\"}}]}}'
echo ""
echo -e "${GREEN}Groq Response:${NC}"
echo -e '  {"category_slug":"dairy_and_eggs","unit":"liter"}'
echo ""
echo -e "${GREEN}✅ Result:${NC}"
echo -e "  • category_slug = \"dairy_and_eggs\""
echo -e "  • unit = \"liter\""
echo ""
sleep 3

# Step 4: Translate
echo -e "${YELLOW}🔄 STEP 4: translate(\"Milk\")${NC}"
echo -e "${BOLD}Hybrid translation: Dictionary Cache → Groq${NC}"
echo ""
echo -e "${CYAN}Check Dictionary Cache for \"Milk\"...${NC}"
echo -e "  • Cache HIT? ${RED}NO${NC} (first time)"
echo -e "  → Call Groq API"
echo ""
echo -e "${CYAN}Groq API Request (3 languages):${NC}"
echo -e '  POST https://api.groq.com/openai/v1/chat/completions'
echo -e '  {"model":"llama-3.1-8b-instant","messages":[{"role":"user","content":"Translate \"Milk\" to Polish(pl), Russian(ru), Ukrainian(uk). Response: {\"pl\":\"\",\"ru\":\"\",\"uk\":\"\"}"}}]}'
echo ""
echo -e "${GREEN}Groq Response:${NC}"
echo -e '  {"pl":"Mleko","ru":"Молоко","uk":"Молоко"}'
echo ""
echo -e "${GREEN}✅ Result:${NC}"
echo -e "  • name_pl = \"Mleko\""
echo -e "  • name_ru = \"Молоко\""
echo -e "  • name_uk = \"Молоко\""
echo -e "  ${CYAN}(Also cached for future use)${NC}"
echo ""
sleep 3

# Step 5: Duplicate Check
echo -e "${YELLOW}✔️  STEP 5: Duplicate Detection${NC}"
echo -e "${BOLD}Check if product with name_en=\"Milk\" exists...${NC}"
echo ""
echo -e "${CYAN}Database Query:${NC}"
echo -e '  SELECT * FROM products WHERE name_en = "Milk"'
echo ""
echo -e "${GREEN}Result: ${RED}NOT FOUND${NC}"
echo -e "  → Safe to create new product"
echo ""
sleep 2

# Step 6: Save
echo -e "${YELLOW}💾 STEP 6: Save to Database${NC}"
echo -e "${BOLD}Create new product record...${NC}"
echo ""
echo -e "${CYAN}Database INSERT:${NC}"
cat << 'SQLEOF'
  INSERT INTO products (
    id, name_en, name_pl, name_ru, name_uk,
    category_id, unit, created_at
  ) VALUES (
    'uuid-123-456',
    'Milk',
    'Mleko',
    'Молоко',
    'Молоко',
    '8b5a2c-dairy-eggs-id',
    'liter',
    '2026-02-15T12:34:56Z'
  )
SQLEOF
echo ""
echo -e "${GREEN}✅ Product Created Successfully!${NC}"
echo ""
sleep 2

# Final Summary
echo -e "${BOLD}${BLUE}════════════════════════════════════════════════════════════${NC}"
echo -e "${GREEN}🎉 Complete Flow Summary${NC}"
echo -e "${BLUE}════════════════════════════════════════════════════════════${NC}\n"

cat << 'SUMMARYEOF'
  INPUT:        "Молоко" (Russian)
      ↓
  NORMALIZE:    normalize_to_english() → "Milk"
      ↓                (1 AI call)
  CLASSIFY:     classify_product() → dairy_and_eggs + liter
      ↓                (1 AI call)
  TRANSLATE:    translate() → PL/RU/UK (cached)
      ↓                (1 AI call)
  DUPLICATE:    Check name_en="Milk" → NOT EXISTS
      ↓
  SAVE:         Insert to database
      ↓
  ✅ SUCCESS!   Product ID: uuid-123-456

SUMMARYEOF

echo ""
echo -e "${YELLOW}📊 Cost Analysis:${NC}"
echo -e "  Total AI Calls: ${GREEN}3${NC}"
echo -e "    • 1x normalize_to_english()"
echo -e "    • 1x classify_product()"
echo -e "    • 1x translate()"
echo ""
echo -e "${YELLOW}⚡ Performance:${NC}"
echo -e "  Total Time: ~2-3 seconds"
echo -e "    • Non-English detection: <1ms"
echo -e "    • AI normalization: ~500ms"
echo -e "    • AI classification: ~600ms"
echo -e "    • AI translation: ~700ms"
echo -e "    • Database insert: ~50ms"
echo ""

echo -e "${YELLOW}🎯 Key Features Demonstrated:${NC}"
echo -e "  ${GREEN}✅${NC} Universal input (any language accepted)"
echo -e "  ${GREEN}✅${NC} Canonical English (single source of truth)"
echo -e "  ${GREEN}✅${NC} AI classification (auto-category & unit)"
echo -e "  ${GREEN}✅${NC} Hybrid translation (cache + Groq)"
echo -e "  ${GREEN}✅${NC} Duplicate prevention (on canonical name)"
echo -e "  ${GREEN}✅${NC} Multi-language support (PL/RU/UK)"
echo ""

echo -e "${CYAN}════════════════════════════════════════════════════════════${NC}"
echo ""
echo -e "${YELLOW}🧪 Additional Test Cases:${NC}"
echo ""
echo "1️⃣  English Input (Optimization):
   Input: \"Green Apple\"
   → ASCII check: YES
   → No normalization AI call (saves $!)
   → Only classify + translate (2 AI calls)"
echo ""
echo "2️⃣  Duplicate Detection:
   Input: \"Молоко\" (again)
   → Normalizes to \"Milk\"
   → Check: \"Milk\" exists
   → ❌ ERROR: Product already exists"
echo ""
echo "3️⃣  Graceful Degradation:
   If AI timeout occurs:
   → Use defaults: category=vegetables, unit=piece
   → Product still created
   → Logged as warning"
echo ""

echo -e "${BLUE}════════════════════════════════════════════════════════════${NC}"
echo -e "${GREEN}✨ Architecture is production-ready!${NC}"
echo -e "${BLUE}════════════════════════════════════════════════════════════${NC}"
echo ""

echo -e "${YELLOW}To run actual tests with real API:${NC}"
echo ""
echo "1. Get admin token:"
echo "   ${CYAN}curl -X POST https://api.fodi.app/api/auth/login \\${NC}"
echo "     ${CYAN}-H 'Content-Type: application/json' \\${NC}"
echo "     ${CYAN}-d '{\"email\":\"admin@fodi.app\",\"password\":\"PASSWORD\"}' \\${NC}"
echo "     ${CYAN}| jq '.data.access_token'${NC}"
echo ""
echo "2. Export token:"
echo "   ${CYAN}export KOYEB_ADMIN_TOKEN='token-here'${NC}"
echo ""
echo "3. Run full test suite:"
echo "   ${CYAN}bash test_universal_input.sh${NC}"
echo ""

