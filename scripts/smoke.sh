#!/usr/bin/env bash
# Smoke test of the auth, business, catalog and schedule API against a running server.
# Usage: scripts/smoke.sh [base_url]
# Creates one throw-away account and business (prints the email at the end).
# Never prints tokens.
set -u

BASE="${1:-https://ministerial-yetta-fodi999-c58d8823.koyeb.app}"
EMAIL="smoke-$(date +%s)@example.com"
PASS="correct horse battery"
FAILED=0

# get 'a.b.0.c' < json  -> prints the value at that path
get() {
  python3 -c '
import sys, json
d = json.load(sys.stdin)
for k in sys.argv[1].split("."):
    d = d[int(k)] if k.isdigit() else d[k]
print(d)' "$1"
}

STATUS=""
BODY=""
# req METHOD PATH [TOKEN] [JSON_BODY]
req() {
  local method="$1" path="$2" token="${3:-}" data="${4:-}" out
  local args=(-s -w '\n%{http_code}' -X "$method" "$BASE$path")
  [ -n "$token" ] && args+=(-H "authorization: Bearer $token")
  [ -n "$data" ] && args+=(-H 'content-type: application/json' -d "$data")
  out=$(curl "${args[@]}")
  STATUS=$(printf '%s' "$out" | tail -n1)
  BODY=$(printf '%s' "$out" | sed '$d')
}

expect() { # expect "step name" wanted_status
  if [ "$STATUS" = "$2" ]; then
    printf '  ok    %-44s %s\n' "$1" "$STATUS"
  else
    printf '  FAIL  %-44s got %s, wanted %s\n        %s\n' "$1" "$STATUS" "$2" "$BODY"
    FAILED=$((FAILED + 1))
  fi
}

echo "Smoke test: $BASE"

req GET /ready;                                             expect "ready" 200

REG="{\"email\":\"$EMAIL\",\"password\":\"$PASS\",\"accepted_terms\":true,\"display_name\":\"Smoke\",\"device\":{\"platform\":\"ios\"}}"
req POST /v1/auth/register "" "$REG";                       expect "register" 201
ACCESS=$(printf '%s' "$BODY" | get tokens.access_token)
REFRESH=$(printf '%s' "$BODY" | get tokens.refresh_token)

req POST /v1/auth/register "" "$REG";                       expect "register again -> conflict" 409
req POST /v1/auth/login "" "{\"email\":\"$EMAIL\",\"password\":\"wrong password!\"}"
                                                            expect "login wrong password" 401
req POST /v1/auth/login "" "{\"email\":\"$EMAIL\",\"password\":\"$PASS\"}"
                                                            expect "login" 200
req GET /v1/me "$ACCESS";                                   expect "me" 200
req GET /v1/me;                                             expect "me without token" 401

req POST /v1/businesses "$ACCESS" '{"name":"Smoke Lashes"}'; expect "create business" 201
BIZ=$(printf '%s' "$BODY" | get id)
[ "$(printf '%s' "$BODY" | get role)" = "owner" ] || { echo "  FAIL  role is not owner"; FAILED=$((FAILED + 1)); }

req GET /v1/me "$ACCESS";                                   expect "me lists the business" 200
[ "$(printf '%s' "$BODY" | get memberships.0.role)" = "owner" ] || { echo "  FAIL  membership missing"; FAILED=$((FAILED + 1)); }
req GET "/v1/businesses/$BIZ" "$ACCESS";                    expect "get business" 200
req PATCH "/v1/businesses/$BIZ" "$ACCESS" '{"description":"Lash extensions"}'
                                                            expect "update business (owner)" 200
req GET "/v1/businesses/00000000-0000-0000-0000-000000000000" "$ACCESS"
                                                            expect "unknown business -> 404" 404

# --- catalog ---
B="/v1/businesses/$BIZ"
req GET "$B/staff" "$ACCESS";                               expect "list staff" 200
STAFF=$(printf '%s' "$BODY" | get 0.id)
req POST "$B/categories" "$ACCESS" '{"name":{"pl":"Rzęsy","en":"Lashes"},"sort_order":1}'
                                                            expect "create category" 201
