# 🔧 Исправление: Имена пользователей не сохраняются

**Дата:** 14 февраля 2026  
**Проблема:** В админ панели у большинства пользователей отображается "—" вместо имени  
**Статус:** 🟡 Требует исправления на фронтенде

---

## 🔍 Анализ проблемы

### Текущая ситуация

**Статистика:**
- Всего пользователей: 52
- С именами: 2 (3.8%)
- Без имен: 50 (96.2%)

**Пользователи с именами:**
```json
{
  "email": "test_catalog@restaurant.com",
  "name": "Catalog Tester",
  "restaurant_name": "Test Restaurant PL"
}
{
  "email": "test_pl@restaurant.com",
  "name": "Jan Kowalski",
  "restaurant_name": "Polish Restaurant"
}
```

---

## 🧪 Проверка Backend

### 1. Структура базы данных ✅

```sql
CREATE TABLE users (
    id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL REFERENCES tenants(id),
    email TEXT NOT NULL UNIQUE,
    password_hash TEXT NOT NULL,
    display_name TEXT,        -- ✅ Колонка есть
    role TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

**Результат:** Колонка `display_name` существует и может хранить имя.

---

### 2. API Endpoint регистрации ✅

```rust
// src/interfaces/http/auth.rs
#[derive(Debug, Deserialize, Validate)]
pub struct RegisterRequest {
    #[validate(email)]
    pub email: String,
    
    #[validate(length(min = 8, max = 128))]
    pub password: String,
    
    #[validate(length(min = 1, max = 255))]
    pub restaurant_name: String,
    
    #[validate(length(min = 1, max = 255))]
    pub owner_name: Option<String>,  // ✅ Поле есть (опциональное)
    
    pub language: Option<Language>,
}
```

**API:**
```http
POST /api/auth/register
Content-Type: application/json

{
  "email": "user@example.com",
  "password": "SecurePass123",
  "restaurant_name": "My Restaurant",
  "owner_name": "John Doe",       // ✅ Можно передать
  "language": "ru"
}
```

**Результат:** Backend принимает и сохраняет `owner_name`.

---

### 3. Сохранение в БД ✅

```rust
// src/application/auth.rs - register()
let owner_name = command
    .owner_name
    .map(DisplayName::new)
    .transpose()?;

let user = User::new(
    tenant.id,
    email,
    password_hash,
    owner_name,      // ✅ Передается в User
    UserRole::Owner,
    language,
);
self.user_repo.create(&user).await?;
```

**Результат:** Если `owner_name` передан, он сохраняется в `users.display_name`.

---

### 4. Admin Users Endpoint ✅

```rust
// src/interfaces/http/admin_users.rs
SELECT 
    u.id::text,
    u.email,
    u.display_name as name,    -- ✅ Правильная колонка
    t.name as restaurant_name,
    COALESCE(u.language, 'ru') as language,
    u.created_at::text
FROM users u
JOIN tenants t ON u.tenant_id = t.id
ORDER BY u.created_at DESC
```

**Результат:** Запрос правильный, возвращает `display_name` как `name`.

---

## ❌ Корень проблемы

**Frontend НЕ отправляет поле `owner_name` при регистрации!**

### Тестирование регистрации:

```bash
# ❌ Так регистрируются пользователи сейчас (БЕЗ owner_name)
curl -X POST "https://ministerial-yetta-fodi999-c58d8823.koyeb.app/api/auth/register" \
  -H "Content-Type: application/json" \
  -d '{
    "email": "test@example.com",
    "password": "Password123",
    "restaurant_name": "Test Restaurant"
  }'

# ✅ Так ДОЛЖНЫ регистрироваться (С owner_name)
curl -X POST "https://ministerial-yetta-fodi999-c58d8823.koyeb.app/api/auth/register" \
  -H "Content-Type: application/json" \
  -d '{
    "email": "test@example.com",
    "password": "Password123",
    "restaurant_name": "Test Restaurant",
    "owner_name": "John Doe"
  }'
```

---

## ✅ Решение

### 1. Добавить поле в форму регистрации (Frontend)

```tsx
// components/auth/RegisterForm.tsx
interface RegisterFormData {
  email: string;
  password: string;
  restaurant_name: string;
  owner_name: string;      // ✅ Добавить это поле
  language?: string;
}

