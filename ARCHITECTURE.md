# Структура проекта

## 🚀 Production Deployment

**Backend deployed on:** [Koyeb](https://app.koyeb.com)
- **URL:** `https://ministerial-yetta-fodi999-c58d8823.koyeb.app`
- **Database:** Neon PostgreSQL (Serverless)
- **Repository:** [github.com/Fodi999/assistant](https://github.com/Fodi999/assistant)
- **Auto-deploy:** ✅ Push to `main` → automatic redeploy

```
restaurant-backend/
├── Cargo.toml                 # Зависимости проекта (Rust 1.83, Axum 0.7)
├── Cargo.lock                 # Locked dependencies (committed for Docker)
├── Dockerfile                 # Multi-stage production build
├── .dockerignore              # Docker build optimization
├── Makefile                   # Команды для удобства
├── README.md                  # Документация
├── .env.example               # Пример конфигурации
├── .gitignore                 # Git игнорирование
│
├── migrations/                # SQL миграции (sqlx)
│   ├── 20240101000001_initial_schema.sql
│   ├── 20240102000001_assistant_states.sql
│   ├── 20240103000001_add_user_language.sql
│   ├── 20240104000001_catalog_ingredients.sql
│   ├── 20240105000001_catalog_categories.sql
│   ├── 20240106000001_inventory_products.sql
│   ├── 20240107000001_recipes.sql
│   ├── 20240108000001_dishes.sql
│   └── 20240110000001_dish_sales.sql
│
├── .sqlx/                     # SQLx offline query metadata (for Docker builds)
│   └── query-*.json
│
├── examples/                  # Примеры использования API
│   ├── api_examples.sh
│   ├── API_EXAMPLES.md
│   ├── assistant_test.sh
│   ├── inventory_test.sh
│   ├── recipe_test.sh
│   ├── dish_test.sh
│   └── menu_engineering_test.sh
│
├── src/
│   ├── main.rs               # Точка входа (Axum server on port 8000)
│   │
│   ├── domain/               # DOMAIN LAYER - Бизнес-логика
│   │   ├── mod.rs
│   │   ├── tenant.rs         # Tenant aggregate + TenantName value object
│   │   ├── user.rs           # User aggregate + Email, DisplayName, Password value objects
│   │   ├── auth.rs           # RefreshToken entity
│   │   ├── assistant/        # 🤖 AI Assistant wizard (5-step onboarding)
│   │   │   ├── mod.rs
│   │   │   ├── state.rs      # AssistantState (Start→Inventory→Recipes→Dishes→Report)
│   │   │   ├── command.rs    # User commands (AddProduct, CreateDish, etc.)
│   │   │   ├── response.rs   # AssistantResponse with warnings
│   │   │   ├── step.rs       # Wizard steps with progress tracking
│   │   │   └── rules.rs      # State transition rules
│   │   ├── catalog.rs        # Ingredient Catalog (CatalogCategory, CatalogIngredient)
│   │   ├── inventory.rs      # Inventory management (InventoryProduct with expiration)
│   │   ├── recipe.rs         # Recipe domain (Basic/Component recipes)
│   │   ├── dish.rs           # Dish domain (with financial analysis)
│   │   └── menu_engineering.rs # 📊 Menu Engineering (BCG Matrix + ABC Analysis)
│   │
│   ├── application/          # APPLICATION LAYER - Use cases
│   │   ├── mod.rs
│   │   ├── auth.rs           # AuthService: register, login, refresh
│   │   ├── user.rs           # UserService: get_user_with_tenant
│   │   ├── assistant_service.rs # AssistantService: wizard flow + "Момент ВАУ" financials
│   │   ├── catalog.rs        # CatalogService: manage ingredients catalog
│   │   ├── inventory.rs      # InventoryService: add products, check expiration
│   │   ├── recipe.rs         # RecipeService: create recipes, calculate costs
│   │   ├── dish.rs           # DishService: create dishes, calculate profit margins
│   │   └── menu_engineering.rs # MenuEngineeringService: BCG/ABC analysis, sales tracking
│   │
│   ├── infrastructure/       # INFRASTRUCTURE LAYER
│   │   ├── mod.rs
│   │   ├── config.rs         # Конфигурация из env (DATABASE_URL, JWT_SECRET, PORT)
│   │   │
│   │   ├── persistence/      # Репозитории (PostgreSQL + sqlx)
│   │   │   ├── mod.rs
│   │   │   ├── tenant_repository.rs
│   │   │   ├── user_repository.rs
│   │   │   ├── refresh_token_repository.rs
│   │   │   ├── assistant_state_repository.rs
│   │   │   ├── catalog_category_repository.rs
│   │   │   ├── catalog_ingredient_repository.rs
│   │   │   ├── inventory_product_repository.rs
│   │   │   ├── recipe_repository.rs
│   │   │   └── dish_repository.rs
│   │   │
│   │   └── security/         # Security utilities
│   │       ├── mod.rs
│   │       ├── jwt.rs        # JWT generation and validation (HS256)
│   │       └── mod.rs        # PasswordHasher (Argon2id)
│   │
│   ├── interfaces/           # INTERFACES LAYER - HTTP
│   │   ├── mod.rs
│   │   └── http/
│   │       ├── mod.rs
│   │       ├── routes.rs     # Router setup (Axum with CORS)
│   │       ├── auth.rs       # Auth handlers (register, login, refresh)
│   │       ├── user.rs       # User handlers (GET /me)
│   │       ├── assistant.rs  # Assistant handlers (GET /state, POST /command)
│   │       ├── catalog.rs    # Catalog handlers (categories, ingredients search)
│   │       ├── inventory.rs  # Inventory handlers
│   │       ├── recipe.rs     # Recipe handlers (CRUD, cost calculation)
│   │       ├── dish.rs       # Dish handlers (create with financials)
│   │       ├── menu_engineering.rs # Menu Engineering handlers (GET /analysis, POST /sales)
│   │       ├── health.rs     # Health check handler
│   │       ├── middleware.rs # AuthUser extractor (JWT validation)
│   │       └── error.rs      # Error responses (AppError → HTTP status)
│   │
│   └── shared/               # SHARED - Cross-cutting concerns
│       ├── mod.rs
│       ├── types.rs          # TenantId, UserId, RecipeId, DishId, etc.
│       ├── error.rs          # AppError enum (NotFound, Unauthorized, etc.)
│       ├── result.rs         # AppResult<T> type alias
│       ├── language.rs       # Language enum (Pl, En, Uk, Ru)
│       └── i18n.rs           # Multi-language message translation
│

└── tests/                    # Тесты
    ├── domain_tests.rs       # Domain validation tests
    └── integration_tests.rs  # Integration tests
```

---

## 🏗️ Ключевые особенности архитектуры

### 1. DDD Architecture (Domain-Driven Design)
- **Domain**: Чистая бизнес-логика без зависимостей
  - Aggregates: Tenant, User, Recipe, Dish, AssistantState
  - Value Objects: Email, Password, Money, Quantity
  - Entities: RefreshToken, InventoryProduct
- **Application**: Use cases, оркестрация доменных объектов
  - Services координируют domain objects + repositories
  - Транзакционные границы
- **Infrastructure**: БД, JWT, hashing - внешние зависимости
  - PostgreSQL через sqlx (compile-time checked queries)
  - Argon2id для паролей
  - JWT для аутентификации
- **Interfaces**: HTTP handlers, DTOs, маршруты
  - Axum web framework
  - JSON serialization через serde

### 2. Multi-tenancy (SaaS-ready)
- Каждый user принадлежит tenant
- При регистрации создаётся tenant + owner user
- JWT содержит `tenant_id` и `user_id`
- **Все данные изолированы по tenant_id** (Row-Level Security)
- Поддержка нескольких пользователей в одном tenant

### 3. Security & Authentication
- **Пароли**: Argon2id hashing (PHC string format)
- **JWT**: 
  - Access tokens (15 мин) - для API запросов
  - Refresh tokens (30 дней) - для обновления access token
- **Refresh tokens**: хранятся в БД (SHA256 hash), можно отозвать
- **CORS**: Настраиваемые allowed origins (wildcard "*" поддерживается)
- **Input validation**: На уровне domain value objects

### 4. Menu Engineering (BCG Matrix + ABC Analysis)
- **BCG Matrix**: 4 категории блюд
  - ⭐ Star (высокая маржа + высокие продажи)
  - 🐴 Plowhorse (низкая маржа + высокие продажи)
  - 🧩 Puzzle (высокая маржа + низкие продажи)
  - 🐕 Dog (низкая маржа + низкие продажи)
- **ABC Analysis**: Pareto 80/20 по выручке
  - A: топ 80% выручки
  - B: следующие 15%
  - C: последние 5%
- **9 комбинированных стратегий** (BCG × ABC)

### 5. AI Assistant (Wizard Flow)
- **5-шаговый onboarding**:
  1. Start (0%) - приветствие
  2. Inventory Setup (25%) - добавление продуктов
  3. Recipe Setup (50%) - создание рецептов
  4. Dish Setup (75%) - создание блюд с ценами
  5. Report (100%) - финальный отчёт
- **"Момент ВАУ"**: Мгновенный расчёт финансов при создании блюда
  - Себестоимость рецепта
  - Прибыль
  - Маржа (%)
  - Food Cost (%)
- **Предупреждения**:
  - ⚠️ Продукты с истекающим сроком годности
  - ❌ Просроченные продукты
  - 💰 Низкая рентабельность блюд

### 6. Multi-language Support
- **4 языка**: Польский, Английский, Украинский, Русский
- Язык хранится в профиле пользователя
- Все сообщения/подсказки переводятся автоматически

---

## 🔧 Технологии

### Backend Stack
- **axum 0.7** - Modern async web framework
- **tokio 1.x** - Async runtime
- **sqlx 0.7.4** - Database (compile-time checked SQL + offline mode)
- **PostgreSQL 14+** - Relational database
- **argon2 0.5** - Password hashing (Argon2id)
- **jsonwebtoken 9.2** - JWT generation/validation (HS256)
- **serde** - JSON serialization/deserialization
- **uuid** - Unique IDs (UUIDv4)
- **time 0.3.36** - Date/time handling (RFC 3339)
- **thiserror** - Error handling
- **tracing** - Structured logging

### Infrastructure
- **Deployment**: Koyeb (Docker-based, auto-deploy from GitHub)
- **Database**: Neon PostgreSQL (serverless, auto-scaling)
- **Docker**: Multi-stage build (rust:1.83 + debian:bookworm)
- **SQLx Offline Mode**: Build without DB access (.sqlx metadata)

### Key Dependencies Pinned (for Rust 1.83 compatibility)
```toml
time = "=0.3.36"       # Last version without edition2024
base64ct = "=1.6.0"    # Prevent edition2024 requirement
home = "=0.5.9"        # Transitive via sqlx
```

---

## 🚀 Production Deployment (Koyeb)

### Deployment Status
- **URL**: https://ministerial-yetta-fodi999-c58d8823.koyeb.app
- **Status**: ✅ HEALTHY (deployed to production)
- **Database**: Neon PostgreSQL (serverless)
- **Auto-deploy**: GitHub main branch

### Dockerfile (Multi-stage Build)
```dockerfile
FROM rust:1.83 AS builder
WORKDIR /app
RUN apt-get update && apt-get install -y pkg-config libssl-dev

# Copy dependencies + .sqlx metadata
COPY Cargo.toml Cargo.lock ./
COPY .sqlx ./.sqlx

# Enable SQLx offline mode (no DB during build)
ENV SQLX_OFFLINE=true

COPY src ./src
COPY migrations ./migrations

# Build with locked dependencies
RUN cargo build --release --locked

# Runtime stage
FROM debian:bookworm
WORKDIR /app
RUN apt-get update && apt-get install -y ca-certificates libssl3

COPY --from=builder /app/target/release/restaurant-backend /app/restaurant-backend
COPY --from=builder /app/migrations /app/migrations

ENV RUST_BACKTRACE=1
EXPOSE 8000

CMD ["/app/restaurant-backend"]
```

### Environment Variables (Koyeb)
```bash
DATABASE_URL=postgresql://user:pass@host/db?sslmode=require
JWT_SECRET=<64-byte-base64-secret>
PORT=8000
CORS_ALLOWED_ORIGINS=*
JWT_ISSUER=restaurant-backend
ACCESS_TOKEN_TTL_MINUTES=15
REFRESH_TOKEN_TTL_DAYS=30
```

### Health Checks
- **Type**: TCP
- **Port**: 8000
- **Grace Period**: 5s
- **Interval**: 30s

### Auto-Deploy Workflow
1. Push to `main` branch on GitHub
2. Koyeb detects changes
3. Docker build starts (with SQLx offline mode)
4. Health checks validate deployment
5. Traffic switches to new instance

---

## 📡 API Endpoints

### Authentication
```
POST   /api/auth/register    # Register new tenant + user
POST   /api/auth/login       # Login (returns access + refresh tokens)
POST   /api/auth/refresh     # Refresh access token
```

### User Management
```
GET    /api/me               # Get current user + tenant info
```

### AI Assistant (Wizard)
```
GET    /api/assistant/state    # Get current wizard step + state
POST   /api/assistant/command  # Execute command (AddProduct, CreateDish, etc.)
```

### Catalog (Ingredients)
```
GET    /api/catalog/categories          # Get all ingredient categories
GET    /api/catalog/ingredients?search= # Search ingredients by name
```

### Inventory
```
POST   /api/inventory/products  # Add product to inventory
GET    /api/inventory/products  # List inventory (with expiration warnings)
```

### Recipes
```
POST   /api/recipes             # Create recipe
GET    /api/recipes             # List recipes
GET    /api/recipes/:id         # Get recipe details
DELETE /api/recipes/:id         # Delete recipe
GET    /api/recipes/:id/cost    # Calculate recipe cost
```

### Dishes (Menu Items)
```
POST   /api/dishes              # Create dish (returns financials instantly)
GET    /api/dishes              # List dishes
```

### Menu Engineering
```
GET    /api/menu-engineering/analysis?period_days=30  # BCG + ABC analysis
POST   /api/menu-engineering/sales                     # Record sale
```

---

## 🛠️ Development Setup

### Prerequisites
- Rust 1.83+
- PostgreSQL 14+
- sqlx-cli (для миграций)

### Installation

### 1. Настройка окружения
```bash
# Установить sqlx-cli
cargo install sqlx-cli --no-default-features --features postgres

# Скопировать .env
cp .env.example .env

# Отредактировать .env с вашими настройками
```

### 2. Создать и мигрировать БД
```bash
# Создать базу
createdb restaurant_db

# Или использовать Docker
make docker-db

# Запустить миграции
sqlx migrate run
# или
make db-migrate
```

### 3. Запустить сервер
```bash
cargo run
# или
make run
```

### 4. Тестирование
```bash
# Unit tests
cargo test

# Примеры API
chmod +x examples/api_examples.sh
./examples/api_examples.sh
```

## Технологии

- **axum** - Web framework
- **tokio** - Async runtime
- **sqlx** - Database (compile-time checked SQL)
- **argon2** - Password hashing
- **jsonwebtoken** - JWT
- **serde** - Serialization
- **uuid** - Unique IDs
- **time** - Date/time handling
- **thiserror** - Error handling
- **tracing** - Logging


## 📦 Production Checklist

### Completed ✅
- ✅ DDD архитектура с чистым разделением слоёв
- ✅ Multi-tenancy с первого дня
- ✅ Безопасное хранение паролей (Argon2id)
- ✅ JWT authentication (access + refresh tokens)
- ✅ Валидация входных данных на уровне domain
- ✅ Стандартизированные ошибки (AppError)
- ✅ Type-safe ID types (TenantId, UserId, RecipeId, DishId)
- ✅ Database migrations (sqlx)
- ✅ CORS configuration (wildcard support)
- ✅ Structured logging (tracing)
- ✅ No unwrap/expect в runtime коде
- ✅ Unit tests для domain логики
- ✅ Готовность к горизонтальному масштабированию
- ✅ **Koyeb deployment (Docker + auto-deploy from GitHub)**
- ✅ **SQLx offline mode (.sqlx metadata для builds без БД)**
- ✅ **Menu Engineering (BCG Matrix + ABC Analysis)**
- ✅ **AI Assistant (5-step wizard с "Момент ВАУ")**
- ✅ **Multi-language support (PL, EN, UK, RU)**
- ✅ **Health check endpoint**

### In Progress 🔄
- 🔄 Integration tests (частично реализованы)
- 🔄 Frontend integration (Next.js, требует JWT flow)

### Planned ⏳
- ⏳ Priority 2: P&L Reports (profit & loss analytics)
- ⏳ Rate limiting (tower-governor)
- ⏳ Email verification
- ⏳ Password reset flow
- ⏳ CI/CD pipeline
- ⏳ Метрики (Prometheus)
- ⏳ Distributed tracing
- ⏳ Caching (Redis для частых запросов)

---

## 🐛 Troubleshooting

### Docker Build Issues

**Problem**: `error: package requires rustc 1.85.0 or newer`
```
Solution: Pinned dependencies in Cargo.toml
time = "=0.3.36"
base64ct = "=1.6.0"
home = "=0.5.9"
```

**Problem**: `failed to lookup address information: Name or service not known`
```
Solution: 
1. Remove HOST environment variable
2. Always bind to 0.0.0.0 using SocketAddr (no DNS lookup)
3. Use debian:bookworm (not bookworm-slim) for full DNS support
```

**Problem**: `Wildcard origin (*) cannot be passed to AllowOrigin::list`
```rust
Solution: Check for wildcard in CORS setup
if allowed_origins.contains(&"*".to_string()) {
    CorsLayer::permissive()
} else {
    CorsLayer::new().allow_origin(origins)
}
```

**Problem**: SQLx compile-time verification fails during Docker build
```bash
Solution: Use offline mode
1. cargo sqlx prepare                  # Generate .sqlx metadata
2. git add .sqlx && git commit        # Commit metadata
3. ENV SQLX_OFFLINE=true in Dockerfile
```

### Koyeb Deployment Issues

**Problem**: Application exits with code 0/1 immediately
```
Check:
1. Environment variables set correctly (DATABASE_URL, JWT_SECRET)
2. Database accessible (Neon PostgreSQL with sslmode=require)
3. Migrations run successfully (check logs)
4. Server binds to 0.0.0.0:8000 (not localhost)
```

**Problem**: Health checks failing
```
Check:
1. TCP health check on correct port (8000)
2. Server listening before health check grace period expires
3. No blocking operations in startup sequence
```

### Database Issues

**Problem**: Connection pool exhausted
```
Solution: Increase max_connections in config or optimize query patterns
```

**Problem**: Slow queries
```
Solution: 
1. Add indexes on frequently queried columns (tenant_id, user_id)
2. Use EXPLAIN ANALYZE to identify bottlenecks
3. Consider read replicas for analytics queries
```

---

## 📚 Additional Documentation

- **ROADMAP.md** - Feature roadmap (Phase 1-4)
- **QUICKSTART.md** - Quick start guide
- **SECURITY.md** - Security best practices
- **examples/API_EXAMPLES.md** - Complete API usage examples
- **examples/ASSISTANT_API.md** - AI Assistant flow documentation
- **KOYEB_DEPLOYMENT_FINAL.md** - Detailed Koyeb deployment guide
- **INVENTORY_IMPLEMENTATION.md** - Inventory system details
- **EXPIRATION_WARNINGS_IMPLEMENTATION.md** - Expiration tracking
- **PROJECT_STRUCTURE.md** - Detailed file structure

---

## 📞 Support

For issues or questions:
- GitHub Issues: https://github.com/Fodi999/assistant/issues
- Production URL: https://ministerial-yetta-fodi999-c58d8823.koyeb.app

---

**Last Updated**: January 2025
**Version**: 1.0 (Phase 1 Complete + Menu Engineering)
**Status**: ✅ Production Ready (deployed to Koyeb)
