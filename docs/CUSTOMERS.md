# Customers, public catalog and moderation (stage C6)

## Model

| Table | Purpose |
|---|---|
| `client` | A person who books with ONE business (tenant RLS on `business_id`). `user_id` links the signed-in customer (registered or guest session), NULL for a client typed in by staff. `source`: `staff`, `guest`, `account`. |
| `business.moderation_status` | `pending` (default), `approved`, `rejected`, `suspended`. Only a platform admin can change it (database trigger, error 42501 for the app role). |
| `business.is_published` + `city`, `headline`, `about`, `instagram` | The public profile, edited by owner/manager. |
| `business_moderation` | Every decision with its note (append-only). The owner reads their own history. |
| `platform_admin` | Users who may moderate. Created only by an operator (`make_admin`), never by the API. |
| `portfolio_item` | Placeholder model (image URL + caption). No upload yet. |
| `appointment.client_id` | The customer an appointment belongs to. |

A customer's identity is the **user session**. Phone and e-mail are contact data only; they are never used to look up who someone is, so typing another person's phone number gives no access to that person's bookings.

A **guest** is a user without e-mail or password (`POST /v1/public/guest`, terms must be accepted). It gets normal access/refresh tokens, so booking, refresh and "my appointments" work unchanged. Turning a guest into a registered account is a later step (same user id, history kept).

## Public visibility

A business is public only when `moderation_status = 'approved'` AND `is_published` AND `status = 'active'`. This is a row policy on `business` (`business_public_select`) and is repeated in every public query. Everything else (pending, rejected, suspended, unpublished) is a 404 for the public and for customers.

## Endpoints

Owner / staff (`/v1/businesses/:id`):
- `GET /profile` (any member), `PUT /profile` (owner, manager). Body fields are optional; an empty string clears a text field. Besides `city`, `headline`, `about`, `instagram` and `is_published`, the body takes `business_type` (one of `lashes`, `brows`, `nails`, `hair`, `beauty_studio`, `other`; else 400) and `address_line` (one line, ≤200, no geocoding). Both come back in the GET body. `moderation_status` in the body is ignored.
- `GET /clients?q=&limit=&offset=`, `GET /clients/:client_id` (owner, manager, reception).

Public, no sign-in (`/v1/public`):
- `POST /guest` - anonymous customer session.
- `GET /businesses?city=&q=&limit=&offset=` - catalog of approved masters.
- `GET /businesses/:key` - profile by id or slug: masters, categories, services, variants, prices, timezone, portfolio. Never buffers, notice rules, intake questions, team contact data or clients.
- `GET /businesses/:key/availability` - same rules and slots as the staff API, always the `online` channel.

Signed-in customer (registered or guest), `/v1/public/businesses/:key`:
- `POST /holds` (Idempotency-Key), `DELETE /holds/:id`
- `POST /appointments` - confirm a hold (`hold_id`) or book directly (Idempotency-Key required). Body: `client_name`, optional `client_phone`, `client_email`, `note`; `source` is `app` (default) or `web`, never `manual`.
- `GET /appointments?status=` - only the caller's own; `GET /appointments/:id`
- `POST /appointments/:id/cancel`, `POST /appointments/:id/reschedule`

Platform admin (`/v1/admin`): `GET /businesses?status=`, `POST /businesses/:id/approve|reject|suspend` (reject and suspend need `{ "note" }`; reject only a pending business, suspend only an approved one; approve reinstates).

## Security rules (all covered by tests and the smoke)

- A customer sees, cancels and moves only their own appointments. A foreign appointment is a 404, never a 403, so its existence is not revealed. Holds are owned by whoever created them.
- A customer is not a business member: the staff API (`/v1/businesses/:id/...`) answers 404.
- Business A never sees B's clients or appointments (RLS on `client`, `appointment`). Ids of A used in B's URLs are 404.
- Customers use the same booking engine as staff, so the availability rules and the PostgreSQL double-booking guard apply: two customers racing for one slot get 201 + 409 `SLOT_UNAVAILABLE`.
- Owners cannot approve themselves: no API for it, and the database trigger refuses it even for a raw connection as the application role.
- Customers can still cancel their own appointments after a business was suspended (the URL then takes the business id).

Known limits (next steps): ownership between customers is enforced in the application layer (one check, `BookingService::ensure_owned`) on top of business-level RLS; guest creation is only limited by the global rate limit; no "all my bookings across businesses" list yet; no notifications; a guest cannot yet be upgraded to an account.

## Operator steps

Create a platform admin (once): register the account through the API, then
```
export MIGRATION_DATABASE_URL='<direct owner URL>'
cargo run --release --bin make_admin -- admin@yourdomain.com
```
Smoke with the customer flow:
```
ADMIN_EMAIL=admin@yourdomain.com ADMIN_PASSWORD='...' bash scripts/smoke.sh
```
Without these variables the C6 block is skipped (printed as SKIP).

Clean up after a smoke run (Neon SQL editor, owner role):
```sql
DELETE FROM business WHERE name IN ('Smoke Lashes','Stranger Studio','Test Lashes');
DELETE FROM users WHERE email LIKE 'smoke-%@example.com';   -- may fail on consent rows; see docs/AUTH.md
```
Guest users of the smoke have no e-mail; remove them with `DELETE FROM users WHERE email IS NULL AND display_name IN ('Smoke Guest') AND created_at < now() - interval '1 hour'` after their consent rows if needed.
