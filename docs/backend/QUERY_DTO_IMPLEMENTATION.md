# Query DTO Pattern Implementation

## ✅ Реализовано: Вариант A - Backend Query DTO

### Что изменилось

#### До (проблема):
```json
{
  "id": "uuid",
  "catalog_ingredient_id": "uuid",  // ❌ Frontend должен делать второй запрос!
  "quantity": 5.5,
  "price_per_unit_cents": 450,
  "expires_at": "2026-03-15T23:59:59Z"
}
```

Frontend должен был:
1. `GET /api/inventory/products` - получить список
2. `GET /api/catalog/ingredients?id=...` - для КАЖДОГО продукта получить название
3. Склеить данные на клиенте

❌ **N+1 queries проблема**  
❌ Фронтенд "умный" (плохо)  
❌ Медленно на большом списке

#### После (решение):
```json
{
  "id": "uuid",
  "product": {
    "id": "uuid",
    "name": "Milk 3.2%",        // ✅ Название на языке пользователя
    "category": "Dairy",         // ✅ Категория
    "base_unit": "liter"         // ✅ Единица измерения
  },
  "quantity": 5.5,
  "price_per_unit_cents": 450,
  "expires_at": "2026-03-15T23:59:59Z"
}
```

✅ **Один запрос** - все данные  
✅ Фронтенд "тупой" (как надо!)  
✅ Быстро - JOIN на уровне БД  
✅ Domain остается чистым (DTO в application layer)

## Техническая реализация

### 1. Новые DTO в `src/application/inventory.rs`

```rust
/// Rich inventory view DTO (returned from query with JOINs)
#[derive(Debug, Clone, Serialize)]
pub struct InventoryView {
    pub id: uuid::Uuid,
    pub product: ProductInfo,
    pub quantity: f64,
    pub price_per_unit_cents: i64,
    pub expires_at: Option<OffsetDateTime>,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProductInfo {
    pub id: uuid::Uuid,
    pub name: String,       // Multilingual: en/pl/uk/ru
    pub category: String,   // From catalog_categories
    pub base_unit: String,  // gram/kilogram/liter/etc
}
```

### 2. Новый метод в `InventoryService`

```rust
/// Get inventory view with joined catalog ingredient and category data
pub async fn list_products_with_details(
    &self,
    user_id: UserId,
    tenant_id: TenantId,
    language: Language,
) -> AppResult<Vec<InventoryView>>
```

**SQL Query с двумя JOINами:**
```sql
SELECT 
    ip.id,
    ip.catalog_ingredient_id,
    ci.name_en as ingredient_name,  -- Выбирается по языку
    cc.name_en as category_name,    -- Тоже по языку
    ci.default_unit::TEXT as base_unit,
    ip.quantity,
    ip.price_per_unit_cents,
    ip.expires_at,
    ip.created_at,
    ip.updated_at
FROM inventory_products ip
INNER JOIN catalog_ingredients ci ON ip.catalog_ingredient_id = ci.id
LEFT JOIN catalog_categories cc ON ci.category_id = cc.id
WHERE ip.user_id = $1 AND ip.tenant_id = $2
ORDER BY ip.created_at DESC
```

### 3. Обновленный HTTP Handler

```rust
pub async fn list_products(
    State(service): State<InventoryService>,
    auth: AuthUser,
) -> Result<Json<Vec<InventoryView>>, AppError> {
    let language = Language::En;  // TODO: from user preferences
    
    let products = service
        .list_products_with_details(auth.user_id, auth.tenant_id, language)
        .await?;
    
    Ok(Json(products))
}
```

## Преимущества архитектуры

### ✅ Domain остается чистым
- `InventoryProduct` - pure domain entity
- Не знает о HTTP, JSON, презентации
- Бизнес-логика изолирована

### ✅ DTO в правильном месте
- `InventoryView` - в **application layer**
- Служит границей между domain и presentation
- Query DTO - специально для чтения

### ✅ Performance
- **1 SQL запрос** вместо N+1
- JOIN выполняется на уровне БД (быстро)
- Индексы работают эффективно
- Минимальный сетевой трафик

