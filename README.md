# Chef Admin (Tauri)

Local admin desktop app (Tauri + React) for operating the remote backend on Koyeb.

## Purpose

- Accept orders by sending sale events to backend API
- Edit website content via Admin CMS endpoints
- Manage admin catalog and users through existing backend contracts

Backend remains the single source of truth. The desktop app is only an interface.

## Environment

Copy `.env.example` to `.env` and set:

- `VITE_API_BASE_URL` (Koyeb API URL)
- `VITE_ADMIN_STATIC_TOKEN` (admin JWT for local passwordless mode)

In local mode, the app does not show email/password login.
It injects `VITE_ADMIN_STATIC_TOKEN` into local storage as `admin_token` on startup.

Gemini is configured only on the Rust backend with `GEMINI_API_KEY`. Never put
the Gemini key into the admin panel environment. The catalog AI studio calls
`POST /api/admin/catalog/ai/create-product-draft`, lets the admin review the
result, then saves the approved fields through `POST /api/admin/catalog/products`.

## Run

```bash
npm install
npm run tauri dev
```

## Implemented API modules

- `src/api/auth.ts` - login, token verification, logout via `/api/admin/auth/*`
- `src/api/admin.ts` - dashboard statistics and user management via `/api/admin/stats` and `/api/admin/users`
- `src/api/catalog.ts` - protected global catalog via `/api/admin/catalog/*`
- `src/api/cms.ts` - `/api/admin/cms/*`
- `src/api/orders.ts` - `/api/menu-engineering/sales`

## Safety note

Do not move backend runtime logic into this app. Keep domain logic, DB, migrations, and infra on Koyeb backend.
