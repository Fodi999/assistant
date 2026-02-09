# ✅ B2B SaaS ЭТАЛОН: Language Source = Backend

## 🎯 Реализовано

### Приоритет источника языка (BEST PRACTICE):

1. **user.language** (из БД) ✅ - основной источник
2. fallback → 'en' ✅ - в SQL через COALESCE
3. ~~query ?lang=ru~~ - не нужен (frontend не знает о языке)

## 📊 Что изменилось

### 1. AuthUser middleware получает language из БД

**До** (плохо):
```rust
pub struct AuthUser {
    pub user_id: UserId,
    pub tenant_id: TenantId,
    // ❌ Нет языка
}
```

**После** (правильно):
```rust
pub struct AuthUser {
    pub user_id: UserId,
    pub tenant_id: TenantId,
    pub language: Language,  // ✅ Из БД!
}

impl FromRequestParts for AuthUser {
    async fn from_request_parts(...) {
        // Получаем язык из БД при каждом запросе
        let language = sqlx::query_scalar::<_, String>(
            "SELECT language FROM users WHERE id = $1"
        )
        .bind(user_id.as_uuid())
        .fetch_optional(&pool)
        .await?
        .and_then(|lang| Language::from_str(&lang).ok())
        .unwrap_or(Language::En);  // Fallback
        
        Ok(AuthUser { user_id, tenant_id, language })
    }
}
```

### 2. SQL с COALESCE fallback (production-level)

**До** (ломается если нет перевода):
```sql
INNER JOIN catalog_ingredient_translations cit 
    ON cit.ingredient_id = ci.id AND cit.language = $3
```

**После** (graceful fallback):
```sql
LEFT JOIN catalog_ingredient_translations cit_user 
    ON cit_user.ingredient_id = ci.id AND cit_user.language = $3
LEFT JOIN catalog_ingredient_translations cit_en 
    ON cit_en.ingredient_id = ci.id AND cit_en.language = 'en'

SELECT 
    COALESCE(cit_user.name, cit_en.name, 'Unknown') as ingredient_name,
    COALESCE(cct_user.name, cct_en.name, 'Unknown') as category_name
```

**Преимущества:**
- ✅ Если нет перевода на `ru` → берется `en`
- ✅ Если нет `en` → показывается `'Unknown'`
- ✅ Никогда не ломается
- ✅ Идеально для production

### 3. HTTP Handler использует auth.language

**До**:
```rust
pub async fn list_products(
    auth: AuthUser,
) -> Result<...> {
    let language = Language::En;  // ❌ Хардкод!
}
```

**После**:
```rust
pub async fn list_products(
    auth: AuthUser,
) -> Result<...> {
    // ✅ Язык из AuthUser (из БД)
    service.list_products_with_details(
        auth.user_id,
        auth.tenant_id,
        auth.language  // 🎯 Backend = source of truth!
    )
}
```

## 🌍 Как это работает

### Frontend (тупой, как надо):

```typescript
// Просто делает запрос - ВСЁ!
const response = await fetch('/api/inventory/products', {
  headers: {
    'Authorization': `Bearer ${token}`
  }
});

const products = await response.json();
// Названия УЖЕ на языке пользователя!
```

**Никаких:**
- ❌ `?lang=ru` параметров
- ❌ localStorage для языка
- ❌ `Accept-Language` headers
- ❌ переводов на клиенте

### Backend (умный):

1. **Middleware** извлекает JWT
2. **AuthUser** загружает `language` из БД
3. **SQL** делает JOIN с правильным языком
4. **Response** содержит переведенные названия

## 📋 Поддерживаемые языки

| Язык | Code | Пример поиска |
|------|------|---------------|
| 🇬🇧 English | `en` | milk, tomato |
| 🇵🇱 Polski | `pl` | mleko, pomidor |
| 🇺🇦 Українська | `uk` | молоко, помідор |
| 🇷🇺 Русский | `ru` | молоко, помидор |

## 🔧 Технические детали

### Изменённые файлы:

1. **src/interfaces/http/middleware.rs**
   - Добавлено поле `language` в `AuthUser`
   - Добавлен SQL запрос для получения языка из БД
   - Fallback на `Language::En`

2. **src/interfaces/http/routes.rs**
   - Добавлен параметр `pool: PgPool` в `create_router()`
   - Обновлен middleware: `inject_jwt_and_pool()`
   - Pool добавляется в extensions для AuthUser

