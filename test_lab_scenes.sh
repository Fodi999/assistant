#!/bin/bash
# Smoke test for /generate-scenes (Step 9)
set -euo pipefail
API="http://localhost:8000/api"
EMAIL="lab_scenes_$(date +%s)@test.dev"
PASS="Test1234!"

echo "=== 1) Register ==="
REG=$(curl -s -X POST "$API/auth/register" \
  -H "Content-Type: application/json" \
  -d "{\"email\":\"$EMAIL\",\"password\":\"$PASS\",\"name\":\"Lab Tester\",\"restaurant_name\":\"Lab\"}")
TOKEN=$(echo "$REG" | python3 -c "import sys,json; d=json.load(sys.stdin); print(d.get('access_token') or d.get('token') or '')")
if [ -z "$TOKEN" ]; then echo "register failed: $REG"; exit 1; fi
echo "  token: ${TOKEN:0:30}..."

H_AUTH="Authorization: Bearer $TOKEN"
H_JSON="Content-Type: application/json"

echo "=== 2) Create project (target=sauce) ==="
PROJ=$(curl -s -X POST "$API/laboratory/projects" -H "$H_AUTH" -H "$H_JSON" \
  -d '{"name":"Apricot sauce demo","target_product_type":"sauce"}')
PID=$(echo "$PROJ" | python3 -c "import sys,json; print(json.load(sys.stdin)['id'])")
echo "  project: $PID"

echo "=== 3) Add ingredient apricot 300g (role=base) ==="
curl -s -X POST "$API/laboratory/projects/$PID/ingredients" -H "$H_AUTH" -H "$H_JSON" \
  -d '{"ingredient_slug":"apricot","quantity":"300","unit":"g","role":"base"}' > /dev/null

echo "=== 4) Add step heat 85C 15min ==="
curl -s -X POST "$API/laboratory/projects/$PID/steps" -H "$H_AUTH" -H "$H_JSON" \
  -d '{"technique":"heat","temperature_c":"85","duration_min":15,"target_slugs":["apricot"]}' > /dev/null

echo "=== 5) Add step blend ==="
curl -s -X POST "$API/laboratory/projects/$PID/steps" -H "$H_AUTH" -H "$H_JSON" \
  -d '{"technique":"blend","duration_min":2,"target_slugs":["apricot"]}' > /dev/null

echo "=== 6) /analyze ==="
curl -s -X POST "$API/laboratory/projects/$PID/analyze" -H "$H_AUTH" \
  | python3 -c "import sys,json; d=json.load(sys.stdin); a=d.get('latest_analysis') or {}; print('  shelf_life_days:', a.get('shelf_life_days'), 'risk:', a.get('risk_level'))"

echo ""
echo "=== 7) /generate-scenes ==="
curl -s -X POST "$API/laboratory/projects/$PID/generate-scenes" -H "$H_AUTH" | python3 -m json.tool

echo ""
echo "=== 8) Negative case: project without analyze ==="
PROJ2=$(curl -s -X POST "$API/laboratory/projects" -H "$H_AUTH" -H "$H_JSON" \
  -d '{"name":"empty proj"}')
PID2=$(echo "$PROJ2" | python3 -c "import sys,json; print(json.load(sys.stdin)['id'])")
echo "  project: $PID2"
echo "  HTTP status:"
curl -s -o /tmp/scenes_err.json -w "%{http_code}\n" -X POST "$API/laboratory/projects/$PID2/generate-scenes" -H "$H_AUTH"
cat /tmp/scenes_err.json | python3 -m json.tool

echo ""
echo "=== 9) Negative case: bogus project id ==="
curl -s -o /tmp/scenes_404.json -w "%{http_code}\n" -X POST "$API/laboratory/projects/00000000-0000-0000-0000-000000000000/generate-scenes" -H "$H_AUTH"
cat /tmp/scenes_404.json | python3 -m json.tool
