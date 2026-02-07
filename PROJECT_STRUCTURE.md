# 📁 Структура проекта Restaurant Backend

```
restaurant-backend/
│
├── 📄 Cargo.toml                    # Зависимости и конфигурация проекта
├── 📄 Makefile                      # Команды для удобства (db-migrate, run, test, etc.)
├── 📄 README.md                     # Основная документация
├── 📄 ARCHITECTURE.md               # Подробное описание архитектуры DDD
├── 📄 QUICKSTART.md                 # Быстрый старт
│
├── 📄 .env                          # Переменные окружения (DATABASE_URL, JWT_SECRET, etc.)
├── 📄 .env.example                  # Пример конфигурации
├── 📄 .gitignore                    # Игнорируемые файлы
├── 📄 .sqlxrc                       # Конфигурация sqlx
│
├── 📁 .cargo/
│   └── 📄 config.toml               # Cargo конфигурация (SQLX_OFFLINE)
│
├── 📁 migrations/                   # 🗄️ SQL миграции базы данных
│   └── 📄 20240101000001_initial_schema.sql
│       ├── CREATE TABLE tenants
│       ├── CREATE TABLE users
│       ├── CREATE TABLE refresh_tokens
│       └── CREATE INDEX (email, tenant_id, user_id, token_hash)
│
├── 📁 examples/                     # 📝 Примеры использования API
│   ├── 📄 API_EXAMPLES.md           # Документация с примерами curl запросов
│   └── 📄 api_examples.sh           # Bash скрипт для автоматического тестирования
│
├── 📁 tests/                        # 🧪 Тесты
│   ├── 📄 domain_tests.rs           # Unit тесты для domain валидации
│   └── 📄 integration_tests.rs      # Integration тесты (заглушки)
│
└── 📁 src/                          # 💻 Исходный код
    │
    ├── 📄 main.rs                   # 🚀 Точка входа приложения
    │   ├── Инициализация логирования
    │   ├── Загрузка конфигурации
    │   ├── Подключение к БД
    │   ├── Запуск миграций
    │   ├── Создание сервисов
    │   └── Запуск HTTP сервера
    │
    ├── 📁 shared/                   # 🔧 Общие утилиты и типы
    │   ├── 📄 mod.rs
    │   ├── 📄 types.rs               # TenantId, UserId, RefreshTokenId (newtype wrappers)
    │   ├── 📄 error.rs               # AppError enum (Validation, Authentication, etc.)
    │   └── 📄 result.rs              # AppResult<T> type alias
    │
    ├── 📁 domain/                   # 🎯 DOMAIN LAYER - Бизнес-логика
    │   ├── 📄 mod.rs
    │   │
    │   ├── 📄 tenant.rs              # Tenant Aggregate
    │   │   ├── struct Tenant { id, name, created_at }
    │   │   └── struct TenantName (value object с валидацией)
    │   │
    │   ├── 📄 user.rs                # User Aggregate
    │   │   ├── struct User { id, tenant_id, email, password_hash, display_name, role, created_at }
    │   │   ├── enum UserRole { Owner, Manager, Staff }
    │   │   ├── struct Email (value object с email валидацией)
    │   │   ├── struct DisplayName (value object с length валидацией)
    │   │   └── struct Password (value object с complexity валидацией)
    │   │
    │   └── 📄 auth.rs                # RefreshToken Entity
    │       └── struct RefreshToken { id, user_id, token_hash, expires_at, revoked_at, created_at }
    │
    ├── 📁 application/              # 🔄 APPLICATION LAYER - Use Cases
    │   ├── 📄 mod.rs
    │   │
    │   ├── 📄 auth.rs                # AuthService
    │   │   ├── async fn register() -> AuthResponse
    │   │   │   └── Создаёт Tenant + Owner User + Tokens
    │   │   ├── async fn login() -> AuthResponse
    │   │   │   └── Проверяет credentials + генерирует tokens
    │   │   └── async fn refresh() -> AuthResponse
    │   │       └── Обновляет access token по refresh token
    │   │
    │   └── 📄 user.rs                # UserService
    │       └── async fn get_user_with_tenant() -> UserWithTenant
    │
    ├── 📁 infrastructure/           # 🏗️ INFRASTRUCTURE LAYER
    │   ├── 📄 mod.rs
    │   ├── 📄 config.rs              # Config (загрузка из env)
    │   │   ├── DatabaseConfig
    │   │   ├── ServerConfig
    │   │   ├── JwtConfig
    │   │   └── CorsConfig
    │   │
    │   ├── 📁 persistence/           # 🗄️ Репозитории (PostgreSQL + sqlx)
    │   │   ├── 📄 mod.rs
    │   │   ├── 📄 tenant_repository.rs
    │   │   │   ├── trait TenantRepositoryTrait
    │   │   │   ├── async fn create()
    │   │   │   └── async fn find_by_id()
    │   │   │
    │   │   ├── 📄 user_repository.rs
    │   │   │   ├── trait UserRepositoryTrait
    │   │   │   ├── async fn create()
    │   │   │   ├── async fn find_by_id()
    │   │   │   ├── async fn find_by_email()
    │   │   │   └── async fn exists_by_email()
    │   │   │
    │   │   └── 📄 refresh_token_repository.rs
    │   │       ├── trait RefreshTokenRepositoryTrait
    │   │       ├── async fn create()
    │   │       ├── async fn find_by_token_hash()
    │   │       ├── async fn revoke()
    │   │       └── async fn revoke_all_for_user()
    │   │
    │   └── 📁 security/              # 🔐 Безопасность
    │       ├── 📄 mod.rs
    │       │
    │       ├── 📄 password.rs        # PasswordHasher (Argon2)
    │       │   ├── fn hash_password() -> String
    │       │   └── fn verify_password() -> bool
    │       │
    │       └── 📄 jwt.rs             # JwtService
    │           ├── fn generate_access_token() -> String
    │           ├── fn generate_refresh_token() -> String
    │           ├── fn verify_access_token() -> AccessTokenClaims
    │           └── struct AccessTokenClaims { sub, tenant_id, iss, iat, exp }
    │
    └── 📁 interfaces/               # 🌐 INTERFACES LAYER - HTTP
        ├── 📄 mod.rs
        │
        └── 📁 http/
            ├── 📄 mod.rs
            │
            ├── 📄 routes.rs          # 🛣️ Router Setup
            │   ├── fn create_router() -> Router
            │   ├── CORS configuration
            │   ├── Auth routes (public)
            │   └── Protected routes + JWT middleware
            │
            ├── 📄 auth.rs            # 🔑 Auth Handlers
            │   ├── POST /api/auth/register
            │   │   └── async fn register_handler()
            │   ├── POST /api/auth/login
            │   │   └── async fn login_handler()
            │   └── POST /api/auth/refresh
            │       └── async fn refresh_handler()
            │
            ├── 📄 user.rs            # 👤 User Handlers
            │   └── GET /api/me (protected)
            │       └── async fn me_handler()
            │
            ├── 📄 middleware.rs      # 🛡️ Middleware
            │   └── struct AuthUser (JWT extractor)
            │       └── impl FromRequestParts
            │
            └── 📄 error.rs           # ⚠️ Error Handling
                ├── impl IntoResponse for AppError
                └── struct ErrorResponse { code, message, details }
```

