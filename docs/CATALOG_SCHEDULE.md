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

## Availability (server-side slots)

`GET /availability?service_id=&variant_id=&from=YYYY-MM-DD[&to=YYYY-MM-DD][&staff_id=][&channel=online|manual]`
(any member; at most 14 days; dates are local dates of the business time zone).

```json
{ "timezone": "Europe/Warsaw", "service_id": "...", "variant_id": "...", "duration_min": 120, "buffer_after_min": 10,
  "slots": [ { "date": "2030-06-03", "staff_id": "...", "start_at": "2030-06-03T07:00:00Z", "end_at": "2030-06-03T09:00:00Z" } ] }
```

A start time is offered when the whole variant duration fits a working interval on the service slot grid
(minutes from local midnight), and duration + `buffer_after_min` does not touch a break, time off or (from the
booking stage) an appointment/hold, and it lies within `min_notice_min` .. `max_advance_days` from now.
Exceptions: `day_off` gives nothing, `custom_hours` replaces the weekly pattern (weekly breaks do not apply to it).
`channel=online` (default) requires `is_online_bookable`. Clients never compute slots themselves.

## Holds (temporary slot reservation)

| Method | Path | |
|---|---|---|
| POST | `/holds` | needs header `Idempotency-Key` (8-100 chars). 201 new hold, 200 same key + same request |
| GET | `/holds/:id` | `status` is `held`, or `expired` when time is up / released |
| DELETE | `/holds/:id` | release; repeating is harmless (204) |

Body: `{ "service_id", "variant_id", "staff_id", "start_at" (UTC RFC 3339 from availability), "source": "manual|app|web" }`.
A hold lives 10 minutes (`hold_expires_at`), blocks the calendar (service + buffer) exactly like a booking, and one user may keep
at most 10 live holds (429). The start time is re-validated on the server against the very same availability rules.

Errors: `409 SLOT_UNAVAILABLE` (taken, not offered, or lost a race), `409 CONFLICT` (Idempotency-Key reused for another request),
`404` unknown service/variant/staff, `403` employee for another master's calendar, `400` bad key/date/source.

Double booking is prevented by PostgreSQL itself: `EXCLUDE USING gist (staff_id WITH =, tstzrange(start_at, blocked_end) WITH &&)
WHERE status IN ('held','confirmed')`. Two simultaneous requests for one slot: one gets 201, the other 409.

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

## Appointments (confirmed bookings, cancel, reschedule, history)

| Method | Path | |
|---|---|---|
| POST | `/appointments` | 201 created, 200 replay of the same request |
| GET | `/appointments?from&to&staff_id&status` | calendar by business-local dates, at most 31 days, default `status=confirmed`, several allowed comma separated (`confirmed,completed,no_show`); an employee sees only their own calendar |
| GET | `/appointments/:id` | one appointment (also works for a hold id) |
| POST | `/appointments/:id/cancel` | body optional `{ "reason" }`; repeating is harmless (200) |
| POST | `/appointments/:id/reschedule` | `{ "start_at", "staff_id"?, "reason"? }`; same row, same id |
| POST | `/appointments/:id/complete` | mark a visit done: only from `confirmed`, and only after its start (409 otherwise); repeating is harmless (200); an employee only on their own calendar |
| POST | `/appointments/:id/no-show` | same rules; marks the client as not having come |
| GET | `/appointments/:id/history` | append-only events: `hold_created`, `booked` (direct), `confirmed` (hold confirmed), `hold_released`, `hold_expired`, `rescheduled`, `cancelled`, `completed`, `no_show` |

Two ways to book:
1. Confirm a hold: `{ "hold_id", "client_name", "client_phone"?, "note"? }` (Idempotency-Key optional; `hold_id` cannot be combined with service/staff/start/source, 400).
   The hold must be alive; confirming twice with the same data returns 200, with other data 409, an expired hold 409 `SLOT_UNAVAILABLE`.
2. Direct booking: `{ "service_id", "variant_id", "staff_id", "start_at", "client_name", ... }` with header `Idempotency-Key` (required).
   The start time is validated by the same availability rules as slots.

`client_phone` is stored in E.164 (`+48 600 100 200` becomes `+48600100200`). `note` is an internal note (up to 500 characters);
do not put health information there. The appointment keeps a snapshot of service/variant names, duration and price.

Cancellation policy (no payments yet): cancelling less than 24 hours before the start sets `late_cancellation=true`
(the threshold is a code constant, `FREE_CANCELLATION_HOURS`); the slot is freed at once either way. Started appointments
cannot be cancelled or moved (409). Rescheduling keeps the booked duration and re-checks the new time against availability,
ignoring the appointment's own current time; the old slot is freed in the same statement.

Errors: `409 SLOT_UNAVAILABLE` (taken / not offered / lost a race / hold expired), `409 CONFLICT` (key reused with other data,
wrong state), `400 VALIDATION_ERROR` (name, phone, nothing to change), `403` employee for another master, `404` other business.
Two simultaneous bookings or moves onto one slot: one succeeds, the other gets 409.

