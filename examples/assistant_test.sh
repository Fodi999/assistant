#!/bin/bash
set -e

BASE_URL="http://localhost:8080/api/assistant"

echo "🧪 Testing Guided Assistant State Machine"
echo "=========================================="
echo ""

# Test 1: Get initial state
echo "1️⃣  GET /api/assistant/state"
curl -s $BASE_URL/state | jq '.step, .progress, .message'
echo ""

# Test 2: Start → InventorySetup
echo "2️⃣  Start → InventorySetup"
curl -s -X POST $BASE_URL/command \
  -H "Content-Type: application/json" \
  -d '{"step": "Start", "command": "start_inventory"}' | jq '.step, .progress'
echo ""

# Test 3: InventorySetup → RecipeSetup
echo "3️⃣  InventorySetup → RecipeSetup"
curl -s -X POST $BASE_URL/command \
  -H "Content-Type: application/json" \
  -d '{"step": "InventorySetup", "command": "finish_inventory"}' | jq '.step, .progress'
echo ""

# Test 4: RecipeSetup → DishSetup
echo "4️⃣  RecipeSetup → DishSetup"
curl -s -X POST $BASE_URL/command \
  -H "Content-Type: application/json" \
  -d '{"step": "RecipeSetup", "command": "finish_recipes"}' | jq '.step, .progress'
echo ""

# Test 5: DishSetup → Report
echo "5️⃣  DishSetup → Report"
curl -s -X POST $BASE_URL/command \
  -H "Content-Type: application/json" \
  -d '{"step": "DishSetup", "command": "finish_dishes"}' | jq '.step, .progress'
echo ""

# Test 6: Report → Completed
echo "6️⃣  Report → Completed"
curl -s -X POST $BASE_URL/command \
  -H "Content-Type: application/json" \
  -d '{"step": "Report", "command": "view_report"}' | jq '.step, .progress'
echo ""

# Test 7: Invalid transition (should stay at Start)
echo "7️⃣  Invalid transition test: Start + finish_recipes (should stay at Start)"
curl -s -X POST $BASE_URL/command \
  -H "Content-Type: application/json" \
  -d '{"step": "Start", "command": "finish_recipes"}' | jq '.step, .message'
echo ""

echo "✅ All tests completed!"
