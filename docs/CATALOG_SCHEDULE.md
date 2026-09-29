# Catalog and master schedule

Base: `/v1/businesses/:business_id`. All calls need `Authorization: Bearer <access token>`.
Non-members get 404. Errors: `400` validation, `401` no/invalid token, `403` role, `404` not found.

## Rules

- Money: integer minor units (`price_minor`, 25000 = 250.00) + `currency` (must equal the business currency).
- Names/descriptions: `{ "pl": "...", "en": "...", "ru": "...", "uk": "..." }`, at least one language.
- Catalog writes: owner/manager. Reads: any member. Soft delete (`deleted_at`), `version` grows on every change.
- Schedule times are wall-clock in `business.timezone` (IANA, default `Europe/Warsaw`), format `HH:MM`, no crossing midnight.
  Weekday `0` = Monday ... `6` = Sunday.
- `time_off` is an absolute instant, RFC 3339, always returned in UTC (`Z`).
- Who edits schedules: owner/manager anyone (also the past); employee only own card and only present/future; reception nobody.

## Catalog endpoints

| Method | Path |
|---|---|
| GET, POST | `/categories` |
| PATCH, DELETE | `/categories/:category_id` |
| GET, POST | `/services` (POST accepts nested `variants`) |
| GET, PATCH, DELETE | `/services/:service_id` |
| POST | `/services/:service_id/variants` |
| PUT | `/services/:service_id/staff` (replace who performs the service) |
| PATCH, DELETE | `/variants/:variant_id` |

## Schedule endpoints (`staff/:staff_id`)

| Method | Path | Notes |
|---|---|---|
| GET | `/staff/:staff_id/schedule` | timezone, weekly, breaks, exceptions from today |
| PUT | `/staff/:staff_id/schedule/weekly` | replaces the whole pattern; overlaps -> 400 |
| PUT | `/staff/:staff_id/schedule/breaks` | replaces all breaks; overlaps -> 400 |
| PUT, DELETE | `/staff/:staff_id/schedule/exceptions/:date` | `day_off` or `custom_hours`; `date` = `YYYY-MM-DD` |
| GET, POST | `/staff/:staff_id/time-off` | GET filter: `?from=&to=` (RFC 3339, overlap) |
| DELETE | `/time-off/:time_off_id` | |

## Team (temporary, until invitations)

`POST /members` `{ "email", "role": "manager|reception|employee", "display_name"? }` adds an **existing** account
to the business with a staff card. Owner adds any role, manager only employee/reception; others get 403.
Unknown e-mail -> 404, already a member -> 409.

## Examples

```bash
B=https://<host>/v1/businesses/$BIZ ; H="Authorization: Bearer $TOKEN"

# category
curl -X POST $B/categories -H "$H" -H 'content-type: application/json' \
  -d '{"name":{"pl":"Rzęsy","en":"Lashes"},"sort_order":1}'

# service with two variants
curl -X POST $B/services -H "$H" -H 'content-type: application/json' -d '{
  "category_id":"<uuid>","name":{"pl":"Klasyczne 1:1","en":"Classic 1:1"},"buffer_after_min":10,
  "variants":[{"name":{"pl":"Nowy zestaw"},"duration_min":120,"price_minor":25000},
              {"duration_min":150,"price_minor":32000,"price_type":"from"}]}'
# -> 201 {"id":"...","booking_step_minutes":15,"variants":[{"price_minor":25000,"currency":"PLN",...}],"staff_ids":[]}

# who performs it
curl -X PUT $B/services/$SVC/staff -H "$H" -H 'content-type: application/json' -d '{"staff_ids":["<staff uuid>"]}'

# weekly pattern (split shift on Monday)
curl -X PUT $B/staff/$STAFF/schedule/weekly -H "$H" -H 'content-type: application/json' -d '{"intervals":[
  {"weekday":0,"start":"09:00","end":"13:00"},{"weekday":0,"start":"14:00","end":"18:00"}]}'
# -> 200 [{"id":"...","weekday":0,"start":"09:00","end":"13:00","valid_from":null,"valid_to":null}, ...]

# day off / custom hours
curl -X PUT $B/staff/$STAFF/schedule/exceptions/2030-12-24 -H "$H" -H 'content-type: application/json' \
  -d '{"kind":"custom_hours","start":"10:00","end":"14:00"}'

# vacation (offset input is stored as UTC)
curl -X POST $B/staff/$STAFF/time-off -H "$H" -H 'content-type: application/json' \
  -d '{"start_at":"2030-07-01T09:00:00+02:00","end_at":"2030-07-15T00:00:00Z","kind":"vacation"}'
# -> 201 {"id":"...","staff_id":"...","start_at":"2030-07-01T07:00:00Z","end_at":"2030-07-15T00:00:00Z","kind":"vacation","note":null}

# overlap -> 400
# {"code":"VALIDATION_ERROR","message":"Validation failed","details":"Working intervals of the same weekday must not overlap"}
```

## Deferred (not in this stage)

Per-variant staff overrides, deposit policy, `If-Match`/version enforcement, recurring time off (`rrule`),
schedule-vs-future-appointments conflict check (needs bookings), `24:00` as end time.
