# 🔧 Исправление конфликта миграций

## Проблема

Сервер не может запуститься из-за конфликта версий миграций:
```
Error: VersionMismatch(20240115000001)
```

## Причина

Миграция `20240115000001_add_user_activity_tracking.sql` была создана с номером, который уже используется другой миграцией (`20240115000001_add_price_to_catalog.sql`).

Когда первая попытка деплоя произошла, запись о миграции попала в таблицу `_sqlx_migrations`, но сам файл миграции был переименован на `20240121000001`. Теперь SQLx видит несоответствие.

## Решение

### Вариант 1: Через Neon Console (Рекомендуется)

1. Откройте [Neon Console](https://console.neon.tech)
2. Перейдите в ваш проект
3. Откройте SQL Editor
4. Выполните SQL из файла `manual_migration_fix.sql`:

```sql
-- Шаг 1: Удалить конфликтующую запись
DELETE FROM _sqlx_migrations 
WHERE version = 20240115000001 
  AND description = 'add user activity tracking';

-- Шаг 2: Проверить, существуют ли колонки
SELECT column_name 
FROM information_schema.columns 
WHERE table_name = 'users' 
  AND column_name IN ('login_count', 'last_login_at');

-- Шаг 3: Если колонок нет, создать их
ALTER TABLE users 
ADD COLUMN IF NOT EXISTS login_count INTEGER NOT NULL DEFAULT 0,
ADD COLUMN IF NOT EXISTS last_login_at TIMESTAMPTZ;

CREATE INDEX IF NOT EXISTS idx_users_last_login ON users(last_login_at DESC NULLS LAST);
CREATE INDEX IF NOT EXISTS idx_users_login_count ON users(login_count DESC);

-- Шаг 4: Проверить миграции
SELECT version, description, success 
FROM _sqlx_migrations 
ORDER BY version DESC 
LIMIT 10;
```

5. После выполнения SQL, Koyeb автоматически задеплоит новую версию

### Вариант 2: Через psql (если есть доступ)

```bash
# Экспортируйте DATABASE_URL из Koyeb
export DATABASE_URL="postgresql://user:pass@host/db"

# Выполните скрипт
psql "$DATABASE_URL" -f manual_migration_fix.sql
```

### Вариант 3: Полный откат миграции

Если хотите полностью откатить изменения:

```sql
-- Удалить запись о миграции
DELETE FROM _sqlx_migrations WHERE version = 20240115000001;

-- Удалить колонки (если были созданы)
ALTER TABLE users 
DROP COLUMN IF EXISTS login_count,
DROP COLUMN IF EXISTS last_login_at;

-- Удалить индексы
DROP INDEX IF EXISTS idx_users_last_login;
DROP INDEX IF EXISTS idx_users_login_count;
```

Затем Koyeb применит миграцию `20240121000001` заново.

## Проверка успешности

После выполнения SQL, проверьте что:

1. **Миграция удалена из таблицы:**
```sql
SELECT * FROM _sqlx_migrations WHERE version = 20240115000001;
-- Должно вернуть 0 строк
```

2. **Колонки существуют:**
```sql
\d users
-- Должны быть видны login_count и last_login_at
```

3. **Koyeb успешно задеплоил:**
- Зайдите в Koyeb Logs
- Должны увидеть: `Database migrations completed`
- Сервер запущен: `Server listening on 0.0.0.0:8000`

## Тест после исправления

```bash
# 1. Получить токен админа
TOKEN=$(curl -s -X POST "https://ministerial-yetta-fodi999-c58d8823.koyeb.app/api/admin/auth/login" \
  -H "Content-Type: application/json" \
  -d '{"email":"admin@fodi.app","password":"Admin123!"}' | jq -r '.token')

# 2. Получить список пользователей с активностью
curl -s "https://ministerial-yetta-fodi999-c58d8823.koyeb.app/api/admin/users" \
  -H "Authorization: Bearer $TOKEN" | \
  jq '.users[0] | {email, login_count, last_login_at}'
```

Должны увидеть поля `login_count` и `last_login_at` в ответе.

## Предотвращение в будущем

1. Всегда проверяйте последний номер миграции перед созданием новой:
```bash
ls -la migrations/ | tail -5
```

2. Используйте следующий доступный номер (текущий последний: `20240120000001`, следующий: `20240121000001`)

3. Никогда не меняйте номер миграции после того, как она была применена в production

---

**Статус:** Готово к исправлению
**Файлы:** 
- `manual_migration_fix.sql` - SQL для ручного исправления
- `fix_migration_conflict.sh` - Bash скрипт (опционально)
