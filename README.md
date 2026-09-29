# beauty-backend

Backend of the **BeautyApp** platform: online booking, calendar, clients, deposits
and reminders for beauty professionals (lash artists first). One API for iOS,
Android (later) and the public booking page.

Product requirements: [`docs/PRODUCT_SPEC.md`](docs/PRODUCT_SPEC.md) ·
Architecture and plan: [`docs/BACKEND_AUDIT_AND_PLAN.md`](docs/BACKEND_AUDIT_AND_PLAN.md) ·
How this repo was cleaned from the legacy `assistant` backend:
[`docs/BACKEND_CLEANUP_PLAN.md`](docs/BACKEND_CLEANUP_PLAN.md).

## Stack

Rust (pinned in `rust-toolchain.toml`) · Axum 0.7 · sqlx 0.8 · PostgreSQL 16 ·
tracing. Layers: `shared` → `infrastructure` → `interfaces/http`
(`domain` and `application` are added with the first business modules).

## Run locally

```bash
cp .env.example .env          # set JWT_SECRET: openssl rand -base64 64
make db-up                    # PostgreSQL 16 in Docker (roles from docker/initdb)
make migrate                  # apply migrations as the schema owner
make run                      # http://localhost:8000/health  and  /ready
make check                    # fmt + clippy -D warnings + all tests (needs db-up)
```

## Tenant isolation (how data of different businesses is kept apart)

- The app connects as a limited login role that is a member of `beauty_app`;
  it is never the table owner or a superuser.
- Each request runs in a transaction scoped with `DbScope`
  (`src/infrastructure/db.rs`), which sets `app.user_id` and `app.business_id`.
- PostgreSQL row-level security policies compare `business_id` with that
  setting. No scope means no rows. `tests/tenant_isolation.rs` proves it.
- Role rules (who may change roles, etc.) live in the application code; RLS is
  the second line of defence.

## Status

Done: skeleton (`/health`, `/ready`, strict CORS, config with production-safe
secret checks, JWT, argon2, cache) and migration `identity_and_tenancy`
(users, auth identities, devices, rotating refresh tokens, business,
membership, staff, audit log, consents, RLS). The SQL and every RLS rule were
checked against a real PostgreSQL 16; Rust code compiles and its tests pass
(skeleton stage) — re-run `make test` after pulling.
Not done: Docker image build and CI have never been run. Next: auth flows
(OTP, Apple/Google, refresh rotation), then catalog, schedule, availability and
booking (see the plan, stages 3+).

## Rules

- No secrets in git; `.env` is ignored. Staging/production refuse weak secrets.
- Every business table carries `business_id` and has RLS.
- Migrations are applied by a deploy step (`migrate`), never at process start.
- After the first successful build, commit `Cargo.lock`.
