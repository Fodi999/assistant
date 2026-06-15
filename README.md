# Restaurant Backend - Guided Assistant

Production-ready Rust backend with DDD architecture, multi-tenancy, JWT authentication, and intelligent guided assistant for restaurant management.

## 🎯 Project Overview

This is a production-grade monolithic backend built with Rust, featuring:
- **Domain-Driven Design (DDD)** - Clean architecture with 4 layers
- **Multi-tenancy** - Complete tenant isolation with user_id + tenant_id
- **JWT Authentication** - Secure auth with access (15min) + refresh (30 days) tokens
- **Guided Assistant** - Smart state machine with 6 steps for inventory/recipe/dish management
- **Internationalization** - Full i18n support for EN, PL, UK, RU languages
- **Product Catalog** - 100 ingredients with categories, allergens, seasons, multilingual search

## 🚀 Tech Stack

- **Rust 1.75+** with Axum 0.7 web framework
- **PostgreSQL** (Neon) with TIMESTAMPTZ, ENUM types, pg_trgm extension
- **sqlx 0.7** with runtime queries (Neon pooler compatible)
- **JWT** authentication with argon2 password hashing
- **async/await** with tokio runtime

## 📁 Project Structure

```
.
├── docs/                      # Documentation
│   ├── backend/               # Backend-specific architecture & guides
│   └── frontend/              # Frontend-specific integration & UI guides
├── src/                       # Backend Source Code (Rust)
│   ├── domain/                # Core business logic
│   ├── application/           # Use cases & services
│   ├── infrastructure/        # Repositories & external services
│   └── interfaces/            # HTTP API
├── migrations/                # SQL migrations
└── tests/                     # Integration tests
```

## 🔧 Getting Started

### Environment Setup
```bash
export DATABASE_URL="postgresql://user:pass@host/db"
export JWT_SECRET="your-secret-key-min-32-chars"
export SERVER_HOST="0.0.0.0"
export SERVER_PORT="8080"
```

### Run
```bash
cargo sqlx migrate run    # Apply migrations
cargo run                 # Start server
```

### Local Admin Tool

Heavy admin jobs can run locally without sending long-running work through the
Koyeb web process:

```bash
cargo run --bin admin_tool -- help
cargo run --bin admin_tool -- state-audit
cargo run --bin admin_tool -- data-quality
cargo run --bin admin_tool -- generate-states-all
cargo run --bin admin_tool -- autofill-product <product_id>
cargo run --bin admin_tool -- generate-seo <product_id>
cargo run --bin admin_tool -- generate-pairings <product_id>
cargo run --bin admin_tool -- create-product-draft "black garlic"
cargo run --bin admin_tool -- run-intent-scheduler
```

Shortcuts:

```bash
make admin-help
make admin-state-audit
make admin-data-quality
make admin-generate-states-all
make admin-run-intent-scheduler
```

Required locally: `DATABASE_URL`. AI commands also need `GEMINI_API_KEY`.
Image upload commands also need the Cloudflare R2 env vars.

### Lightweight Koyeb Mode

Production can keep the web process small by disabling heavy admin HTTP routes:

```bash
ENABLE_HEAVY_ADMIN_ROUTES=false
ENABLE_INTENT_PAGES_SCHEDULER=false
```

This keeps public reads, auth, user APIs, admin CRUD, and upload-url flows online.
AI generation, bulk catalog jobs, analytics/search-console routes, pSEO generation,
and the intent-pages scheduler should be run locally through `admin_tool`.

## 📡 API Endpoints

### Auth
- `POST /api/auth/register` - Register restaurant + owner
- `POST /api/auth/login` - Login
- `POST /api/auth/refresh` - Refresh token
- `GET /api/me` - Current user

### Assistant
- `GET /api/assistant/state` - Get state (localized)
- `POST /api/assistant/command` - Execute command

## 🗄️ Database

**Tables:**
- tenants, users, refresh_tokens
- assistant_states
- catalog_categories (15 categories)
- catalog_ingredients (100 products)

**Features:**
- PostgreSQL ENUMs (unit, allergen, season)
- pg_trgm + GIN indexes for search
- Multilingual names (pl/en/uk/ru)

## �� Languages

EN | PL | UK | RU - Full i18n for messages, actions, hints

## 🔐 Security

- Argon2id password hashing
- JWT (HS256) with 15min access / 30d refresh
- Tenant isolation in all queries

## 📝 License

MIT

## 🔗 Repository

https://github.com/Fodi999/assistant
