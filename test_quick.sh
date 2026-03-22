#!/bin/bash
# Quick test: queue system + priority + scheduler
# Run AFTER server is started separately

BASE="http://localhost:8000/api"

# 1. Login
echo "=== LOGIN ==="
TOKEN=$(curl -s "$BASE/admin/auth/login" \
  -H 'Content-Type: application/json' \
  -d '{"email":"admin@fodi.app","password":"Admin123!"}' \
  | python3 -c "import sys,json; print(json.load(sys.stdin)['token'])")
echo "OK: ${TOKEN:0:20}..."

# 2. Stats
echo ""
echo "=== STATS ==="
curl -s "$BASE/admin/intent-pages/stats" -H "Authorization: Bearer $TOKEN" | python3 -m json.tool

# 3. Settings
echo ""
echo "=== SETTINGS ==="
curl -s "$BASE/admin/intent-pages/settings" -H "Authorization: Bearer $TOKEN" | python3 -m json.tool

# 4. Generate 2 single pages (fast — one at a time)
echo ""
echo "=== GENERATE page 1 (are-almonds-healthy) ==="
P1=$(curl -s --max-time 60 -X POST "$BASE/admin/intent-pages/generate" \
  -H "Authorization: Bearer $TOKEN" \
  -H 'Content-Type: application/json' \
  -d '{"intent_type":"question","entity_a":"almonds","locale":"en","sub_intent":"is {a} healthy"}')
P1_ID=$(echo "$P1" | python3 -c "import sys,json; d=json.load(sys.stdin); print(d.get('id','FAIL'))")
P1_SLUG=$(echo "$P1" | python3 -c "import sys,json; d=json.load(sys.stdin); print(d.get('slug','FAIL'))")
P1_TITLE=$(echo "$P1" | python3 -c "import sys,json; d=json.load(sys.stdin); print(d.get('title','FAIL'))")
echo "  id=$P1_ID"
echo "  slug=$P1_SLUG"
echo "  title=$P1_TITLE"
if echo "$P1_SLUG" | grep -q "are-"; then echo "  ✅ SLUG grammar correct (are-)"; else echo "  ⚠️ SLUG still has is-"; fi

echo ""
echo "=== GENERATE page 2 (almonds-calories) ==="
P2=$(curl -s --max-time 60 -X POST "$BASE/admin/intent-pages/generate" \
  -H "Authorization: Bearer $TOKEN" \
  -H 'Content-Type: application/json' \
  -d '{"intent_type":"question","entity_a":"almonds","locale":"en","sub_intent":"how many calories in {a}"}')
P2_ID=$(echo "$P2" | python3 -c "import sys,json; d=json.load(sys.stdin); print(d.get('id','FAIL'))")
P2_SLUG=$(echo "$P2" | python3 -c "import sys,json; d=json.load(sys.stdin); print(d.get('slug','FAIL'))")
echo "  id=$P2_ID  slug=$P2_SLUG"

# 5. Enqueue with priorities
echo ""
echo "=== ENQUEUE page1 priority=0 (low) ==="
curl -s -X POST "$BASE/admin/intent-pages/$P1_ID/enqueue" \
  -H "Authorization: Bearer $TOKEN" | python3 -c "import sys,json; d=json.load(sys.stdin); print(f'  status={d.get(\"status\")}, queued_at={d.get(\"queued_at\")}')"

echo ""
echo "=== ENQUEUE page2 priority=2 (high) via bulk ==="
curl -s -X POST "$BASE/admin/intent-pages/enqueue-bulk" \
  -H "Authorization: Bearer $TOKEN" \
  -H 'Content-Type: application/json' \
  -d "{\"ids\": [\"$P2_ID\"], \"priority\": 2}" | python3 -m json.tool

# 6. Stats — should show queued_high=1, queued_low=1
echo ""
echo "=== STATS after enqueue ==="
curl -s "$BASE/admin/intent-pages/stats" -H "Authorization: Bearer $TOKEN" | python3 -m json.tool

# 7. Run scheduler — should publish HIGH priority first
echo ""
echo "=== UPDATE settings: limit=1 (only 1 per day) ==="
curl -s -X PUT "$BASE/admin/intent-pages/settings" \
  -H "Authorization: Bearer $TOKEN" \
  -H 'Content-Type: application/json' \
  -d '{"publish_limit_per_day": 1}' | python3 -m json.tool

echo ""
echo "=== RUN SCHEDULER ==="
curl -s -X POST "$BASE/admin/intent-pages/scheduler/run" \
  -H "Authorization: Bearer $TOKEN" | python3 -m json.tool

# 8. Check — page2 (high priority) should be published, page1 (low) still queued
echo ""
echo "=== FINAL page statuses ==="
curl -s "$BASE/admin/intent-pages" -H "Authorization: Bearer $TOKEN" | python3 -c "
import sys, json
pages = json.load(sys.stdin)
if isinstance(pages, dict): pages = pages.get('pages', [])
for p in pages:
    print(f'  {p[\"id\"][:8]}  status={p.get(\"status\",\"?\"):10s}  priority={p.get(\"priority\",\"?\")}  slug={p.get(\"slug\",\"?\")}')
"

echo ""
echo "=== FINAL STATS ==="
curl -s "$BASE/admin/intent-pages/stats" -H "Authorization: Bearer $TOKEN" | python3 -m json.tool

# Restore settings
echo ""
echo "=== RESTORE settings: limit=24 ==="
curl -s -X PUT "$BASE/admin/intent-pages/settings" \
  -H "Authorization: Bearer $TOKEN" \
  -H 'Content-Type: application/json' \
  -d '{"publish_limit_per_day": 24}' | python3 -m json.tool

echo ""
echo "=== ALL TESTS DONE ==="
