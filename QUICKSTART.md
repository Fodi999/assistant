# Краткая инструкция по запуску

## ✅ Проект успешно создан!

### Что сделано:
1. ✅ DDD архитектура (domain/application/infrastructure/interfaces)
2. ✅ Multi-tenant система
3. ✅ Auth система (register/login/refresh)
4. ✅ PostgreSQL схема и миграции
5. ✅ Безопасность (Argon2, JWT)
6. ✅ Валидация данных

### База данных подключена:
- Таблицы созданы: `tenants`, `users`, `refresh_tokens`
- Миграции применены успешно

## 🚀 Как запустить проект:

### Вариант 1: Использовать sqlx offline mode (рекомендуется для разработки)

Добавьте в Cargo.toml в секцию sqlx:
```toml
sqlx = { version = "0.7", features = ["runtime-tokio-rustls", "postgres", "uuid", "time", "migrate", "offline"] }
```

Затем создайте файл `.cargo/config.toml`:
```bash
mkdir -p .cargo
cat > .cargo/config.toml << 'EOF'
[env]
SQLX_OFFLINE = "true"
EOF
```

И соберите:
```bash
cargo build --release
```

### Вариант 2: Использовать prepared metadata

Используйте прямое подключение (не pooler) для подготовки:
```bash
# Получите direct connection string из Neon Dashboard
# Обычно это замена -pooler.eu-central-1 на .eu-central-1
DATABASE_URL="postgresql://neondb_owner:PASSWORD@ep-orange-bird-a2yh5v07.eu-central-1.aws.neon.tech/neondb?sslmode=require" cargo sqlx prepare

# Затем соберите проект
cargo build --release
```

### Вариант 3: Собрать и запустить сразу (проще всего)

```bash
cargo run
```

Сервер запустится на `http://localhost:8080`

## 📝 Тестирование API:

```bash
# 1. Регистрация
curl -X POST http://localhost:8080/api/auth/register \
  -H "Content-Type: application/json" \
  -d '{
    "email": "owner@restaurant.com",
    "password": "SecurePass123!",
    "restaurant_name": "My Restaurant",
    "owner_name": "John Doe"
  }'

# 2. Вход
curl -X POST http://localhost:8080/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{
    "email": "owner@restaurant.com",
    "password": "SecurePass123!"
  }'

# 3. Получить токен и использовать
# (скопируйте access_token из ответа)
curl -X GET http://localhost:8080/api/me \
  -H "Authorization: Bearer YOUR_ACCESS_TOKEN"
```

## 🔧 Если возникли проблемы:

### Проблема: sqlx требует DATABASE_URL во время компиляции

**Решение:** Используйте offline mode (см. Вариант 1 выше)

### Проблема: ошибки компиляции с "prepared statement does not exist"

**Решение:** Это происходит потому что Neon pooler не поддерживает prepared statements при компиляции. Используйте:
```bash
SQLX_OFFLINE=true cargo build
```

Или добавьте в `.cargo/config.toml`:
```toml
[env]
SQLX_OFFLINE = "true"
```

## 📚 Документация:

- `README.md` - Полная документация
- `ARCHITECTURE.md` - Описание архитектуры
- `examples/API_EXAMPLES.md` - Примеры API запросов
- `examples/api_examples.sh` - Bash скрипт для тестирования

## 🎯 Следующие шаги:

1. Запустите сервер: `cargo run`
2. Протестируйте API с помощью curl или Postman
3. Изучите структуру кода в `src/`
4. Добавьте свою бизнес-логику в domain модели

Удачи! 🚀