### ✅ Frontend упрощен
```typescript
// Frontend code
const products = await fetch('/api/inventory/products');
// Все данные уже есть! Просто рендерим:
products.map(p => (
  <div>
    <h3>{p.product.name}</h3>
    <span>{p.product.category}</span>
    <span>{p.quantity} {p.product.base_unit}</span>
  </div>
))
```

Никаких:
- ❌ дополнительных запросов
- ❌ кэширования на клиенте
- ❌ склейки данных
- ❌ состояний загрузки для каждого поля

## Multilingual Support

Метод поддерживает 4 языка:
- 🇵🇱 Polish (`Language::Pl`)
- 🇬🇧 English (`Language::En`)
- 🇺🇦 Ukrainian (`Language::Uk`)
- 🇷🇺 Russian (`Language::Ru`)

SQL динамически выбирает правильный столбец:
```rust
let lang_column = match language {
    Language::Pl => "ci.name_pl",
    Language::En => "ci.name_en",
    Language::Uk => "ci.name_uk",
    Language::Ru => "ci.name_ru",
};
```

## Следующие шаги

### TODO: User Language Preferences
Сейчас используется `Language::En` по умолчанию. Нужно:

1. **Вариант A**: Из JWT токена
```rust
// Add to JWT claims
pub struct Claims {
    pub user_id: UserId,
    pub tenant_id: TenantId,
    pub language: Language,  // <-- Добавить
}
```

2. **Вариант B**: Из HTTP заголовка
```rust
pub async fn list_products(
    State(service): State<InventoryService>,
    auth: AuthUser,
    TypedHeader(accept_language): TypedHeader<AcceptLanguage>,
) -> Result<...>
```

3. **Вариант C**: Из query parameter
```
GET /api/inventory/products?lang=pl
```

## Тестирование

После деплоя (3-5 минут):

```bash
# Получить токен
TOKEN=$(curl -s -X POST .../api/auth/register \
  -d '{"email":"test@test.com","password":"Pass123!","restaurant_name":"Test"}' \
  | jq -r '.access_token')

# Добавить продукт
INGREDIENT_ID=$(curl -s -H "Authorization: Bearer $TOKEN" \
  ".../api/catalog/ingredients?query=milk" | jq -r '.ingredients[0].id')

curl -X POST .../api/inventory/products \
  -H "Authorization: Bearer $TOKEN" \
  -d "{\"catalog_ingredient_id\":\"$INGREDIENT_ID\",\"price_per_unit_cents\":450,\"quantity\":5.5}"

# Получить богатый список
curl -H "Authorization: Bearer $TOKEN" .../api/inventory/products | jq .
```

Ожидаемый ответ:
```json
[
  {
    "id": "...",
    "product": {
      "id": "...",
      "name": "Milk",           // 🎯 Готово к отображению!
      "category": "Dairy",      // 🎯 Категория сразу!
      "base_unit": "liter"      // 🎯 Единица измерения!
    },
    "quantity": 5.5,
    "price_per_unit_cents": 450,
    "expires_at": "2026-03-15T23:59:59Z",
    "created_at": "...",
    "updated_at": "..."
  }
]
```

## Архитектурные принципы

### ✅ CQRS-lite
- **Command**: `add_product()`, `update_product()` - возвращают domain entities
- **Query**: `list_products_with_details()` - возвращает specialized DTO

### ✅ Clean Architecture
```
┌─────────────────────┐
│   HTTP Handler      │ <-- InventoryView (Query DTO)
├─────────────────────┤
│ Application Service │ <-- list_products_with_details()
├─────────────────────┤
│   Domain Layer      │ <-- InventoryProduct (pure entity)
├─────────────────────┤
│   Repository        │ <-- SQL + JOINs
└─────────────────────┘
```

### ✅ Performance-First
- JOIN на уровне БД (PostgreSQL оптимизирует)
- Один network round-trip
- Индексы на FK (catalog_ingredient_id, category_id)
- Pagination ready (добавить LIMIT/OFFSET)

## Commit
```bash
✅ e3b20c3 "feat: implement Query DTO pattern for inventory (returns product details with JOIN)"
```

## Deployment
- Auto-deploy на Koyeb через GitHub push
- ETA: 3-5 минут
- URL: https://ministerial-yetta-fodi999-c58d8823.koyeb.app
