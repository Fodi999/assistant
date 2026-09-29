#!/usr/bin/env bash
# Smoke test of the auth, business, catalog, schedule, availability and booking API against a running server.
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
  [ -n "${IDEM:-}" ] && args+=(-H "idempotency-key: $IDEM")
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
VAR0=$(printf '%s' "$BODY" | get variants.0.id)
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

# --- availability (server-side slots) ---
# Next Monday at least 3 days ahead (the smoke owner works Monday 09-13 and 14-18).
MONDAY=$(python3 -c 'import datetime as d; t=d.date.today()+d.timedelta(days=3)
while t.weekday()!=0: t+=d.timedelta(days=1)
print(t)')
AV="$B/availability?service_id=$SVC&variant_id=$VAR0&from=$MONDAY"
req GET "$AV" "$ACCESS";                                    expect "availability for next Monday" 200
[ "$(printf '%s' "$BODY" | python3 -c 'import sys,json;print(len(json.load(sys.stdin)["slots"]))')" = "18" ] || { echo "  FAIL  expected 18 slots (2h service, 15 min grid)"; FAILED=$((FAILED + 1)); }
[ "$(printf '%s' "$BODY" | get slots.0.date)" = "$MONDAY" ] || { echo "  FAIL  slot date"; FAILED=$((FAILED + 1)); }
printf '%s' "$BODY" | get slots.0.start_at | grep -q 'Z$' || { echo "  FAIL  slot start_at is not UTC"; FAILED=$((FAILED + 1)); }
req GET "$AV&to=2099-01-01" "$ACCESS";                      expect "range over 14 days -> 400" 400
req GET "$AV&staff_id=00000000-0000-0000-0000-000000000000" "$ACCESS"
                                                            expect "availability unknown staff -> 404" 404
req GET "$AV&staff_id=$STAFF" "$ACCESS";                    expect "availability for one master" 200
req GET "$AV" "";                                           expect "availability without token -> 401" 401

# --- holds (temporary slot reservation) ---
req GET "$AV" "$ACCESS";                                    expect "availability before holds" 200
SLOT_A=$(printf '%s' "$BODY" | get slots.0.start_at)
SLOT_B=$(printf '%s' "$BODY" | get slots.9.start_at)
SLOT_C=$(printf '%s' "$BODY" | get slots.17.start_at)
hold_body() { printf '{"service_id":"%s","variant_id":"%s","staff_id":"%s","start_at":"%s"}' "$SVC" "$VAR0" "$STAFF" "$1"; }
KEY="smoke-hold-$(date +%s)"
IDEM="$KEY-1" req POST "$B/holds" "$ACCESS" "$(hold_body "$SLOT_A")"
                                                            expect "hold a slot" 201
HOLD1=$(printf '%s' "$BODY" | get id)
[ "$(printf '%s' "$BODY" | get status)" = "held" ] || { echo "  FAIL  hold status"; FAILED=$((FAILED + 1)); }
IDEM="$KEY-1" req POST "$B/holds" "$ACCESS" "$(hold_body "$SLOT_A")"
                                                            expect "same Idempotency-Key -> same hold (200)" 200
[ "$(printf '%s' "$BODY" | get id)" = "$HOLD1" ] || { echo "  FAIL  replay returned another hold"; FAILED=$((FAILED + 1)); }
IDEM="$KEY-1" req POST "$B/holds" "$ACCESS" "$(hold_body "$SLOT_B")"
                                                            expect "same key, other request -> 409" 409
req POST "$B/holds" "$ACCESS" "$(hold_body "$SLOT_B")";     expect "hold without Idempotency-Key -> 400" 400
IDEM="$KEY-2" req POST "$B/holds" "$ACCESS" "$(hold_body "$SLOT_A")"
                                                            expect "held slot again -> 409" 409
printf '%s' "$BODY" | grep -q SLOT_UNAVAILABLE || { echo "  FAIL  error code is not SLOT_UNAVAILABLE"; FAILED=$((FAILED + 1)); }
req GET "$B/holds/$HOLD1" "$ACCESS";                        expect "read hold" 200
req GET "$AV" "$ACCESS";                                    expect "availability with a hold" 200
printf '%s' "$BODY" | grep -q "$SLOT_A" && { echo "  FAIL  held slot is still offered"; FAILED=$((FAILED + 1)); }
req DELETE "$B/holds/$HOLD1" "$ACCESS";                     expect "release hold" 204
req GET "$AV" "$ACCESS";                                    expect "availability after release" 200
printf '%s' "$BODY" | grep -q "$SLOT_A" || { echo "  FAIL  released slot is not offered again"; FAILED=$((FAILED + 1)); }
IDEM="$KEY-3" req POST "$B/holds" "$ACCESS" "$(hold_body "$SLOT_A")"
                                                            expect "hold the freed slot again" 201
HOLD_KEEP=$(printf '%s' "$BODY" | get id)

# --- roles and tenant isolation ---
TS=$(date +%s)
signup() { # signup email -> prints access token
  req POST /v1/auth/register "" "{\"email\":\"$1\",\"password\":\"$PASS\",\"accepted_terms\":true,\"display_name\":\"$2\",\"device\":{\"platform\":\"ios\"}}"
  printf '%s' "$BODY" | get tokens.access_token
}
MGR_MAIL="smoke-mgr-$TS@example.com"; EMP_MAIL="smoke-emp-$TS@example.com"
REC_MAIL="smoke-rec-$TS@example.com"; EMP2_MAIL="smoke-emp2-$TS@example.com"; STR_MAIL="smoke-str-$TS@example.com"
MGR=$(signup "$MGR_MAIL" Mgr); EMP=$(signup "$EMP_MAIL" Emp); REC=$(signup "$REC_MAIL" Rec)
EMP2=$(signup "$EMP2_MAIL" Emp2); STR=$(signup "$STR_MAIL" Stranger)

req POST "$B/members" "$ACCESS" "{\"email\":\"$MGR_MAIL\",\"role\":\"manager\"}"
                                                            expect "owner adds manager" 201
req POST "$B/members" "$ACCESS" "{\"email\":\"$EMP_MAIL\",\"role\":\"employee\"}"
                                                            expect "owner adds employee" 201
EMP_STAFF=$(printf '%s' "$BODY" | get staff_id)
req POST "$B/members" "$ACCESS" "{\"email\":\"$EMP2_MAIL\",\"role\":\"employee\"}"
                                                            expect "owner adds second employee" 201
EMP2_STAFF=$(printf '%s' "$BODY" | get staff_id)
req POST "$B/members" "$ACCESS" "{\"email\":\"$REC_MAIL\",\"role\":\"reception\"}"
                                                            expect "owner adds reception" 201
REC_STAFF=$(printf '%s' "$BODY" | get staff_id)
req POST "$B/members" "$MGR" "{\"email\":\"$STR_MAIL\",\"role\":\"manager\"}"
                                                            expect "manager cannot add manager -> 403" 403
req POST "$B/members" "$EMP" "{\"email\":\"$STR_MAIL\",\"role\":\"employee\"}"
                                                            expect "employee cannot add members -> 403" 403

HOURS='{"intervals":[{"weekday":2,"start":"10:00","end":"16:00"}]}'
req PUT "$B/staff/$EMP_STAFF/schedule/weekly" "$EMP" "$HOURS"
                                                            expect "employee edits OWN schedule" 200
req PUT "$B/staff/$STAFF/schedule/weekly" "$EMP" "$HOURS"
                                                            expect "employee edits owner's schedule -> 403" 403
req PUT "$B/staff/$EMP2_STAFF/schedule/weekly" "$EMP" "$HOURS"
                                                            expect "employee edits colleague's schedule -> 403" 403
req GET "$B/staff/$EMP2_STAFF/schedule" "$EMP";             expect "employee reads colleague's schedule -> 403" 403
req POST "$B/staff/$EMP_STAFF/time-off" "$EMP" '{"start_at":"2020-07-01T00:00:00Z","end_at":"2020-07-02T00:00:00Z","kind":"sick"}'
                                                            expect "employee cannot block the past -> 400" 400
req POST "$B/categories" "$EMP" '{"name":{"pl":"X"}}';      expect "employee cannot write catalog -> 403" 403
req GET "$B/services" "$EMP";                               expect "employee reads catalog" 200

req PUT "$B/staff/$EMP2_STAFF/schedule/weekly" "$MGR" "$HOURS"
                                                            expect "manager edits any schedule" 200
req POST "$B/categories" "$MGR" '{"name":{"pl":"Brwi","en":"Brows"}}'
                                                            expect "manager writes catalog" 201

req GET "$B/staff/$REC_STAFF/schedule" "$REC";              expect "reception reads schedule -> 403" 403
req GET "$B/staff/$STAFF/schedule" "$REC";                  expect "reception reads master schedule -> 403" 403
req PUT "$B/staff/$STAFF/schedule/weekly" "$REC" "$HOURS";  expect "reception edits schedule -> 403" 403
req GET "$B/services" "$REC";                               expect "reception reads catalog" 200
req GET "$AV" "$REC";                                       expect "reception reads availability" 200
req POST "$B/services" "$REC" '{"name":{"pl":"X"}}';        expect "reception cannot write catalog -> 403" 403

# business B belongs to the stranger
req POST /v1/businesses "$STR" '{"name":"Stranger Studio"}'; expect "stranger creates own business" 201
SB=$(printf '%s' "$BODY" | get id)
req GET "$B/services" "$STR";                               expect "B reads A catalog -> 404" 404
req POST "$B/services" "$STR" '{"name":{"pl":"Evil"}}';     expect "B writes A catalog -> 404" 404
req GET "$B/staff" "$STR";                                  expect "B reads A staff -> 404" 404
req GET "$AV" "$STR";                                       expect "B reads A availability -> 404" 404
req GET "$B/staff/$STAFF/schedule" "$STR";                  expect "B reads A schedule -> 404" 404
req PUT "$B/staff/$STAFF/schedule/weekly" "$STR" "$HOURS";  expect "B edits A schedule -> 404" 404
req POST "$B/members" "$STR" "{\"email\":\"$EMP_MAIL\",\"role\":\"employee\"}"
                                                            expect "B adds members to A -> 404" 404

# A's ids used inside B's URLs
req POST "$B/staff/$STAFF/time-off" "$ACCESS" '{"start_at":"2030-09-01T00:00:00Z","end_at":"2030-09-02T00:00:00Z","kind":"blocked"}'
                                                            expect "A creates time off" 201
TOFF_A=$(printf '%s' "$BODY" | get id)
req GET "/v1/businesses/$SB/services/$SVC" "$STR";          expect "A service id in B url -> 404" 404
req PUT "/v1/businesses/$SB/staff/$STAFF/schedule/weekly" "$STR" "$HOURS"
                                                            expect "A staff id in B url -> 404" 404
req GET "/v1/businesses/$SB/staff/$STAFF/time-off" "$STR";  expect "A staff id time-off in B url -> 404" 404
req DELETE "/v1/businesses/$SB/time-off/$TOFF_A" "$STR";    expect "A time-off id in B url -> 404" 404
req POST "/v1/businesses/$SB/services" "$STR" "{\"name\":{\"pl\":\"Mine\"},\"category_id\":\"$CAT\"}"
                                                            expect "A category on B service -> 400" 400
req POST "/v1/businesses/$SB/services" "$STR" '{"name":{"pl":"Mine"}}'
                                                            expect "B creates own service" 201
MINE=$(printf '%s' "$BODY" | get id)
req PUT "/v1/businesses/$SB/services/$MINE/staff" "$STR" "{\"staff_ids\":[\"$STAFF\"]}"
                                                            expect "A staff on B service -> 400" 400
req PATCH "/v1/businesses/$SB/services/$SVC" "$STR" '{"is_active":false}'
                                                            expect "patch A service via B url -> 404" 404
req DELETE "$B/time-off/$TOFF_A" "$EMP";                    expect "employee deletes owner's time off -> 403" 403
req DELETE "$B/time-off/$TOFF_A" "$ACCESS";                 expect "owner deletes time off" 204

# --- double booking race: two members take the same slot at the same moment ---
race() { # race SLOT LABEL
  local dir; dir=$(mktemp -d)
  local body; body=$(hold_body "$1")
  for who in owner manager; do
    local tok="$ACCESS"; [ "$who" = manager ] && tok="$MGR"
    ( curl -s -o "$dir/$who.body" -w '%{http_code}' -X POST "$BASE$B/holds" \
        -H "authorization: Bearer $tok" -H "idempotency-key: smoke-race-$2-$who-$TS" \
        -H 'content-type: application/json' -d "$body" > "$dir/$who.code" ) &
  done
  wait
  local codes; codes=$(printf '%s\n%s\n' "$(cat "$dir/owner.code")" "$(cat "$dir/manager.code")" | sort | tr '\n' ' ')
  if [ "$codes" = "201 409 " ]; then
    printf '  ok    %-44s %s\n' "race $2: one wins (201), one loses (409)" "$codes"
  else
    printf '  FAIL  %-44s got %s\n        %s %s\n' "race $2: expected 201 + 409" "$codes" "$(cat "$dir/owner.body")" "$(cat "$dir/manager.body")"
    FAILED=$((FAILED + 1))
  fi
  cat "$dir/owner.body" "$dir/manager.body" | grep -q SLOT_UNAVAILABLE || { echo "  FAIL  race $2: loser did not get SLOT_UNAVAILABLE"; FAILED=$((FAILED + 1)); }
  rm -rf "$dir"
}
race "$SLOT_B" 1
race "$SLOT_C" 2
req GET "$AV" "$ACCESS";                                    expect "availability after the races" 200
printf '%s' "$BODY" | grep -q "$SLOT_B" && { echo "  FAIL  raced slot is still offered"; FAILED=$((FAILED + 1)); }

# --- C4/C5: confirmed appointments, cancel, reschedule, history ---
# Two later Mondays, each with four disjoint 2h slots (09:00, 11:00, 14:00, 16:00 local).
MON2=$(python3 -c "import datetime as d; print(d.date.fromisoformat('$MONDAY')+d.timedelta(days=7))")
MON3=$(python3 -c "import datetime as d; print(d.date.fromisoformat('$MONDAY')+d.timedelta(days=14))")
slots_of() { # slots_of DATE -> prints "s0 s8 s9 s17"
  req GET "$B/availability?service_id=$SVC&variant_id=$VAR0&from=$1" "$ACCESS"
  printf '%s' "$BODY" | python3 -c 'import sys,json;s=json.load(sys.stdin)["slots"];print(" ".join(s[i]["start_at"] for i in (0,8,9,17)))'
}
read -r M0 M1 M2 M3 <<< "$(slots_of "$MON2")"
read -r N0 N1 N2 N3 <<< "$(slots_of "$MON3")"
AP="$B/appointments"
appt_body() { # appt_body START [NAME]
  printf '{"service_id":"%s","variant_id":"%s","staff_id":"%s","start_at":"%s","client_name":"%s","client_phone":"+48 600 100 200","source":"app","note":"smoke"}' "$SVC" "$VAR0" "$STAFF" "$1" "${2-Anna Test}"
}
AK="smoke-appt-$TS"
req POST "$AP" "$ACCESS" "$(appt_body "$M0")";              expect "book without Idempotency-Key -> 400" 400
IDEM="$AK-1" req POST "$AP" "$ACCESS" "$(appt_body "$M0" "")"
                                                            expect "empty client name -> 400" 400
IDEM="$AK-1" req POST "$AP" "$ACCESS" "$(appt_body "$M0" | sed 's/+48 600 100 200/abc/')"
                                                            expect "bad phone -> 400" 400
IDEM="$AK-1" req POST "$AP" "$ACCESS" "$(appt_body "$M0")"; expect "book appointment (direct)" 201
APPT1=$(printf '%s' "$BODY" | get id)
[ "$(printf '%s' "$BODY" | get status)" = "confirmed" ] || { echo "  FAIL  status not confirmed"; FAILED=$((FAILED + 1)); }
[ "$(printf '%s' "$BODY" | get client_phone)" = "+48600100200" ] || { echo "  FAIL  phone not normalised"; FAILED=$((FAILED + 1)); }
[ "$(printf '%s' "$BODY" | get price_minor)" = "25000" ] || { echo "  FAIL  price snapshot"; FAILED=$((FAILED + 1)); }
IDEM="$AK-1" req POST "$AP" "$ACCESS" "$(appt_body "$M0")"; expect "same key -> same appointment (200)" 200
[ "$(printf '%s' "$BODY" | get id)" = "$APPT1" ] || { echo "  FAIL  replay returned another appointment"; FAILED=$((FAILED + 1)); }
IDEM="$AK-1" req POST "$AP" "$ACCESS" "$(appt_body "$M0" "Other Person")"
                                                            expect "same key, other data -> 409" 409
IDEM="$AK-2" req POST "$AP" "$ACCESS" "$(appt_body "$M0")"; expect "book a taken slot -> 409" 409
printf '%s' "$BODY" | grep -q SLOT_UNAVAILABLE || { echo "  FAIL  taken slot: wrong error code"; FAILED=$((FAILED + 1)); }
req GET "$B/availability?service_id=$SVC&variant_id=$VAR0&from=$MON2" "$ACCESS"
printf '%s' "$BODY" | grep -q "$M0" && { echo "  FAIL  booked slot is still offered"; FAILED=$((FAILED + 1)); }
req GET "$AP/$APPT1" "$ACCESS";                             expect "read appointment" 200
req GET "$AP/$APPT1/history" "$ACCESS";                     expect "appointment history" 200
printf '%s' "$BODY" | grep -q '"booked"' || { echo "  FAIL  history has no booked event"; FAILED=$((FAILED + 1)); }
req GET "$AP?from=$MON2&to=$MON2" "$ACCESS";                expect "calendar for the day" 200
printf '%s' "$BODY" | grep -q "$APPT1" || { echo "  FAIL  appointment missing in calendar"; FAILED=$((FAILED + 1)); }
req GET "$AP?from=$MON2&to=2099-01-01" "$ACCESS";           expect "calendar range over 31 days -> 400" 400

# hold -> confirm
IDEM="$AK-h" req POST "$B/holds" "$ACCESS" "$(hold_body "$M1")"
                                                            expect "hold a slot for confirming" 201
HOLD2=$(printf '%s' "$BODY" | get id)
req POST "$AP" "$ACCESS" "{\"hold_id\":\"$HOLD2\",\"client_name\":\"Maria Hold\"}"
                                                            expect "confirm the hold" 201
[ "$(printf '%s' "$BODY" | get status)" = "confirmed" ] || { echo "  FAIL  confirmed hold status"; FAILED=$((FAILED + 1)); }
[ "$(printf '%s' "$BODY" | get id)" = "$HOLD2" ] || { echo "  FAIL  confirming created another row"; FAILED=$((FAILED + 1)); }
req POST "$AP" "$ACCESS" "{\"hold_id\":\"$HOLD2\",\"client_name\":\"Maria Hold\"}"
                                                            expect "confirm again, same data (200)" 200
req POST "$AP" "$ACCESS" "{\"hold_id\":\"$HOLD2\",\"client_name\":\"Somebody Else\"}"
                                                            expect "confirm again, other data -> 409" 409

# reschedule
RS() { printf '{"start_at":"%s","reason":"client asked"}' "$1"; }
req POST "$AP/$APPT1/reschedule" "$ACCESS" "$(RS "$M1")";   expect "move onto a taken slot -> 409" 409
printf '%s' "$BODY" | grep -q SLOT_UNAVAILABLE || { echo "  FAIL  move: wrong error code"; FAILED=$((FAILED + 1)); }
req POST "$AP/$APPT1/reschedule" "$ACCESS" "$(RS "$M0")";   expect "move to the same time -> 400" 400
req POST "$AP/$APPT1/reschedule" "$ACCESS" "$(RS "$M2")";   expect "reschedule" 200
[ "$(printf '%s' "$BODY" | get id)" = "$APPT1" ] || { echo "  FAIL  reschedule changed the id"; FAILED=$((FAILED + 1)); }
[ "$(printf '%s' "$BODY" | get start_at)" = "$M2" ] || { echo "  FAIL  reschedule start_at"; FAILED=$((FAILED + 1)); }
req GET "$B/availability?service_id=$SVC&variant_id=$VAR0&from=$MON2" "$ACCESS"
printf '%s' "$BODY" | grep -q "$M0" || { echo "  FAIL  old slot was not freed by reschedule"; FAILED=$((FAILED + 1)); }
req GET "$AP/$APPT1/history" "$ACCESS"
printf '%s' "$BODY" | grep -q '"rescheduled"' || { echo "  FAIL  history has no rescheduled event"; FAILED=$((FAILED + 1)); }

# cancel
req POST "$AP/$HOLD2/cancel" "$ACCESS" '{"reason":"client is ill"}'
                                                            expect "cancel appointment" 200
[ "$(printf '%s' "$BODY" | get status)" = "cancelled" ] || { echo "  FAIL  cancel status"; FAILED=$((FAILED + 1)); }
[ "$(printf '%s' "$BODY" | get late_cancellation)" = "False" ] || { echo "  FAIL  a far-away cancel must not be late"; FAILED=$((FAILED + 1)); }
req POST "$AP/$HOLD2/cancel" "$ACCESS";                     expect "cancel again (idempotent)" 200
req POST "$AP/$HOLD2/reschedule" "$ACCESS" "$(RS "$M3")";   expect "reschedule a cancelled one -> 409" 409
req GET "$B/availability?service_id=$SVC&variant_id=$VAR0&from=$MON2" "$ACCESS"
printf '%s' "$BODY" | grep -q "$M1" || { echo "  FAIL  cancelled slot was not freed"; FAILED=$((FAILED + 1)); }
IDEM="$AK-3" req POST "$AP" "$ACCESS" "$(appt_body "$M1" "Rebooked")"
                                                            expect "the freed slot can be booked again" 201

# roles and tenants for appointments
req POST "$AP/$APPT1/cancel" "$EMP";                        expect "employee cancels owner's appointment -> 403" 403
req GET "$AP?from=$MON2&to=$MON2" "$EMP";                   expect "employee lists own calendar" 200
printf '%s' "$BODY" | grep -q "$APPT1" && { echo "  FAIL  employee sees another master's appointment"; FAILED=$((FAILED + 1)); }
IDEM="$AK-r" req POST "$AP" "$REC" "$(appt_body "$N0" "By Reception")"
                                                            expect "reception books any master" 201
REC_APPT=$(printf '%s' "$BODY" | get id)
IDEM="$AK-e" req POST "$AP" "$EMP" "$(appt_body "$N1" "By Employee")"
                                                            expect "employee books another master -> 403" 403
req POST "$AP/$REC_APPT/cancel" "$REC";                     expect "reception cancels" 200
req GET "$AP/$APPT1" "$STR";                                expect "B reads A appointment -> 404" 404
req POST "$AP/$APPT1/cancel" "$STR";                        expect "B cancels A appointment -> 404" 404
req GET "$AP?from=$MON2" "$STR";                            expect "B lists A calendar -> 404" 404
req GET "/v1/businesses/$SB/appointments/$APPT1" "$STR";    expect "A appointment id in B url -> 404" 404
req POST "/v1/businesses/$SB/appointments/$APPT1/reschedule" "$STR" "$(RS "$M3")"
                                                            expect "A appointment moved via B url -> 404" 404
IDEM="$AK-x" req POST "/v1/businesses/$SB/appointments" "$STR" "$(appt_body "$N2")"
case "$STATUS" in 400|404) printf '  ok    %-44s %s\n' "A staff/service booked in B -> 4xx" "$STATUS" ;;
  *) printf '  FAIL  %-44s got %s\n' "A staff/service booked in B" "$STATUS"; FAILED=$((FAILED + 1)) ;; esac

