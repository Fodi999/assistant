# ⚠️ СРОЧНОЕ ИСПРАВЛЕНИЕ - Конфликт миграций

## Что случилось

Миграция с номером `20240115000001` была создана дважды с разными именами, что вызвало конфликт версий.

## Быстрое решение (выполните в Neon SQL Editor)

Откройте https://console.neon.tech → ваш проект → SQL Editor и выполните:

```sql
DELETE FROM _sqlx_migrations WHERE version = 20240115000001 AND description = 'add user activity tracking';
```

**Важно:** Выполните только эту одну команду! После этого Koyeb сможет задеплоить новую миграцию (`20240122000001`).

## После выполнения SQL

1. Koyeb автоматически попытается задеплоить снова
2. Новая миграция `20240122000001_fix_user_activity_tracking.sql` будет применена
3. Колонки `login_count` и `last_login_at` будут созданы (если их ещё нет)
4. Сервер запустится успешно

## Проверка

После деплоя выполните:

```bash
TOKEN=$(curl -s -X POST "https://ministerial-yetta-fodi999-c58d8823.koyeb.app/api/admin/auth/login" \
  -H "Content-Type: application/json" \
  -d '{"email":"admin@fodi.app","password":"Admin123!"}' | jq -r '.token')

curl -s "https://ministerial-yetta-fodi999-c58d8823.koyeb.app/api/admin/users" \
  -H "Authorization: Bearer $TOKEN" | jq '.users[0]'
```

Должны увидеть поля `login_count: 0` и `last_login_at: null`.

---

**Статус:** Готов commit с исправлением, ждём выполнения SQL
