# 📊 Отслеживание активности пользователей - РЕАЛИЗОВАНО

## 🎯 Что добавлено

Система отслеживания активности пользователей для админ-панели:
- Счётчик входов в систему
- Дата последнего входа
- Сортировка по активности (самые активные вверху)

## 📋 Реализация

### 1. Миграция базы данных

**Файл:** `migrations/20240115000001_add_user_activity_tracking.sql`

```sql
ALTER TABLE users 
ADD COLUMN IF NOT EXISTS login_count INTEGER NOT NULL DEFAULT 0,
ADD COLUMN IF NOT EXISTS last_login_at TIMESTAMPTZ;

-- Индексы для быстрой сортировки
CREATE INDEX IF NOT EXISTS idx_users_last_login ON users(last_login_at DESC NULLS LAST);
CREATE INDEX IF NOT EXISTS idx_users_login_count ON users(login_count DESC);
```

**Новые поля:**
- `login_count` - количество входов (по умолчанию 0)
- `last_login_at` - дата последнего входа (nullable)

### 2. Обновлённая структура API

**Обновлённый `UserInfo` в `admin_users.rs`:**

```rust
#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct UserInfo {
    pub id: String,
    pub email: String,
    pub name: Option<String>,
    pub restaurant_name: String,
    pub language: String,
    pub created_at: String,
    pub login_count: i32,                // ✨ НОВОЕ
    pub last_login_at: Option<String>,   // ✨ НОВОЕ
}
```

### 3. Обновлённый SQL запрос

**Сортировка по активности:**

```sql
SELECT 
    u.id::text,
    u.email,
    u.display_name as name,
    t.name as restaurant_name,
    COALESCE(u.language, 'ru') as language,
    u.created_at::text,
    u.login_count,                         -- ✨ НОВОЕ
    u.last_login_at::text as last_login_at -- ✨ НОВОЕ
FROM users u
JOIN tenants t ON u.tenant_id = t.id
ORDER BY u.login_count DESC, u.last_login_at DESC NULLS LAST
```

**Логика сортировки:**
1. Сначала по количеству входов (больше = выше)
2. Потом по дате последнего входа (новее = выше)
3. Пользователи без входов в конце (NULLS LAST)

### 4. Автоматическое обновление при логине

**Добавлен метод в `UserRepository`:**

```rust
async fn update_login_stats(&self, user_id: UserId) -> AppResult<()> {
    sqlx::query(
        "UPDATE users SET login_count = login_count + 1, last_login_at = NOW() WHERE id = $1"
    )
    .bind(user_id.as_uuid())
    .execute(&self.pool)
    .await?;

    Ok(())
}
```

**Интеграция в `auth.rs`:**

```rust
// После проверки пароля
if !password_valid {
    return Err(AppError::authentication("Invalid email or password"));
}

// Обновляем статистику входа
if let Err(e) = self.user_repo.update_login_stats(user.id).await {
    tracing::warn!("Failed to update login statistics: {}", e);
}

// Генерируем токены
...
```

**Важно:** Ошибка обновления статистики не блокирует логин (только warning в логах).

## 📱 Пример ответа API

### GET /api/admin/users

```json
{
  "users": [
    {
      "id": "123e4567-e89b-12d3-a456-426614174000",
      "email": "active-user@example.com",
      "name": "Активный Пользователь",
      "restaurant_name": "Активный Ресторан",
      "language": "ru",
      "created_at": "2024-01-15T10:00:00Z",
      "login_count": 245,                    // ✨ Много входов
      "last_login_at": "2024-02-14T09:30:00Z" // ✨ Недавно заходил
    },
    {
      "id": "234e5678-e89b-12d3-a456-426614174001",
      "email": "rare-user@example.com",
      "name": "Редкий Пользователь",
      "restaurant_name": "Редкий Ресторан",
      "language": "en",
      "created_at": "2024-01-10T12:00:00Z",
      "login_count": 3,                      // ✨ Мало входов
      "last_login_at": "2024-01-20T15:00:00Z" // ✨ Давно не заходил
    },
    {
      "id": "345e6789-e89b-12d3-a456-426614174002",
      "email": "never-logged@example.com",
      "name": null,
      "restaurant_name": "Никогда не входил",
      "language": "ru",
      "created_at": "2024-02-01T08:00:00Z",
      "login_count": 0,                      // ✨ Никогда не входил
      "last_login_at": null                  // ✨ NULL
    }
  ],
  "total": 3
}
```

## 🎨 Frontend отображение

### React компонент с активностью

```tsx
function UsersListTable() {
  const [users, setUsers] = useState<UserInfo[]>([]);

  // ... fetch logic ...

  const formatLastLogin = (lastLogin: string | null): string => {
    if (!lastLogin) return 'Никогда';
    
    const date = new Date(lastLogin);
    const now = new Date();
    const diffMs = now.getTime() - date.getTime();
    const diffDays = Math.floor(diffMs / (1000 * 60 * 60 * 24));
    
    if (diffDays === 0) return 'Сегодня';
    if (diffDays === 1) return 'Вчера';
    if (diffDays < 7) return `${diffDays} дней назад`;
    if (diffDays < 30) return `${Math.floor(diffDays / 7)} недель назад`;
    
    return date.toLocaleDateString('ru-RU');
  };

  return (
    <table className="users-table">
      <thead>
        <tr>
          <th>Email</th>
          <th>Имя</th>
          <th>Ресторан</th>
          <th>Входов</th>          {/* ✨ НОВОЕ */}
          <th>Последний вход</th>   {/* ✨ НОВОЕ */}
          <th>Действия</th>
        </tr>
      </thead>
      <tbody>
        {users.map(user => (
          <tr key={user.id}>
            <td>{user.email}</td>
            <td>{user.name || '—'}</td>
            <td>{user.restaurant_name}</td>
            <td>
              <span className={user.login_count > 100 ? 'high-activity' : ''}>
                {user.login_count}
              </span>
            </td>
            <td>{formatLastLogin(user.last_login_at)}</td>
            <td>
              <button onClick={() => handleDelete(user.id)}>Удалить</button>
            </td>
          </tr>
        ))}
      </tbody>
    </table>
  );
}
```