# races on confirmed bookings: one 201/200 wins, the other gets 409 SLOT_UNAVAILABLE
race2() { # race2 LABEL WANT_OK PATH_OWNER BODY_OWNER PATH_MGR BODY_MGR
  local dir; dir=$(mktemp -d)
  ( curl -s -o "$dir/owner.body" -w '%{http_code}' -X POST "$BASE$3" \
      -H "authorization: Bearer $ACCESS" -H "idempotency-key: smoke-r2-$1-owner-$TS" \
      -H 'content-type: application/json' -d "$4" > "$dir/owner.code" ) &
  ( curl -s -o "$dir/mgr.body" -w '%{http_code}' -X POST "$BASE$5" \
      -H "authorization: Bearer $MGR" -H "idempotency-key: smoke-r2-$1-mgr-$TS" \
      -H 'content-type: application/json' -d "$6" > "$dir/mgr.code" ) &
  wait
  local codes; codes=$(printf '%s\n%s\n' "$(cat "$dir/owner.code")" "$(cat "$dir/mgr.code")" | sort | tr '\n' ' ')
  if [ "$codes" = "$2 409 " ]; then
    printf '  ok    %-44s %s\n' "race $1: one wins, one loses (409)" "$codes"
  else
    printf '  FAIL  %-44s got %s\n        %s %s\n' "race $1: expected $2 + 409" "$codes" "$(cat "$dir/owner.body")" "$(cat "$dir/mgr.body")"
    FAILED=$((FAILED + 1))
  fi
  cat "$dir/owner.body" "$dir/mgr.body" | grep -q SLOT_UNAVAILABLE || { echo "  FAIL  race $1: loser did not get SLOT_UNAVAILABLE"; FAILED=$((FAILED + 1)); }
  rm -rf "$dir"
}
race2 book "201" "$AP" "$(appt_body "$N1" "Race Owner")" "$AP" "$(appt_body "$N1" "Race Manager")"
# two different appointments moved onto one slot at the same moment
IDEM="$AK-m1" req POST "$AP" "$ACCESS" "$(appt_body "$N2" "Mover One")"; expect "appointment to move (1)" 201
MV1=$(printf '%s' "$BODY" | get id)
IDEM="$AK-m2" req POST "$AP" "$ACCESS" "$(appt_body "$N0" "Mover Two")"; expect "appointment to move (2)" 201
MV2=$(printf '%s' "$BODY" | get id)
race2 reschedule "200" "$AP/$MV1/reschedule" "$(RS "$N3")" "$AP/$MV2/reschedule" "$(RS "$N3")"
req GET "$AP?from=$MON3&to=$MON3" "$ACCESS";                expect "calendar after the races" 200

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
