# Структура проекта

```
restaurant-backend/
├── Cargo.toml                 # Зависимости проекта
├── Makefile                   # Команды для удобства
├── README.md                  # Документация
├── .env.example               # Пример конфигурации
├── .gitignore                 # Git игнорирование
│
├── migrations/                # SQL миграции (sqlx)
│   └── 20240101000001_initial_schema.sql
│
├── examples/                  # Примеры использования API
│   ├── api_examples.sh
│   └── API_EXAMPLES.md
│
├── src/
│   ├── main.rs               # Точка входа
│   │
│   ├── domain/               # DOMAIN LAYER - Бизнес-логика
│   │   ├── mod.rs
│   │   ├── tenant.rs         # Tenant aggregate + TenantName value object
│   │   ├── user.rs           # User aggregate + Email, DisplayName, Password value objects
│   │   └── auth.rs           # RefreshToken entity
│   │
│   ├── application/          # APPLICATION LAYER - Use cases
│   │   ├── mod.rs
│   │   ├── auth.rs           # AuthService: register, login, refresh
│   │   └── user.rs           # UserService: get_user_with_tenant
│   │
│   ├── infrastructure/       # INFRASTRUCTURE LAYER
│   │   ├── mod.rs
│   │   ├── config.rs         # Конфигурация из env
│   │   │
│   │   ├── persistence/      # Репозитории (PostgreSQL + sqlx)
│   │   │   ├── mod.rs
│   │   │   ├── tenant_repository.rs
│   │   │   ├── user_repository.rs
│   │   │   └── refresh_token_repository.rs
│   │   │
│   │   └── security/         # Security utilities
│   │       ├── mod.rs
│   │       ├── password.rs   # Argon2 password hashing
│   │       └── jwt.rs        # JWT generation and validation
│   │
│   ├── interfaces/           # INTERFACES LAYER - HTTP
│   │   ├── mod.rs
│   │   └── http/
│   │       ├── mod.rs
│   │       ├── routes.rs     # Router setup
│   │       ├── auth.rs       # Auth handlers
│   │       ├── user.rs       # User handlers
│   │       ├── middleware.rs # AuthUser extractor
│   │       └── error.rs      # Error responses
│   │
│   └── shared/               # SHARED - Cross-cutting concerns
│       ├── mod.rs
│       ├── types.rs          # TenantId, UserId, RefreshTokenId
│       ├── error.rs          # AppError enum
│       └── result.rs         # AppResult type alias
│
└── tests/                    # Тесты
    ├── domain_tests.rs       # Domain validation tests
    └── integration_tests.rs  # Integration tests
```

## Ключевые особенности

### 1. DDD Architecture
- **Domain**: Чистая бизнес-логика без зависимостей
- **Application**: Use cases, оркестрация доменных объектов
- **Infrastructure**: БД, JWT, hashing - внешние зависимости
- **Interfaces**: HTTP handlers, DTOs, маршруты

### 2. Multi-tenancy
- Каждый user принадлежит tenant
- При регистрации создаётся tenant + owner user
- JWT содержит `tenant_id` и `user_id`

### 3. Security
- Пароли: Argon2 hashing (не хранятся в открытом виде)
- JWT: Access tokens (15 мин) + Refresh tokens (30 дней)
- Refresh tokens: хранятся в БД (SHA256 hash), можно revoke
- Валидация на уровне domain (Email, Password, etc.)

### 4. API Endpoints
```
POST /api/auth/register  - Регистрация (создаёт tenant + user)
POST /api/auth/login     - Вход
POST /api/auth/refresh   - Обновление access token
GET  /api/me             - Получить текущего пользователя (защищённый)
```

### 5. Database Schema
```sql
tenants:
  - id (UUID, PK)
  - name (TEXT)
  - created_at (TIMESTAMPTZ)

users:
  - id (UUID, PK)
  - tenant_id (UUID, FK → tenants)
  - email (TEXT, UNIQUE)
  - password_hash (TEXT)
  - display_name (TEXT, nullable)
  - role (TEXT: owner/manager/staff)
  - created_at (TIMESTAMPTZ)

refresh_tokens:
  - id (UUID, PK)
  - user_id (UUID, FK → users)
  - token_hash (TEXT)
  - expires_at (TIMESTAMPTZ)
  - revoked_at (TIMESTAMPTZ, nullable)
  - created_at (TIMESTAMPTZ)
```

## Команды запуска

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

## Production Checklist

✅ DDD архитектура с чистым разделением слоёв
✅ Multi-tenancy с первого дня
✅ Безопасное хранение паролей (Argon2)
✅ JWT authentication (access + refresh tokens)
✅ Валидация входных данных
✅ Стандартизированные ошибки
✅ Type-safe ID types (TenantId, UserId)
✅ Database migrations (sqlx)
✅ CORS configuration
✅ Structured logging (tracing)
✅ No unwrap/expect в runtime коде
✅ Unit tests для domain логики
✅ Готовность к горизонтальному масштабированию

## Следующие шаги

1. Добавить rate limiting (tower-governor)
2. Добавить email verification
3. Добавить password reset flow
4. Добавить более полные integration tests
5. Добавить health check endpoint
6. Настроить CI/CD
7. Добавить метрики (Prometheus)
8. Настроить distributed tracing
9. Добавить caching (Redis)
10. Реализовать остальные доменные модули (Menu, Orders, etc.)