CAT=$(printf '%s' "$BODY" | get id)
req POST "$B/services" "$ACCESS" "{\"category_id\":\"$CAT\",\"name\":{\"pl\":\"Klasyczne 1:1\",\"en\":\"Classic 1:1\"},\"variants\":[{\"duration_min\":120,\"price_minor\":25000},{\"duration_min\":150,\"price_minor\":32000,\"price_type\":\"from\"}]}"
                                                            expect "create service with variants" 201
SVC=$(printf '%s' "$BODY" | get id)
[ "$(printf '%s' "$BODY" | get variants.0.price_minor)" = "25000" ] || { echo "  FAIL  price_minor"; FAILED=$((FAILED + 1)); }
[ "$(printf '%s' "$BODY" | get variants.0.currency)" = "PLN" ] || { echo "  FAIL  currency"; FAILED=$((FAILED + 1)); }
req POST "$B/services" "$ACCESS" '{"name":{"pl":"X"},"variants":[{"duration_min":0,"price_minor":100}]}'
                                                            expect "invalid variant -> 400" 400
req PUT "$B/services/$SVC/staff" "$ACCESS" "{\"staff_ids\":[\"$STAFF\"]}"
                                                            expect "assign staff to service" 200
req GET "$B/services" "$ACCESS";                            expect "list services" 200
req GET /v1/businesses/00000000-0000-0000-0000-000000000000/services "$ACCESS"
                                                            expect "catalog of foreign business -> 404" 404

# --- schedule ---
S="$B/staff/$STAFF"
req GET "$S/schedule" "$ACCESS";                            expect "empty schedule" 200
[ "$(printf '%s' "$BODY" | get timezone)" = "Europe/Warsaw" ] || { echo "  FAIL  timezone"; FAILED=$((FAILED + 1)); }
req PUT "$S/schedule/weekly" "$ACCESS" '{"intervals":[{"weekday":0,"start":"09:00","end":"13:00"},{"weekday":0,"start":"14:00","end":"18:00"}]}'
                                                            expect "set weekly (split shift)" 200
req PUT "$S/schedule/weekly" "$ACCESS" '{"intervals":[{"weekday":1,"start":"09:00","end":"13:00"},{"weekday":1,"start":"12:00","end":"16:00"}]}'
                                                            expect "overlapping intervals -> 400" 400
req PUT "$S/schedule/breaks" "$ACCESS" '{"breaks":[{"weekday":0,"start":"13:00","end":"14:00"}]}'
                                                            expect "set breaks" 200
req PUT "$S/schedule/exceptions/2030-12-24" "$ACCESS" '{"kind":"day_off"}'
                                                            expect "day off exception" 200
req POST "$S/time-off" "$ACCESS" '{"start_at":"2030-07-01T09:00:00+02:00","end_at":"2030-07-15T00:00:00Z","kind":"vacation"}'
                                                            expect "create time off" 201
[ "$(printf '%s' "$BODY" | get start_at)" = "2030-07-01T07:00:00Z" ] || { echo "  FAIL  time off not normalised to UTC"; FAILED=$((FAILED + 1)); }
TOFF=$(printf '%s' "$BODY" | get id)
req GET "$S/time-off" "$ACCESS";                            expect "list time off" 200
req DELETE "$B/time-off/$TOFF" "$ACCESS";                   expect "delete time off" 204
req GET "$S/schedule" "$ACCESS";                            expect "schedule readback" 200
[ "$(printf '%s' "$BODY" | get weekly.1.start)" = "14:00" ] || { echo "  FAIL  weekly readback"; FAILED=$((FAILED + 1)); }

req POST /v1/auth/refresh "" "{\"refresh_token\":\"$REFRESH\"}"
                                                            expect "refresh" 200
NEWREFRESH=$(printf '%s' "$BODY" | get refresh_token)
req POST /v1/auth/refresh "" "{\"refresh_token\":\"$REFRESH\"}"
                                                            expect "replay used refresh token -> 401" 401
req POST /v1/auth/refresh "" "{\"refresh_token\":\"$NEWREFRESH\"}"
                                                            expect "reuse revoked the whole login -> 401" 401
req POST /v1/auth/logout "" '{"refresh_token":"unknown"}';  expect "logout (unknown token)" 204

echo
if [ "$FAILED" -eq 0 ]; then echo "All checks passed."; else echo "$FAILED check(s) failed."; fi
echo "Test account: $EMAIL  (delete it in Neon, see docs/AUTH.md)"
exit "$FAILED"
