# beauty-backend

Backend of the **BeautyApp** platform: online booking, calendar, clients, deposits
and reminders for beauty professionals (lash artists first). One API for iOS,
Android (later) and the public booking page.

Product requirements: [`docs/PRODUCT_SPEC.md`](docs/PRODUCT_SPEC.md) ·
Architecture and plan: [`docs/BACKEND_AUDIT_AND_PLAN.md`](docs/BACKEND_AUDIT_AND_PLAN.md) ·
How this repo was cleaned from the legacy `assistant` backend:
[`docs/BACKEND_CLEANUP_PLAN.md`](docs/BACKEND_CLEANUP_PLAN.md).

## Stack

Rust (pinned in `rust-toolchain.toml`) · Axum 0.7 · sqlx 0.7 · PostgreSQL 16 ·
tracing. Layers: `shared` → `infrastructure` → `interfaces/http`
(`domain` and `application` are added with the first business modules).

## Run locally

```bash
cp .env.example .env          # set JWT_SECRET: openssl rand -base64 64
make db-up                    # PostgreSQL in Docker
make run                      # http://localhost:8000/health  and  /ready
make check                    # fmt + clippy -D warnings + tests
```

## Status

Skeleton only: `/health` (liveness), `/ready` (database), strict CORS,
config with production-safe secret checks, JWT service, argon2, cache.
**Not compiled or run yet** — the code was only syntax-checked with `rustfmt`.
First step on a machine with crates.io access: `cargo check && cargo test`,
then commit `Cargo.lock`. Business modules, schema, RLS and migrations come
next (see the plan, stages 2+).

## Rules

- No secrets in git; `.env` is ignored. Staging/production refuse weak secrets.
- Tenant isolation: every business table carries `business_id` and gets
  row-level security (planned, stage 2).
- Migrations are applied by a deploy step, never at process start.
