# Deploy to Koyeb + Neon

The legacy `assistant` backend already runs on Koyeb (auto-deploy from its own
GitHub repo). beauty-backend is deployed as a **separate Koyeb service** with its
own database, so nothing about the legacy service changes. Shut the legacy service
down only after every site that uses it has been retired or moved.

## 1. Database (Neon, EU region)

1. New Neon project in an EU region (GDPR: data and backups stay in the EU).
   Use separate projects or branches for `staging` and `production`.
2. Take the **direct** (non-pooled) URL of the owner role: this is
   `MIGRATION_DATABASE_URL`, used only to run migrations.
3. Apply the schema from your machine or from CI, never at process start:
   `MIGRATION_DATABASE_URL='postgres://…' cargo run --release --bin migrate`
   (the migration creates the `beauty_app` role).
4. Create the application login and make it a member of `beauty_app`
   (Neon SQL editor, as the owner):
   ```sql
   CREATE ROLE beauty_api LOGIN PASSWORD '<strong password>' IN ROLE beauty_app;
   ```
5. **Verify RLS applies to it** (must return `f`; if it returns `t`, RLS is bypassed
   and the role must not be used):
   ```sql
   SELECT rolbypassrls FROM pg_roles WHERE rolname = 'beauty_api';
   ```
6. `DATABASE_URL` for the app = the **pooled** Neon URL with `beauty_api`.
   Per-request scope uses transaction-local settings, so it is safe behind the pooler.

## 2. Koyeb service

| Setting | Value |
|---|---|
| Source | the new GitHub repo (Dockerfile builder) or a container image |
| Region | an EU region (Frankfurt/Paris; check the console list) |
| Port | 8000, HTTP |
| Health check | `GET /health` (liveness) |
| Instances | 1 for staging, 2 for production |
| Auto-deploy | on for the new repo only |

Environment (secrets go into Koyeb Secrets, never into git):

```
APP_ENV=production            # weak secrets are refused
LOG_FORMAT=json
DATABASE_URL=<secret>         # pooled URL, role beauty_api
JWT_SECRET=<secret>           # openssl rand -base64 64
JWT_ISSUER=beauty-backend
JWT_AUDIENCE=beauty-app
CORS_ALLOWED_ORIGINS=https://<booking-page-domain>
```

## 3. Release routine

1. CI is green (fmt, clippy, tests including `tenant_isolation`).
2. If the release has a new migration: run `migrate` against the target database first.
   Migrations must stay backwards compatible with the previous app version, because
   old instances keep running while the new ones start.
3. Deploy the service; watch `/ready` (database) after the rollout.

## 4. Not decided yet

Custom domain for the API, staging/production naming, alerting, backup retention
(Neon point-in-time restore window), and where CI stores `MIGRATION_DATABASE_URL`.
