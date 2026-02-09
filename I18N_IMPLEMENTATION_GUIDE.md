# Правильная реализация i18n для Query DTO

## ❌ Текущая проблема

### Что НЕ ТАК сейчас:

1. **Старая структура БД**: Используются колонки `name_pl, name_en, name_uk, name_ru`
2. **Language хардкодом**: `Language::En` в HTTP handler
3. **Нет translations table**: Правильной i18n структуры нет
4. **Frontend не знает язык пользователя**: Нужен source of truth

## ✅ ЭТАЛОННАЯ реализация (создана)

### 1. Миграция: `20240111000001_catalog_translations.sql`

```sql
-- Новая структура (правильная)
CREATE TABLE catalog_ingredient_translations (
    id UUID PRIMARY KEY,
    ingredient_id UUID REFERENCES catalog_ingredients(id),
    language TEXT CHECK (language IN ('en', 'pl', 'uk', 'ru')),
    name TEXT NOT NULL,
    UNIQUE (ingredient_id, language)
);

CREATE TABLE catalog_category_translations (
    id UUID PRIMARY KEY,
    category_id UUID REFERENCES catalog_categories(id),
    language TEXT CHECK (language IN ('en', 'pl', 'uk', 'ru')),
    name TEXT NOT NULL,
    UNIQUE (category_id, language)
);
```

### 2. Обновленный SQL (ЭТАЛОН)

```sql
SELECT 
    ip.id,
    ip.catalog_ingredient_id,
    cit.name as ingredient_name,        -- 🎯 Из translations!
    cct.name as category_name,          -- 🎯 Из translations!
    ci.default_unit::TEXT as base_unit,
    ip.quantity,
    ip.price_per_unit_cents,
    ip.expires_at,
    ip.created_at,
    ip.updated_at
FROM inventory_products ip
INNER JOIN catalog_ingredients ci 
    ON ip.catalog_ingredient_id = ci.id
INNER JOIN catalog_ingredient_translations cit 
    ON cit.ingredient_id = ci.id AND cit.language = $3  -- 🎯 Язык из параметра!
LEFT JOIN catalog_categories cc 
    ON ci.category_id = cc.id
LEFT JOIN catalog_category_translations cct 
    ON cct.category_id = cc.id AND cct.language = $3
WHERE ip.user_id = $1 AND ip.tenant_id = $2
ORDER BY ip.created_at DESC
```

**Параметры:**
- `$1` = `user_id`
- `$2` = `tenant_id`
- `$3` = `language` ('en' | 'pl' | 'uk' | 'ru')

### 3. Backend код (обновлен в `inventory.rs`)

```rust
pub async fn list_products_with_details(
    &self,
    user_id: UserId,
    tenant_id: TenantId,
    language: Language,  // 🎯 Язык передается как параметр
) -> AppResult<Vec<InventoryView>> {
    let lang_code = language.code();  // "en" | "pl" | "uk" | "ru"
    
    let rows = sqlx::query(QUERY)
        .bind(user_id.as_uuid())
        .bind(tenant_id.as_uuid())
        .bind(lang_code)  // 🎯 Передается в SQL
        .fetch_all(&self.pool)
        .await?;
    
    // ...
}
```

## 🔑 Источник языка - НЕ frontend!

### Вариант A: Из JWT токена (РЕКОМЕНДУЮ для production)

#### Шаг 1: Добавить language в JWT Claims

```rust
// src/infrastructure/security/jwt.rs
#[derive(Debug, Serialize, Deserialize)]
pub struct AccessTokenClaims {
    pub sub: String,          // user_id
    pub tenant_id: String,    // tenant_id
    pub language: String,     // 🎯 ДОБАВИТЬ
    pub iss: String,
    pub iat: i64,
    pub exp: i64,
}

impl JwtService {
    pub fn generate_access_token(
        &self,
        user_id: UserId,
        tenant_id: TenantId,
        language: Language,   // 🎯 ДОБАВИТЬ параметр
    ) -> AppResult<String> {
        let claims = AccessTokenClaims {
            sub: user_id.to_string(),
            tenant_id: tenant_id.to_string(),
            language: language.code().to_string(),  // 🎯 ДОБАВИТЬ
            iss: self.issuer.clone(),
            iat: now.unix_timestamp(),
            exp: expires_at.unix_timestamp(),
        };
        // ...
    }
}
```

#### Шаг 2: Обновить AuthUser middleware

```rust
// src/interfaces/http/middleware.rs
#[derive(Debug, Clone)]
pub struct AuthUser {
    pub user_id: UserId,
    pub tenant_id: TenantId,
    pub language: Language,  // 🎯 ДОБАВИТЬ
}

impl<S> FromRequestParts<S> for AuthUser {
    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let claims = jwt_service.verify_access_token(bearer.token())?;
        
        Ok(AuthUser {
            user_id: claims.user_id()?,
            tenant_id: claims.tenant_id()?,
            language: Language::from_str(&claims.language)?,  // 🎯 ДОБАВИТЬ
        })
    }
}
```

#### Шаг 3: Использовать в handler