function RegisterForm() {
  const [formData, setFormData] = useState<RegisterFormData>({
    email: '',
    password: '',
    restaurant_name: '',
    owner_name: '',        // ✅ Добавить
    language: 'ru'
  });

  return (
    <form onSubmit={handleSubmit}>
      <div>
        <label>Email *</label>
        <input
          type="email"
          value={formData.email}
          onChange={e => setFormData({...formData, email: e.target.value})}
          required
        />
      </div>

      <div>
        <label>Пароль *</label>
        <input
          type="password"
          value={formData.password}
          onChange={e => setFormData({...formData, password: e.target.value})}
          required
        />
      </div>

      <div>
        <label>Название ресторана *</label>
        <input
          type="text"
          value={formData.restaurant_name}
          onChange={e => setFormData({...formData, restaurant_name: e.target.value})}
          required
        />
      </div>

      {/* ✅ ДОБАВИТЬ ЭТО ПОЛЕ */}
      <div>
        <label>Ваше имя *</label>
        <input
          type="text"
          value={formData.owner_name}
          onChange={e => setFormData({...formData, owner_name: e.target.value})}
          required
          placeholder="Иван Иванов"
        />
      </div>

      <div>
        <label>Язык</label>
        <select
          value={formData.language}
          onChange={e => setFormData({...formData, language: e.target.value})}
        >
          <option value="ru">Русский</option>
          <option value="en">English</option>
          <option value="pl">Polski</option>
          <option value="uk">Українська</option>
        </select>
      </div>

      <button type="submit">Зарегистрироваться</button>
    </form>
  );
}
```

---

### 2. API запрос с owner_name

```typescript
// services/auth.ts
export async function register(data: RegisterFormData) {
  const response = await fetch(
    'https://ministerial-yetta-fodi999-c58d8823.koyeb.app/api/auth/register',
    {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        email: data.email,
        password: data.password,
        restaurant_name: data.restaurant_name,
        owner_name: data.owner_name,    // ✅ Передать в API
        language: data.language || 'ru'
      })
    }
  );

  if (!response.ok) {
    const error = await response.json();
    throw new Error(error.details || 'Registration failed');
  }

  return response.json();
}
```

---

## 🔄 Миграция существующих пользователей (опционально)

Если нужно добавить имена существующим пользователям, можно создать админ endpoint:

### Backend: Update User Name

```rust
// src/interfaces/http/admin_users.rs

#[derive(Debug, Deserialize)]
pub struct UpdateUserNameRequest {
    pub display_name: String,
}

/// PATCH /api/admin/users/:id/name - Update user display name
pub async fn update_user_name(
    State(pool): State<PgPool>,
    Path(user_id): Path<String>,
    Json(req): Json<UpdateUserNameRequest>,
) -> Result<impl IntoResponse, StatusCode> {
    sqlx::query(
        "UPDATE users SET display_name = $1 WHERE id = $2"
    )
    .bind(&req.display_name)
    .bind(user_id)
    .execute(&pool)
    .await
    .map_err(|e| {
        tracing::error!("Database error updating user name: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(serde_json::json!({
        "message": "User name updated successfully"
    })))
}
```

### Frontend: Inline Edit

```tsx
// components/admin/UsersListTable.tsx
const handleNameEdit = async (userId: string, newName: string) => {
  const token = localStorage.getItem('admin_token');
  
  await fetch(
    `https://ministerial-yetta-fodi999-c58d8823.koyeb.app/api/admin/users/${userId}/name`,
    {
      method: 'PATCH',
      headers: {
        'Authorization': `Bearer ${token}`,
        'Content-Type': 'application/json'
      },
      body: JSON.stringify({ display_name: newName })
    }
  );
  
  fetchUsers(); // Refresh list
};
```

---

## 📋 Чек-лист исправления

### Must Have (обязательно)
- [ ] Добавить поле "Ваше имя" в форму регистрации
- [ ] Сделать поле `owner_name` обязательным (required)
- [ ] Передавать `owner_name` в API запросе регистрации
- [ ] Протестировать новую регистрацию
- [ ] Проверить, что имя появляется в админ панели

### Nice to Have (опционально)
- [ ] Создать endpoint для редактирования имен существующих пользователей
- [ ] Добавить inline editing в UsersListTable
- [ ] Массовое обновление имен через CSV импорт
- [ ] Валидация имени (минимум 2 символа)

---

## 🎯 Ожидаемый результат

После исправления:

```
Email                          Имя              Ресторан
test@example.com              John Doe          My Restaurant
user@restaurant.com           Jane Smith        Italian Place
owner@cafe.com                Иван Петров       Cafe Moscow
```

Вместо:

```
Email                          Имя              Ресторан
test@example.com              —                My Restaurant
user@restaurant.com           —                Italian Place
owner@cafe.com                —                Cafe Moscow
```

---

## 📝 Примечания

1. **Backend полностью готов** - никаких изменений не требуется
2. **Проблема только на фронтенде** - не отправляется `owner_name`
3. **Быстрое решение** - добавить одно поле в форму регистрации (5 минут)
4. **Существующие пользователи** - останутся без имен (если не сделать миграцию)

---

## ✅ Тестовый запрос

Проверь, что регистрация с именем работает:

```bash
curl -X POST "https://ministerial-yetta-fodi999-c58d8823.koyeb.app/api/auth/register" \
  -H "Content-Type: application/json" \
  -d '{
    "email": "test-with-name@example.com",
    "password": "TestPass123",
    "restaurant_name": "Named Restaurant",
    "owner_name": "Алексей Смирнов",
    "language": "ru"
  }'
```

Потом проверь в админ панели - имя должно появиться!

---

**🔧 Статус: Требуется обновление формы регистрации на frontend**
