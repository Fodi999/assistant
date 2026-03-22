#!/bin/bash
set -e

BASE="http://localhost:8000"
API="$BASE/api"

echo "=== 1. Login ==="
LOGIN_RESP=$(curl -s "$API/admin/auth/login" \
  -H 'Content-Type: application/json' \
  -d '{"email":"admin@fodi.app","password":"Admin123!"}')
echo "Login response: $LOGIN_RESP"
TOKEN=$(echo "$LOGIN_RESP" | python3 -c "import sys,json; print(json.load(sys.stdin)['token'])")
echo "TOKEN=${TOKEN:0:20}..."

echo ""
echo "=== 2. Current stats ==="
curl -s "$API/admin/intent-pages/stats" \
  -H "Authorization: Bearer $TOKEN" | python3 -m json.tool

echo ""
echo "=== 3. Get settings ==="
curl -s "$API/admin/intent-pages/settings" \
  -H "Authorization: Bearer $TOKEN" | python3 -m json.tool

echo ""
echo "=== 4. List existing pages ==="
PAGES=$(curl -s "$API/admin/intent-pages" \
  -H "Authorization: Bearer $TOKEN")
echo "$PAGES" | python3 -c "
import sys, json
data = json.load(sys.stdin)
pages = data.get('pages', data) if isinstance(data, dict) else data
for p in (pages if isinstance(pages, list) else []):
    print(f\"  {p['id'][:8]}  status={p.get('status','?'):10s}  priority={p.get('priority','?')}  slug={p.get('slug','?')}\")
print(f'Total: {len(pages if isinstance(pages, list) else [])}')
"

echo ""
echo "=== 5. Delete old pages & clear AI cache ==="
# Get all page IDs and delete them
IDS=$(echo "$PAGES" | python3 -c "
import sys, json
data = json.load(sys.stdin)
pages = data.get('pages', data) if isinstance(data, dict) else data
for p in (pages if isinstance(pages, list) else []):
    print(p['id'])
")
for id in $IDS; do
  CODE=$(curl -s -o /dev/null -w "%{http_code}" -X DELETE "$API/admin/intent-pages/$id" \
    -H "Authorization: Bearer $TOKEN")
  echo "  DELETE $id -> $CODE"
done

echo ""
echo "=== 6. Generate fresh batch (almonds, en) — test slug grammar ==="
echo "  (this may take 30-60s for AI generation...)"
BATCH=$(curl -s --max-time 120 -X POST "$API/admin/intent-pages/generate-batch" \
  -H "Authorization: Bearer $TOKEN" \
  -H 'Content-Type: application/json' \
  -d '{"intent_type":"question","entity_a":"almonds","locale":"en"}')
echo "  Raw response length: ${#BATCH} bytes"
echo "$BATCH" | python3 -c "
import sys, json
data = json.load(sys.stdin)
results = data.get('results', [])
for r in results:
    p = r.get('page', {})
    if p:
        slug = p.get('slug', '?')
        title = p.get('title', '?')
        has_are = 'are' in slug.lower() or 'are' in title.lower()
        print(f\"  slug={slug}\")
        print(f\"  title={title}\")
        print(f\"  grammar_ok={'✅' if has_are else '⚠️  (no are)'}\")
        print()
print(f'Generated: {len(results)} pages')
"

echo ""
echo "=== 7. List new pages ==="
NEW_PAGES=$(curl -s "$API/admin/intent-pages" \
  -H "Authorization: Bearer $TOKEN")
echo "$NEW_PAGES" | python3 -c "
import sys, json
data = json.load(sys.stdin)
pages = data.get('pages', data) if isinstance(data, dict) else data
for p in (pages if isinstance(pages, list) else []):
    print(f\"  {p['id'][:8]}  status={p.get('status','?'):10s}  priority={p.get('priority','?')}  slug={p.get('slug','?')}\")
"

echo ""
echo "=== 8. Enqueue bulk with priority=2 (high) ==="
NEW_IDS=$(echo "$NEW_PAGES" | python3 -c "
import sys, json
data = json.load(sys.stdin)
pages = data.get('pages', data) if isinstance(data, dict) else data
ids = [p['id'] for p in (pages if isinstance(pages, list) else [])]
print(json.dumps(ids))
")
echo "  IDs: $NEW_IDS"
ENQUEUE_RESULT=$(curl -s -X POST "$API/admin/intent-pages/enqueue-bulk" \
  -H "Authorization: Bearer $TOKEN" \
  -H 'Content-Type: application/json' \
  -d "{\"ids\": $NEW_IDS, \"priority\": 2}")
echo "  Result: $ENQUEUE_RESULT"

echo ""
echo "=== 9. Stats after enqueue ==="
curl -s "$API/admin/intent-pages/stats" \
  -H "Authorization: Bearer $TOKEN" | python3 -m json.tool

echo ""
echo "=== 10. Run scheduler ==="
SCHED=$(curl -s -X POST "$API/admin/intent-pages/scheduler/run" \
  -H "Authorization: Bearer $TOKEN")
echo "$SCHED" | python3 -m json.tool

echo ""
echo "=== 11. Final stats ==="
curl -s "$API/admin/intent-pages/stats" \
  -H "Authorization: Bearer $TOKEN" | python3 -m json.tool

echo ""
echo "=== 12. Final page statuses ==="
curl -s "$API/admin/intent-pages" \
  -H "Authorization: Bearer $TOKEN" | python3 -c "
import sys, json
data = json.load(sys.stdin)
pages = data.get('pages', data) if isinstance(data, dict) else data
for p in (pages if isinstance(pages, list) else []):
    print(f\"  {p['id'][:8]}  status={p.get('status','?'):10s}  priority={p.get('priority','?')}  slug={p.get('slug','?')}\")
"

echo ""
echo "=== ALL TESTS COMPLETE ==="