```rust
// src/interfaces/http/inventory.rs
pub async fn list_products(
    State(service): State<InventoryService>,
    auth: AuthUser,
) -> Result<Json<Vec<InventoryView>>, AppError> {
    let products = service
        .list_products_with_details(
            auth.user_id, 
            auth.tenant_id, 
            auth.language  // 🎯 Из JWT!
        )
        .await?;
    
    Ok(Json(products))
}
```

**Плюсы:**
- ✅ Language всегда доступен
- ✅ Нет дополнительных запросов к БД
- ✅ Frontend не знает о языке (правильно!)

**Минусы:**
- ❌ Требует ре-логина всех пользователей
- ❌ Нужно обновить все места где генерируется JWT

### Вариант B: Из `GET /api/me` (быстрое решение)

```rust
// src/interfaces/http/user.rs
#[derive(Serialize)]
pub struct MeResponse {
    pub id: Uuid,
    pub email: String,
    pub restaurant_name: String,
    pub language: String,  // 🎯 Возвращаем язык
}

pub async fn me_handler(
    State(service): State<UserService>,
    auth: AuthUser,
) -> Result<Json<MeResponse>, AppError> {
    let user = service.get_user(auth.user_id).await?;
    
    Ok(Json(MeResponse {
        id: user.id.as_uuid(),
        email: user.email,
        restaurant_name: user.restaurant_name,
        language: user.language.code().to_string(),  // 🎯 Из БД
    }))
}
```

Frontend:
```typescript
// App initialization
const { data: me } = await fetch('/api/me');
localStorage.setItem('userLanguage', me.language);

// Later in requests
const lang = localStorage.getItem('userLanguage') || 'en';
```

**Плюсы:**
- ✅ Не требует изменения JWT
- ✅ Быстро реализуется
- ✅ Работает сразу

**Минусы:**
- ❌ Frontend должен знать о языке (не идеально)
- ❌ Дополнительный запрос при загрузке

### Вариант C: Query parameter (для тестирования)

```rust
pub async fn list_products(
    State(service): State<InventoryService>,
    auth: AuthUser,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<Vec<InventoryView>>, AppError> {
    let language = params
        .get("lang")
        .and_then(|l| Language::from_str(l).ok())
        .unwrap_or(Language::En);  // Default
    
    // ...
}
```

```
GET /api/inventory/products?lang=ru
```

**Плюсы:**
- ✅ Гибкость для тестирования
- ✅ Не требует изменений в auth

**Минусы:**
- ❌ Frontend должен передавать язык (плохо)
- ❌ Легко забыть передать параметр

## 🎯 Рекомендация

### Сейчас (quick win):
1. ✅ Запустить миграцию `20240111000001_catalog_translations.sql`
2. ✅ Использовать query parameter `?lang=...` для тестирования
3. ✅ Обновить SQL на эталонный (уже сделано)

### Потом (production ready):
1. Добавить `language` в JWT Claims
2. Обновить `AuthUser` middleware
3. Удалить старые колонки `name_pl/name_en/name_uk/name_ru`

## 📊 Проверка после миграции

```sql
-- 1. Проверить количество переводов
SELECT language, COUNT(*) 
FROM catalog_ingredient_translations 
GROUP BY language;

-- Ожидается:
-- en | 100
-- pl | 100
-- uk | 100
-- ru | 100

-- 2. Проверить поиск на русском
SELECT name 
FROM catalog_ingredient_translations 
WHERE language = 'ru' AND name ILIKE '%мол%';

-- Должен найти: "Молоко", "Молоко обезжиренное", etc.

-- 3. Проверить JOIN
SELECT 
    ci.id,
    cit.name as ingredient_name,
    cct.name as category_name
FROM catalog_ingredients ci
INNER JOIN catalog_ingredient_translations cit 
    ON cit.ingredient_id = ci.id AND cit.language = 'pl'
LEFT JOIN catalog_categories cc 
    ON ci.category_id = cc.id
LEFT JOIN catalog_category_translations cct 
    ON cct.category_id = cc.id AND cct.language = 'pl'
LIMIT 10;

-- Должен вернуть польские названия
```

## 🚀 Deployment Plan

1. **Create migration** ✅ (файл создан)
2. **Test locally** ⏳ (нужно запустить)
3. **Push to production** ⏳
4. **Verify data** ⏳
5. **Update frontend** ⏳

## ⚠️ ВАЖНО: Проверка `uk` языка

В enum `Language` есть `Uk`, проверьте что:
- ✅ В БД есть записи с `language = 'uk'`
- ✅ В CHECK constraint разрешен 'uk'
- ✅ В backend enum есть `Language::Uk`

Частая ошибка:
```sql
-- ❌ ПЛОХО
CHECK (language IN ('en', 'pl', 'ru'))  -- uk забыли!

-- ✅ ХОРОШО
CHECK (language IN ('en', 'pl', 'uk', 'ru'))
```

## 📝 Итого

✅ Миграция создана  
✅ SQL обновлен на эталонный  
✅ Backend использует `language.code()`  
⏳ Нужно запустить миграцию  
⏳ Нужно обновить HTTP handler для получения language  
⏳ Нужно добавить language в JWT (опционально)