### CSS стили

```css
.high-activity {
  font-weight: bold;
  color: #16a34a; /* зелёный для активных */
}

.users-table tbody tr {
  opacity: 1;
}

.users-table tbody tr:has(td:nth-child(4):contains("0")) {
  opacity: 0.6; /* полупрозрачность для неактивных */
}
```

## 📊 Метрики активности

### Категории пользователей

1. **Супер активные** (login_count > 100)
   - Заходят регулярно
   - Отображаются первыми
   - Зелёная подсветка

2. **Активные** (login_count 10-100)
   - Периодически используют систему
   - Обычное отображение

3. **Редкие** (login_count 1-10)
   - Заходили несколько раз
   - Потенциально нуждаются в обучении

4. **Неактивные** (login_count = 0)
   - Ни разу не входили
   - Отображаются последними
   - Полупрозрачные
   - Возможно, нужно напомнить о регистрации

## 🔍 Возможные улучшения

### Dashboard с аналитикой

```typescript
interface UserActivityStats {
  total_users: number;
  active_today: number;        // last_login_at = today
  active_this_week: number;     // last_login_at > now - 7 days
  active_this_month: number;    // last_login_at > now - 30 days
  never_logged_in: number;      // login_count = 0
  average_logins: number;       // AVG(login_count)
}
```

### Дополнительные метрики

```sql
-- Средний интервал между входами
SELECT 
    user_id,
    login_count,
    EXTRACT(EPOCH FROM (last_login_at - created_at)) / login_count AS avg_days_between_logins
FROM users
WHERE login_count > 1;

-- Пользователи, которые давно не заходили
SELECT *
FROM users
WHERE last_login_at < NOW() - INTERVAL '30 days'
  AND login_count > 0
ORDER BY last_login_at DESC;
```

## 🧪 Тестирование

### Тест 1: Проверка миграции

```bash
# После деплоя проверяем структуру таблицы
psql $DATABASE_URL -c "\d users"

# Должны быть поля:
# login_count | integer | not null | default 0
# last_login_at | timestamp with time zone | |
```

### Тест 2: Проверка обновления при логине

```bash
# 1. Получаем текущую статистику
TOKEN=$(curl -s -X POST "$API_URL/api/admin/auth/login" \
  -H "Content-Type: application/json" \
  -d '{"email":"admin@fodi.app","password":"Admin123!"}' | jq -r '.token')

BEFORE=$(curl -s "$API_URL/api/admin/users" \
  -H "Authorization: Bearer $TOKEN" | \
  jq '.users[] | select(.email == "test@example.com") | {login_count, last_login_at}')

echo "До: $BEFORE"

# 2. Логинимся от имени пользователя
curl -s -X POST "$API_URL/api/auth/login" \
  -H "Content-Type: application/json" \
  -d '{"email":"test@example.com","password":"password"}' > /dev/null

# 3. Проверяем обновление
AFTER=$(curl -s "$API_URL/api/admin/users" \
  -H "Authorization: Bearer $TOKEN" | \
  jq '.users[] | select(.email == "test@example.com") | {login_count, last_login_at}')

echo "После: $AFTER"

# login_count должен увеличиться на 1
# last_login_at должен обновиться
```

### Тест 3: Проверка сортировки

```bash
# Проверяем, что пользователи отсортированы по активности
curl -s "$API_URL/api/admin/users" \
  -H "Authorization: Bearer $TOKEN" | \
  jq '.users | .[0:5] | .[] | {email, login_count, last_login_at}'

# Первые 5 должны иметь наибольший login_count
```

## 📝 История изменений

**Commit:** `0e13eae` - "feat: Add user activity tracking (login count and last login date)"

**Изменённые файлы:**
1. `migrations/20240115000001_add_user_activity_tracking.sql` - новая миграция
2. `src/interfaces/http/admin_users.rs` - обновлён UserInfo и SQL запрос
3. `src/infrastructure/persistence/user_repository.rs` - добавлен метод update_login_stats
4. `src/application/auth.rs` - вызов update_login_stats при логине

## 🎉 Статус

**✅ ГОТОВО К ДЕПЛОЮ**

- Миграция создана ✅
- Backend обновлён ✅
- Автоматическое обновление при логине ✅
- Сортировка по активности ✅
- Компиляция успешна ✅
- Committed и pushed ✅

## 📚 Связанные документы

- `FRONTEND_ADMIN_GUIDE.md` - Общее руководство по админ-панели
- `ADMIN_USERS_COMPLETE.md` - Документация управления пользователями
- `ADMIN_DELETE_USER_SUCCESS.md` - Удаление пользователей

---

**Дата:** 14 февраля 2026  
**Автор:** AI Assistant  
**Commit:** 0e13eae