## 📊 Статистика проекта

### Файлы по категориям:
- **Domain Layer**: 3 файла (tenant.rs, user.rs, auth.rs)
- **Application Layer**: 2 файла (auth.rs, user.rs)
- **Infrastructure Layer**: 7 файлов (config, 3 repositories, 2 security)
- **Interfaces Layer**: 5 файлов (routes, auth, user, middleware, error)
- **Shared**: 3 файла (types, error, result)
- **Tests**: 2 файла
- **Migrations**: 1 файл
- **Documentation**: 4 файла (README, ARCHITECTURE, QUICKSTART, API_EXAMPLES)

### Всего строк кода (примерно):
- Domain: ~300 строк
- Application: ~250 строк
- Infrastructure: ~500 строк
- Interfaces: ~300 строк
- Shared: ~150 строк
- **Итого: ~1500 строк чистого Rust кода**

## 🎯 Ключевые особенности архитектуры

### 1. **Domain Layer** (Чистая бизнес-логика)
- ✅ Нет зависимостей от внешних библиотек (кроме serde, time)
- ✅ Value Objects с валидацией
- ✅ Доменные правила
- ✅ Strong typing (TenantId, UserId вместо UUID напрямую)

### 2. **Application Layer** (Use Cases)
- ✅ Оркестрация доменных объектов
- ✅ Транзакционная логика
- ✅ Вызов репозиториев и сервисов

### 3. **Infrastructure Layer** (Внешний мир)
- ✅ PostgreSQL через sqlx (без ORM)
- ✅ Argon2 для паролей
- ✅ JWT для токенов
- ✅ Конфигурация из env

### 4. **Interfaces Layer** (HTTP API)
- ✅ Axum web framework
- ✅ DTOs для запросов/ответов
- ✅ Middleware для аутентификации
- ✅ Стандартизированные ошибки

## 🔌 API Endpoints

```
PUBLIC:
  POST /api/auth/register  - Регистрация (создаёт tenant + owner user)
  POST /api/auth/login     - Вход (возвращает access + refresh tokens)
  POST /api/auth/refresh   - Обновление access token

PROTECTED:
  GET /api/me              - Текущий пользователь + tenant info
```

## 🗄️ Database Schema

```sql
tenants (id, name, created_at)
  ↓ one-to-many
users (id, tenant_id, email, password_hash, display_name, role, created_at)
  ↓ one-to-many
refresh_tokens (id, user_id, token_hash, expires_at, revoked_at, created_at)
```

## 🚀 Запуск

```bash
# 1. Настройка
cp .env.example .env
# Отредактируйте .env с вашими настройками

# 2. Миграции (уже выполнены для Neon DB)
sqlx migrate run

# 3. Запуск
cargo run

# 4. Тестирование
curl -X POST http://localhost:8080/api/auth/register \
  -H "Content-Type: application/json" \
  -d '{"email":"test@example.com","password":"SecurePass123!","restaurant_name":"Test Restaurant","owner_name":"John Doe"}'
```

## ✅ Production Ready Features

- [x] DDD архитектура
- [x] Multi-tenancy
- [x] Secure password hashing (Argon2)
- [x] JWT authentication
- [x] Refresh tokens с revocation
- [x] Input validation
- [x] Error handling
- [x] Database migrations
- [x] CORS support
- [x] Structured logging
- [x] Type safety
- [x] Unit tests
- [x] API documentation

## 📈 Следующие шаги для расширения

1. **Menu Domain**: Добавить категории, продукты, модификаторы
2. **Orders Domain**: Создание заказов, статусы, оплата
3. **Staff Domain**: Управление сотрудниками, роли, расписание
4. **Analytics Domain**: Отчёты, статистика, метрики
5. **Notifications**: Email/SMS уведомления
6. **File Upload**: Загрузка изображений для продуктов
7. **Search**: Полнотекстовый поиск
8. **WebSockets**: Real-time обновления заказов
