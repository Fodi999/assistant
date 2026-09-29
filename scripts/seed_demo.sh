#!/usr/bin/env bash
# Creates ONE demo studio through the real API so the app has something to show:
# master account -> business -> profile -> services -> schedule -> admin approval.
# Usage: ADMIN_EMAIL=... ADMIN_PASSWORD=... scripts/seed_demo.sh [base_url]
# Safe to re-run (each run adds another demo studio). Never prints tokens.
set -eu

BASE="${1:-https://ministerial-yetta-fodi999-c58d8823.koyeb.app}"
: "${ADMIN_EMAIL:?set ADMIN_EMAIL}" "${ADMIN_PASSWORD:?set ADMIN_PASSWORD}"
STAMP="$(date +%s)"
EMAIL="demo-master-$STAMP@example.com"
PASS="correct horse battery"

get() { python3 -c '
import sys, json
d = json.load(sys.stdin)
for k in sys.argv[1].split("."):
    d = d[int(k)] if k.isdigit() else d[k]
print(d)' "$1"; }

call() { # call METHOD PATH TOKEN [JSON]  -> body on stdout, fails on non-2xx
  local method="$1" path="$2" token="${3:-}" data="${4:-}" out code
  local args=(-s -w '\n%{http_code}' -X "$method" "$BASE$path")
  [ -n "$token" ] && args+=(-H "authorization: Bearer $token")
  [ -n "$data" ] && args+=(-H 'content-type: application/json' -d "$data")
  out=$(curl "${args[@]}")
  code=$(printf '%s' "$out" | tail -n1)
  case "$code" in 2*) printf '%s' "$out" | sed '$d' ;; *) echo "FAILED $method $path -> $code: $(printf '%s' "$out" | sed '$d')" >&2; exit 1 ;; esac
}

echo "Seeding demo studio on $BASE"
TOK=$(call POST /v1/auth/register "" "{\"email\":\"$EMAIL\",\"password\":\"$PASS\",\"accepted_terms\":true,\"display_name\":\"Anna Demo\",\"device\":{\"platform\":\"ios\"}}" | get tokens.access_token)
BIZ=$(call POST /v1/businesses "$TOK" '{"name":"Anna Lash Studio"}' | get id)
B="/v1/businesses/$BIZ"
call PUT "$B/profile" "$TOK" '{"city":"Warsaw","headline":"Classic, volume and hybrid lash extensions","about":"Demo studio created for the BeautyApp customer flow.","instagram":"annalash.demo","is_published":true}' >/dev/null
STAFF=$(call GET "$B/staff" "$TOK" | get 0.id)
CAT=$(call POST "$B/categories" "$TOK" '{"name":{"pl":"Rzęsy","en":"Lashes","ru":"Ресницы","uk":"Вії"},"sort_order":1}' | get id)
S1=$(call POST "$B/services" "$TOK" "{\"category_id\":\"$CAT\",\"name\":{\"pl\":\"Klasyczne 1:1\",\"en\":\"Classic 1:1\",\"ru\":\"Классика 1:1\",\"uk\":\"Класика 1:1\"},\"variants\":[{\"duration_min\":120,\"price_minor\":25000},{\"duration_min\":150,\"price_minor\":32000,\"price_type\":\"from\"}]}" | get id)
S2=$(call POST "$B/services" "$TOK" "{\"category_id\":\"$CAT\",\"name\":{\"pl\":\"Objętość 2D-3D\",\"en\":\"Volume 2D-3D\",\"ru\":\"Объём 2D-3D\",\"uk\":\"Об'єм 2D-3D\"},\"variants\":[{\"duration_min\":150,\"price_minor\":30000}]}" | get id)
for S in "$S1" "$S2"; do
  call PUT "$B/services/$S/staff" "$TOK" "{\"staff_ids\":[\"$STAFF\"]}" >/dev/null
done
HOURS='{"intervals":[{"weekday":0,"start":"09:00","end":"18:00"},{"weekday":1,"start":"09:00","end":"18:00"},{"weekday":2,"start":"09:00","end":"18:00"},{"weekday":3,"start":"09:00","end":"18:00"},{"weekday":4,"start":"09:00","end":"16:00"}]}'
call PUT "$B/staff/$STAFF/schedule/weekly" "$TOK" "$HOURS" >/dev/null

ADM=$(call POST /v1/auth/login "" "{\"email\":\"$ADMIN_EMAIL\",\"password\":\"$ADMIN_PASSWORD\"}" | get tokens.access_token)
call POST "/v1/admin/businesses/$BIZ/approve" "$ADM" >/dev/null

echo "Done. Public page: $BASE/v1/public/businesses/$(call GET "/v1/public/businesses/$BIZ" "" | get slug)"
echo "Demo master login: $EMAIL  (password in this script)"