## Deferred (not in this stage)

Per-variant staff overrides, deposit policy, `If-Match`/version enforcement, recurring time off (`rrule`),
schedule-vs-future-appointments conflict check, client/CRM table, configurable cancellation policy, customer-facing booking API, `24:00` as end time.

## Deleting services and variants, and the booking snapshot

- `DELETE /services/:id` and `DELETE /variants/:id` are soft deletes (`deleted_at`).
  Hiding (`is_active = false`) is the normal way to retire a service; the apps do not offer delete.
- **409** while a live booking uses the item: a `confirmed` appointment, or a `held` one whose
  hold is still running, with `end_at` after now. Cancelled, completed, no-show and past visits
  do not block.
- **409** when deleting the last offered (active) variant of an active service. A hidden
  service may lose all its variants.
- An appointment is a record of what was booked, not a view of the catalog. `AppointmentView`
  returns the snapshot stored in `appointment_item`: `service_name`, `variant_name` (nullable;
  `{"pl": "...", ...}` objects), `duration_min`, `price_minor`, `currency`. They never change
  when the catalog is edited or the service is deleted. `service_id` / `variant_id` remain as
  references only.

## Team management

- `GET /members` (owner, manager): `[{membership_id, staff_id, role, status: active|suspended, display_name, is_bookable, is_me}]`, owner first. No e-mail addresses are returned.
- `PATCH /members/:membership_id` `{ role?: manager|reception|employee, status?: active|suspended }`
  - The owner manages everyone but owners; a manager only employees and reception and cannot make a manager (403).
  - Nobody changes their own membership (409). An owner cannot be changed or removed while they are the last active owner (409).
  - `suspended` revokes access to the business at once and sets the card `is_bookable = false`; `active` restores both. Existing appointments are untouched.
  - Role `owner` and statuses other than `active` / `suspended` are 400.
- `PATCH /staff/:id` also takes `is_bookable` (owner, manager; 403 for a master editing their own card).
- `POST /members` (existing): unknown e-mail 404, already a member (also a suspended one) 409.

## Clients (CRM)

A client card belongs to one business (RLS on `business_id`). `source` says who made it:
`staff` (the business), `guest` or `account` (an app customer, made when they book).

**Who sees what.** Owner, manager and reception read and edit every card. An employee
reads only cards that have one of *their own* appointments, gets 404 for any other, and
never sees the note; the visit numbers and the history they get are limited to their own
appointments too. An employee cannot create or edit cards (403). The one thing that
reaches them automatically is the card of a client they booked by hand (below).

**Manual bookings and phones.** A visit typed in by hand (`source = manual`) with a phone:

1. the phone is normalised to E.164;
2. the business's `staff` card with that phone is used, or made on the spot;
3. the visit gets its `client_id`. The card keeps its name; the visit keeps the name typed for it.

Without a phone nothing is made and `client_id` stays NULL (`POST /clients` makes a card
without a phone, on purpose). The phone is unique **per business and only among `staff`
cards** (`client_staff_phone_key`): an app customer's phone is typed by them and not
verified, so a manual visit never lands on a customer's own card (they would see it in
"my appointments"). Two cards for the same person (a `staff` one and an `account` one) are
possible until the merge step, which is a later stage.

A booking can also name a card: `POST /appointments` with `client_id` (name and phone come
from the card; an employee may use only cards they can see).

**API** (`/v1/businesses/:id`)

| | |
|---|---|
| `GET /clients?q=&sort=name\|recent&limit=&offset=` | search name/phone/e-mail |
| `GET /clients/:client_id` | one card |
| `POST /clients` | `{full_name, phone?, email?, note?}` → 201. A taken phone: 409 `CLIENT_PHONE_EXISTS`, `details` = id of the existing card |
| `PATCH /clients/:client_id` | absent fields stay; `""` clears phone, e-mail, note. Name, phone, e-mail only on `staff` cards (409 otherwise); the note always |
| `GET /clients/:client_id/appointments?limit=&offset=` | visits, newest first, holds left out; `AppointmentView` with the booking snapshot |

`ClientView` numbers: `confirmed_appointments`, `completed_count`, `no_show_count`,
`last_visit_at` (latest completed), `total_spent_minor` (sum of the **booked** prices of
completed visits only) and `total_spent_currency` (the business currency).

`note` is free text (1000 characters) for the business; the app tells people not to put
health information in it. Every create and update writes `audit_log` (`client.create`,
`client.update`) with the names of the changed fields only, never phones or notes.

**Migration `20260930100000_crm`.** Adds `client.note`, makes one `staff` card per
`(business, phone)` for old manual visits that carry a valid phone (name from the latest
visit), links those visits, then creates the unique index. Visits without a phone stay
without a card. It does not bump `appointment.version`. Dry run first:
`scripts/crm_backfill_dry_run.sql`.
