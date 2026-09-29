# Auth and roles (stage 3, part 1)

Email + password accounts, rotating refresh tokens, businesses with owner membership.
Sign in with Apple / Google and one-time codes come next; they end in the same token pair.

## Endpoints

| Method | Path | Auth | Notes |
|---|---|---|---|
| POST | `/v1/auth/register` | none | `email`, `password` (10-128), `accepted_terms: true`, optional `display_name`, `locale`, `phone` (E.164, see below), `device` |
| POST | `/v1/auth/login` | none | 5 wrong passwords per email lock sign-in for 10 minutes (429) |
| POST | `/v1/auth/refresh` | none | body `{refresh_token}`; the old token is used up |
| POST | `/v1/auth/logout` | none | body `{refresh_token}`; always 204 |
| GET | `/v1/me` | Bearer | user + active memberships |
| POST | `/v1/businesses` | Bearer | caller becomes owner; creates a bookable staff card |
| GET | `/v1/businesses/:id` | member | non-members get 404 |
| PATCH | `/v1/businesses/:id` | owner, manager | employee/reception get 403 |
| PATCH | `/v1/businesses/:id/staff/:staff_id` | owner, manager (any card); others (own card only) | body `{name?, bio?}`; name 1-120, bio ≤1000, empty bio clears; other tenant's card is 404 |

Auth responses: `{ user, tokens: { access_token, refresh_token, token_type, expires_in } }`.

## Phone number

- `phone` on register is optional, so existing clients keep working. It is stored on the **user**
  (`users.phone_e164`), never on the business, and returned as `UserView.phone` (register, login, `/v1/me`).
- Format: international with a leading `+`; spaces, dashes, dots and brackets are stripped; 7-15 digits,
  the first not `0`. Stored as `+48600100200`. Bad format: 400. Already used by another account: 409.
- Verification is not built. `users.phone_verified_at` (migration `20260930090000`) is reserved for a future
  SMS/code step; it can only be set for a row that has a phone. Nothing reads it yet.
- Admin sees the owner's phone in `GET /v1/admin/businesses/:id` (`owner.phone`), together with
  `business_type` and `address_line`.

## Security decisions

- Access token: JWT (HS256, `iss`/`aud`/`jti`), 15 min. It carries only the user id; the role in a
  business is looked up per request, because one person can be a customer in one place and staff in another.
- Refresh token: 64 random hex chars, stored only as SHA-256 (`hash_token`). Each use rotates it.
  Presenting an already-used token revokes the whole login (family), which is the theft response.
  Two truly simultaneous refreshes of the same token also count as reuse (client must not retry in parallel).
- Passwords: argon2id. Unknown emails still pay for one hash (no timing oracle); wrong password and
  unknown email return the same message.
- Pre-login lookups go through two `SECURITY DEFINER` functions (`auth_login_lookup`,
  `auth_refresh_lookup`); everything else runs under row-level security with the user's scope.
- Registration stores the `terms` and `privacy` consents with the accepted text version.
- Business role checks: `BusinessService::access` (membership, active) then `BusinessAccess::require`.

## Known limits (deliberate, tracked)

- Lockout counters live in memory of one instance; move to the database or Redis before scaling out.
  A locked email can be locked by anyone who knows it (10 min), the usual trade-off.
- A disabled user keeps a valid access token for up to 15 minutes; refresh is refused immediately.
- No email verification or password reset yet (needs the email provider decision).
- No invites yet: employees are attached to a business by the next stage.