3. **src/main.rs**
   - Передается `repositories.pool.clone()` в router

4. **src/application/inventory.rs**
   - SQL обновлен на COALESCE fallback
   - 2x LEFT JOIN вместо 1x INNER JOIN
   - Graceful degradation

5. **src/interfaces/http/inventory.rs**
   - Использует `auth.language` вместо хардкода
   - Убраны комментарии TODO

6. **src/interfaces/http/menu_engineering.rs**
   - Обновлена деструктуризация AuthUser

7. **src/interfaces/http/dish.rs**
   - Обновлена деструктуризация AuthUser

### Performance considerations:

**Q: Не будет ли медленно делать SELECT language при каждом запросе?**

A: Нет, потому что:
1. SELECT по PRIMARY KEY (id) - моментально
2. users таблица маленькая (1 строка на пользователя)
3. PostgreSQL кеширует часто используемые строки
4. Альтернатива - добавить language в JWT (но требует ре-логин)

**Q: Можно ли кешировать?**

A: Да, можно добавить Redis:
```rust
// Сначала проверяем кеш
let language = cache.get(user_id).await
    .or_else(|| db.query(user_id).await);
```

Но для большинства B2B SaaS это преждевременная оптимизация.

## 🧪 Тестирование

### 1. Регистрация с языком

```bash
curl -X POST https://...koyeb.app/api/auth/register \
  -H "Content-Type: application/json" \
  -d '{
    "email": "test@test.com",
    "password": "Pass123!",
    "restaurant_name": "Test Restaurant"
  }'
# По умолчанию user.language = 'en' (из миграции)
```

### 2. Получение данных на языке пользователя

```bash
TOKEN="..." # ваш токен

# Просто делаем запрос - backend сам знает язык!
curl -H "Authorization: Bearer $TOKEN" \
  https://...koyeb.app/api/inventory/products

# Ответ содержит названия на языке пользователя:
[{
  "product": {
    "name": "Almonds",        // Если user.language = 'en'
    "category": "Nuts & Seeds"
  }
}]
```

### 3. Изменение языка пользователя (TODO: endpoint)

```sql
-- Пока через БД:
UPDATE users SET language = 'ru' WHERE email = 'test@test.com';
```

После этого все запросы будут возвращать русские названия!

## 🎯 Преимущества подхода

### ✅ Для Frontend:
- Простой API - один запрос
- Не нужно знать о языках
- Не нужно хранить настройки
- Меньше кода

### ✅ Для Backend:
- Single source of truth (БД)
- Централизованное управление
- Легко добавить новый язык
- Консистентность данных

### ✅ Для Бизнеса:
- Пользователь видит свой язык сразу
- Можно менять язык без ре-логина
- Админ может менять язык пользователю
- Аналитика по языкам

## 📈 Следующие шаги

### Опционально (можно добавить):

1. **Endpoint для смены языка**:
   ```rust
   PATCH /api/me/language
   { "language": "ru" }
   ```

2. **Language в JWT** (чтобы избежать SELECT):
   ```rust
   pub struct AccessTokenClaims {
       pub language: String,  // Кешируется в токене
   }
   ```

3. **Query parameter override** (для тестирования):
   ```rust
   GET /api/inventory/products?lang=pl
   // Временно переопределяет user.language
   ```

4. **Accept-Language fallback**:
   ```rust
   // Если user.language = NULL, берем из HTTP header
   let language = user.language
       .or_else(|| parse_accept_language_header());
   ```

Но для большинства случаев **текущая реализация идеальна**!

## 📝 Итог

✅ **Backend = source of truth** для языка  
✅ **user.language** из БД загружается в AuthUser  
✅ **COALESCE fallback** на английский в SQL  
✅ **Frontend тупой** - просто делает запросы  
✅ **Production ready** - graceful degradation  
✅ **B2B SaaS standard** - централизованное управление  

**Commit:** `df5b4ba` - "feat: implement B2B SaaS standard - language from AuthUser"  
**Status:** 🚀 Деплоится на Koyeb...

## 🔗 Связанные документы

- `I18N_IMPLEMENTATION_GUIDE.md` - полное руководство по i18n
- `QUERY_DTO_IMPLEMENTATION.md` - паттерн Query DTO
- `migrations/20240111000001_catalog_translations.sql` - translations tables
