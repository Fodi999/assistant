# Deploy to Koyeb + Neon (reusing the existing production setup)

The legacy `assistant` backend already runs on Koyeb with a Neon database, and
every project that used it is retired (moved to Cloudflare). So beauty-backend
**reuses the same Koyeb service and Neon project**; only the code and the
database contents are new.

## 0. Before switching (one-time)

1. Push the safety net from the old repo: tag `legacy-final` and branch
   `legacy/main` (`git push origin legacy-final legacy/main` in `assistant`).
2. Take a final `pg_dump` of the old Neon database and keep the file outside git.
   The old data is dead, but a dump costs nothing.
3. Write down which environment variables the Koyeb service has now
   (only names, not values); most of them are removed in step 3 below.

## 1. Database (Neon)

- **Do not run the new migrations on the old database.** It already has a
  different `users`, `tenants`, ... schema and 98 tables of old data.
- Create a **new database (or branch) for beauty** inside the existing Neon
  project, or reset the old one after the dump is verified.
- Check the project region (Neon → Settings). For GDPR it must be in the EU
  (Frankfurt). If it is not, create a new Neon project in an EU region instead.
- Take the **direct** (non-pooled) owner URL: this is `MIGRATION_DATABASE_URL`.
  Apply the schema from your machine or CI, never at process start:
  `MIGRATION_DATABASE_URL='postgres://…' cargo run --release --bin migrate`
  (the migration creates the `beauty_app` role).
- Create the application login (Neon SQL editor, as the owner):
  ```sql
  CREATE ROLE beauty_api LOGIN PASSWORD '<strong password>' IN ROLE beauty_app;
  ```
- **Verify RLS applies to it.** This must return `f`; if `t`, RLS is bypassed and
  the role must not be used:
  ```sql
  SELECT rolbypassrls FROM pg_roles WHERE rolname = 'beauty_api';
  ```
- `DATABASE_URL` for the app = the **pooled** URL with `beauty_api`. The
  per-request scope uses transaction-local settings, so it is safe behind the pooler.

## 2. Getting the new code into the existing service

The new code already lives in this repository: local `main` is the new clean
history, the old code is kept as tag `legacy-final` and branch `legacy/main`.
Koyeb auto-deploys `Fodi999/assistant` (`main`, Dockerfile builder, port 8000,
health check `/health`), and all of that already matches the new code.

Run in this order (the second command rewrites remote `main`, so the first
must succeed before it):

```bash
cd ~/Desktop/assistant
git push origin legacy-final legacy/main     # 1. safety net for the old code
git push --force-with-lease origin main      # 2. replace main -> Koyeb rebuilds
```

Until step 3 below is done the new service can still start: it only needs
`DATABASE_URL` and `JWT_SECRET`, which the service already has; `/health` answers,
`/ready` shows the database state. If the build fails, Koyeb keeps the previous
deployment running. Rename the repo to `beauty-backend` afterwards (GitHub keeps
redirects; re-check the Koyeb link).

The old local secrets file was renamed to `.env.legacy` (git-ignored). Do not
point `make migrate` / tests at the old production database.

## 3. Service settings

| Setting | Value |
|---|---|
| Port / health | 8000 / `GET /health` (same as now) |
| Region | an EU region (Frankfurt/Paris), check the current one |
| Instances | 2 for production (currently min 1 / max 1) |

Remove the old variables (`GEMINI_*`, `GROQ_*`, `STRIPE_*`, `TELEGRAM_*`, `GA4_*`,
`GOOGLE_*`, `SEARCH_CONSOLE_*`, `ADMIN_*`, `CLOUDFLARE_R2_*`, `ENABLE_*`, the old
`CORS_ALLOWED_ORIGINS`) and rotate/revoke those secrets at their providers.
Set (secrets in Koyeb Secrets, never in git):

```
APP_ENV=production            # weak secrets are refused
LOG_FORMAT=json
DATABASE_URL=<secret>         # pooled URL, role beauty_api
JWT_SECRET=<secret>           # NEW value: openssl rand -base64 64
JWT_ISSUER=beauty-backend
JWT_AUDIENCE=beauty-app
CORS_ALLOWED_ORIGINS=https://<booking-page-domain>
```

Use a staging copy (a second Koyeb service + a Neon branch) before production
traffic exists.

## 4. Release routine

1. CI is green (fmt, clippy, tests including `tenant_isolation`).
2. New migration? Run `migrate` against the target database first. Migrations
   must stay backwards compatible with the previous app version, because old
   instances keep running while new ones start.
3. Deploy; watch `/ready` (database) after the rollout.

## 5. Not decided yet

Custom API domain, staging naming, alerting, backup retention (Neon
point-in-time restore window), where CI keeps `MIGRATION_DATABASE_URL`.
