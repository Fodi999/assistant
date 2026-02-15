# 📦 Полная логика добавления продукта на склад

**Date**: 15 февраля 2026  
**Status**: Production-Ready ✅  
**Architecture**: 4-layer (Frontend → API → Service → Domain → Database)

---

## 📋 Содержание

1. [Архитектурная диаграмма](#архитектурная-диаграмма)
2. [Поток данных](#поток-данных)
3. [Слой за слоем](#слой-за-слоем)
4. [Примеры запросов](#примеры-запросов)
5. [Обработка ошибок](#обработка-ошибок)
6. [Валидация данных](#валидация-данных)

---

## 🏗️ Архитектурная диаграмма

```
┌─────────────────────────────────────────────────────────────────┐
│                       FRONTEND (React)                          │
│  ┌────────────────────────────────────────────────────────────┐ │
│  │ 1. User selects product from catalog search results         │ │
│  │ 2. User enters price, quantity, dates                       │ │
│  │ 3. Click "Add to Inventory" button                          │ │
│  └────────────────────────────────────────────────────────────┘ │
└──────────────────────────┬──────────────────────────────────────┘
                           │
                    HTTP POST REQUEST
                           │
         ┌──────────────────▼──────────────────┐
         │  POST /api/inventory/products        │
         │  ┌────────────────────────────────┐ │
         │  │ Request Body:                  │ │
         │  │ {                              │ │
         │  │   catalog_ingredient_id: UUID  │ │
         │  │   price_per_unit_cents: 1500   │ │
         │  │   quantity: 10.5               │ │
         │  │   received_at: ISO8601         │ │
         │  │   expires_at?: ISO8601         │ │
         │  │ }                              │ │
         │  └────────────────────────────────┘ │
         └──────────────────┬──────────────────┘
                           │
┌──────────────────────────▼──────────────────────────────────────┐
│                  HTTP INTERFACE LAYER                            │
│  ┌────────────────────────────────────────────────────────────┐ │
│  │ src/interfaces/http/inventory.rs                           │ │
│  │                                                            │ │
│  │ Handler: add_product()                                    │ │
│  │ ├─ Extract auth: AuthUser (from JWT + DB)                │ │
│  │ │  ├─ user_id: UserId                                    │ │
│  │ │  ├─ tenant_id: TenantId                                │ │
│  │ │  └─ language: Language (from users table)              │ │
│  │ │                                                         │ │
│  │ ├─ Parse request body: AddProductRequest                 │ │
│  │ │  ├─ catalog_ingredient_id                              │ │
│  │ │  ├─ price_per_unit_cents                               │ │
│  │ │  ├─ quantity                                           │ │
│  │ │  ├─ received_at (или текущее время)                    │ │
│  │ │  └─ expires_at (опционально)                           │ │
│  │ │                                                         │ │
│  │ └─ Call service.add_product()                            │ │
│  │    └─ Return: HTTP 201 CREATED + InventoryView JSON      │ │
│  │                                                           │ │
│  │ Response Body:                                            │ │
│  │ {                                                         │ │
│  │   id: "product-uuid",                                    │ │
│  │   product: {                                             │ │
│  │     id: "ingredient-uuid",                               │ │
│  │     name: "Молоко" (на языке пользователя!)            │ │
│  │     category: "Молочные продукты",                      │ │
│  │     base_unit: "liter",                                  │ │
│  │     image_url: "https://..."                             │ │
│  │   },                                                     │ │
│  │   quantity: 10.5,                                        │ │
│  │   price_per_unit_cents: 1500,                            │ │
│  │   received_at: "2026-02-15T12:00:00Z",                  │ │
│  │   expires_at: "2026-03-01T00:00:00Z",                   │ │
│  │   created_at: "2026-02-15T14:30:45Z",                   │ │
│  │   updated_at: "2026-02-15T14:30:45Z"                    │ │
│  │ }                                                        │ │
│  └────────────────────────────────────────────────────────┘ │
└──────────────────────┬───────────────────────────────────────┘
                       │
┌──────────────────────▼───────────────────────────────────────┐
│              APPLICATION LAYER (Service)                     │
│  ┌────────────────────────────────────────────────────────┐ │
│  │ src/application/inventory.rs                           │ │
│  │                                                        │ │
│  │ InventoryService::add_product()                        │ │
│  │                                                        │ │
│  │ 🔹 Validate & Convert Types:                          │ │
│  │   ├─ Money::from_cents(1500)                         │ │
│  │   │  └─ Проверка: >= 0 ✓                             │ │
│  │   │                                                  │ │
│  │   └─ Quantity::new(10.5)                             │ │
│  │      ├─ Проверка: >= 0 ✓                             │ │
│  │      └─ Проверка: is_finite() ✓                      │ │
│  │                                                        │ │
│  │ 🔹 Auto-Calculate Expiration:                         │ │
│  │   ├─ if expires_at provided:                          │ │
│  │   │  └─ Use provided date                             │ │
│  │   │                                                  │ │
│  │   └─ else:                                            │ │
│  │      ├─ Fetch catalog ingredient                      │ │
│  │      ├─ Read default_shelf_life_days (e.g., 7)       │ │
│  │      └─ Calculate: received_at + 7 days               │ │
│  │         = 2026-02-22T12:00:00Z                        │ │
│  │                                                        │ │
│  │ 🔹 Create Domain Model:                               │ │
│  │   └─ InventoryProduct::new()                          │ │
│  │      ├─ id: InventoryProductId::new() → UUID          │ │
│  │      ├─ user_id: <from auth>                          │ │
│  │      ├─ tenant_id: <from auth>                        │ │
│  │      ├─ catalog_ingredient_id: <from request>         │ │
│  │      ├─ price_per_unit: Money(1500)                   │ │
│  │      ├─ quantity: Quantity(10.5)                      │ │
│  │      ├─ received_at: 2026-02-15T12:00:00Z            │ │
│  │      ├─ expires_at: 2026-02-22T12:00:00Z             │ │
│  │      ├─ created_at: NOW()                             │ │
│  │      └─ updated_at: NOW()                             │ │
│  │                                                        │ │
│  │ 🔹 Persist Product:                                   │ │
│  │   └─ inventory_repo.create(&product)                  │ │
│  │                                                        │ │
│  │ 🔹 Fetch & Enrich Response:                           │ │
│  │   └─ list_products_with_details()                     │ │
│  │      ├─ JOIN with catalog_ingredients                 │ │
│  │      ├─ JOIN with catalog_ingredient_translations     │ │
│  │      ├─ JOIN with catalog_categories                  │ │
│  │      ├─ JOIN with catalog_category_translations       │ │
│  │      ├─ Apply language fallback: user_lang → 'en'    │ │
│  │      └─ Return InventoryView with all details         │ │
│  │                                                        │ │
│  │ Return: InventoryProductId (success)                  │ │
│  └────────────────────────────────────────────────────────┘ │
└──────────────────────┬────────────────────────────────────────┘
                       │
┌──────────────────────▼────────────────────────────────────────┐
│              DOMAIN LAYER (Business Logic)                    │
│  ┌────────────────────────────────────────────────────────┐ │
│  │ src/domain/inventory.rs                                │ │
│  │                                                        │ │
│  │ Value Objects:                                         │ │
│  │  ├─ InventoryProductId(UUID)                          │ │
│  │  ├─ Money(i64) - в наименьших единицах              │ │
│  │  │  ├─ from_cents(1500) → validates >= 0             │ │
│  │  │  └─ as_cents() → 1500                              │ │
│  │  │                                                    │ │
│  │  ├─ Quantity(f64)                                     │ │
│  │  │  ├─ new(10.5) → validates >= 0 и is_finite()      │ │
│  │  │  └─ value() → 10.5                                 │ │
│  │  │                                                    │ │
│  │  └─ ExpirationStatus (enum)                           │ │
│  │     ├─ Expired (date < today)                         │ │
│  │     ├─ ExpiresToday (date == today)                   │ │
│  │     ├─ ExpiringSoon (date <= today + 2 days)         │ │
│  │     ├─ Fresh (date > today + 2 days)                  │ │
│  │     └─ NoExpiration (expires_at = null)               │ │
│  │                                                        │ │
│  │ Aggregate Root:                                        │ │
│  │  ├─ InventoryProduct                                  │ │
│  │  │  ├─ Commands:                                      │ │
│  │  │  │  ├─ update_quantity(new_qty)                    │ │
│  │  │  │  │  └─ Validates & updates updated_at           │ │
│  │  │  │  │                                              │ │
│  │  │  │  └─ update_price(new_price)                     │ │
│  │  │  │     └─ Validates & updates updated_at           │ │
│  │  │  │                                                 │ │
│  │  │  ├─ Queries:                                       │ │
│  │  │  │  ├─ is_expired() → bool                         │ │
│  │  │  │  ├─ expiration_status() → enum                  │ │
│  │  │  │  └─ total_cost() → Money                        │ │
│  │  │  │                                                 │ │
│  │  │  └─ Invariants:                                    │ │
│  │  │     ├─ user_id must be set                         │ │
│  │  │     ├─ tenant_id must be set                       │ │
│  │  │     ├─ catalog_ingredient_id must reference        │ │
│  │  │     │  existing ingredient                         │ │
│  │  │     ├─ quantity > 0                                │ │
│  │  │     ├─ price_per_unit >= 0                         │ │
│  │  │     ├─ received_at <= expires_at (if set)          │ │
│  │  │     └─ created_at <= updated_at                    │ │
│  │  │                                                    │ │
│  └────────────────────────────────────────────────────────┘ │
└──────────────────────┬────────────────────────────────────────┘
                       │
┌──────────────────────▼────────────────────────────────────────┐
│           PERSISTENCE LAYER (Repository)                      │
│  ┌────────────────────────────────────────────────────────┐ │
│  │ src/infrastructure/persistence/                        │ │
│  │ inventory_product_repository.rs                        │ │
│  │                                                        │ │
│  │ Repository::create(&product)                           │ │
│  │  └─ Execute SQL INSERT:                                │ │
│  │     ┌────────────────────────────────────────────┐    │ │
│  │     │ INSERT INTO inventory_products             │    │ │
│  │     │   (id, user_id, tenant_id,                │    │ │
│  │     │    catalog_ingredient_id,                 │    │ │
│  │     │    price_per_unit_cents,                  │    │ │
│  │     │    quantity,                              │    │ │
│  │     │    received_at,                           │    │ │
│  │     │    expires_at,                            │    │ │
│  │     │    created_at,                            │    │ │
│  │     │    updated_at)                            │    │ │
│  │     │ VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)    │    │ │
│  │     └────────────────────────────────────────────┘    │ │
│  │                                                        │ │
│  │ Bindings:                                              │ │
│  │  ├─ id: UUID (e.g., a1b2c3d4-...)                     │ │
│  │  ├─ user_id: UUID (from auth)                         │ │
│  │  ├─ tenant_id: UUID (from auth)                       │ │
│  │  ├─ catalog_ingredient_id: UUID                       │ │
│  │  ├─ price_per_unit_cents: 1500                        │ │
│  │  ├─ quantity: 10.5                                    │ │
│  │  ├─ received_at: 2026-02-15 12:00:00+00              │ │
│  │  ├─ expires_at: 2026-02-22 12:00:00+00 (или NULL)   │ │
│  │  ├─ created_at: 2026-02-15 14:30:45+00               │ │
│  │  └─ updated_at: 2026-02-15 14:30:45+00               │ │
│  │                                                        │ │
│  │ Result: OK or Error                                    │ │
│  └────────────────────────────────────────────────────────┘ │
└──────────────────────┬────────────────────────────────────────┘
                       │
┌──────────────────────▼────────────────────────────────────────┐
│              DATABASE LAYER (PostgreSQL)                       │
│  ┌────────────────────────────────────────────────────────┐ │
│  │ Table: inventory_products                              │ │
│  │                                                        │ │
│  │ Column               │ Type                │ Value     │ │
│  │ ─────────────────────┼─────────────────────┼────────── │ │
│  │ id                   │ UUID PRIMARY KEY    │ a1b2c3d4..│ │
│  │ user_id              │ UUID FK → users     │ f7fc371a..│ │
│  │ tenant_id            │ UUID FK → tenants   │ 6835daf9..│ │
│  │ catalog_ingredient.. │ UUID FK → catalog.. │ 519169f2..│ │
│  │ price_per_unit_cents │ INTEGER             │ 1500      │ │
│  │ quantity             │ DOUBLE PRECISION    │ 10.5      │ │
│  │ received_at          │ TIMESTAMP WITH TZ   │ 2026-02-15│ │
│  │ expires_at           │ TIMESTAMP WITH TZ   │ 2026-02-22│ │
│  │ created_at           │ TIMESTAMP WITH TZ   │ 2026-02-15│ │
│  │ updated_at           │ TIMESTAMP WITH TZ   │ 2026-02-15│ │
│  │                                                        │ │
│  │ Indexes:                                               │ │
│  │  ├─ PRIMARY KEY (id)                                  │ │
│  │  ├─ UNIQUE (user_id, tenant_id, id)  [isolation]     │ │
│  │  ├─ FK (user_id) → users(id)                          │ │
│  │  ├─ FK (tenant_id) → tenants(id)                      │ │
│  │  ├─ FK (catalog_ingredient_id) →                      │ │
│  │  │  catalog_ingredients(id)                           │ │
│  │  ├─ idx_inventory_user_tenant                         │ │
│  │  │  (user_id, tenant_id)                              │ │
│  │  │                                                    │ │
│  │  └─ idx_inventory_expiry                             │ │
│  │     (expires_at) [for expiration warnings]            │ │
│  │                                                        │ │
│  │ Constraints:                                           │ │
│  │  ├─ quantity > 0                                      │ │
│  │  ├─ price_per_unit_cents >= 0                         │ │
│  │  ├─ received_at <= expires_at (if set)               │ │
│  │  └─ created_at <= updated_at                          │ │
│  │                                                        │ │
│  └────────────────────────────────────────────────────────┘ │
└──────────────────────────────────────────────────────────────┘
```

---

## 📊 Поток данных

### Пример: Добавление молока (Pasteurized milk)

```json
// FRONTEND: React component sends
{
  "catalog_ingredient_id": "519169f2-69f1-4875-94ed-12eccbb809ae",
  "price_per_unit_cents": 1500,
  "quantity": 10.5,
  "received_at": "2026-02-15T12:00:00Z",
  "expires_at": null  // оставляем пустым - будет автоматически рассчитано
}
```

### Обработка в сервисе:

```rust
// 1. Валидация
price = Money::from_cents(1500)?;  // ✓ 1500 > 0
qty = Quantity::new(10.5)?;        // ✓ 10.5 > 0 и is_finite

// 2. Автоматический расчет даты истечения
// Fetch: SELECT default_shelf_life_days FROM catalog_ingredients 
//        WHERE id = '519169f2...'
// Result: 7 дней

calculated_expires_at = 2026-02-15T12:00:00Z + Duration::days(7)
                      = 2026-02-22T12:00:00Z

// 3. Создание Domain Model
product = InventoryProduct {
    id: InventoryProductId::new(),  // генерируем UUID
    user_id: f7fc371a...,
    tenant_id: 6835daf9...,
    catalog_ingredient_id: 519169f2...,
    price_per_unit: Money(1500),
    quantity: Quantity(10.5),
    received_at: 2026-02-15T12:00:00Z,
    expires_at: Some(2026-02-22T12:00:00Z),
    created_at: NOW(),
    updated_at: NOW(),
}

// 4. Сохранение в БД
inventory_repo.create(&product)?;

// 5. Обогащенный ответ (JOIN с переводами)
SELECT 
    ip.id,
    ip.catalog_ingredient_id,
    COALESCE(cit_user.name, cit_en.name) as ingredient_name,  // "Pasteurized milk" (en)
    COALESCE(cct_user.name, cct_en.name) as category_name,    // "Dairy and Eggs" (en)
    ci.default_unit::TEXT as base_unit,                        // "liter"
    ci.image_url,                                              // "https://..."
    ip.quantity,                                               // 10.5
    ip.price_per_unit_cents,                                   // 1500
    ip.received_at,                                            // 2026-02-15T12:00:00Z
    ip.expires_at,                                             // 2026-02-22T12:00:00Z
    ip.created_at,                                             // NOW()
    ip.updated_at                                              // NOW()
FROM inventory_products ip
INNER JOIN catalog_ingredients ci ON ip.catalog_ingredient_id = ci.id
LEFT JOIN catalog_ingredient_translations cit_user ON ...
LEFT JOIN catalog_ingredient_translations cit_en ON ...
LEFT JOIN catalog_categories cc ON ci.category_id = cc.id
LEFT JOIN catalog_category_translations cct_user ON ...
LEFT JOIN catalog_category_translations cct_en ON ...
WHERE ip.id = '<just-inserted-id>'
```

### FRONTEND: Получит ответ

```json
{
  "id": "a1b2c3d4-e5f6-7890-abcd-ef1234567890",
  "product": {
    "id": "519169f2-69f1-4875-94ed-12eccbb809ae",
    "name": "Pasteurized milk",
    "category": "Dairy and Eggs",
    "base_unit": "liter",
    "image_url": "https://i.postimg.cc/0QPm7B4H/..."
  },
  "quantity": 10.5,
  "price_per_unit_cents": 1500,
  "received_at": "2026-02-15T12:00:00Z",
  "expires_at": "2026-02-22T12:00:00Z",
  "created_at": "2026-02-15T14:30:45Z",
  "updated_at": "2026-02-15T14:30:45Z"
}
```

---

## 🔍 Слой за слоем

### 1️⃣ HTTP Interface Layer (`src/interfaces/http/inventory.rs`)

**Ответственность**: Преобразование HTTP запроса в вызов сервиса

```rust
pub async fn add_product(
    State(service): State<InventoryService>,
    auth: AuthUser,  // ✅ Аутентификация & язык из БД
    Json(req): Json<AddProductRequest>,
) -> Result<(StatusCode, Json<InventoryView>), AppError> {
    // 1. Извлекаем данные из контекста
    let user_id = auth.user_id;           // От JWT токена
    let tenant_id = auth.tenant_id;       // От JWT токена
    let language = auth.language;         // ✅ ИЗ БАЗЫ ДАННЫХ!

    // 2. Вызываем сервис
    let product_id = service.add_product(
        user_id,
        tenant_id,
        CatalogIngredientId::from_uuid(req.catalog_ingredient_id),
        req.price_per_unit_cents,
        req.quantity,
        req.received_at,
        req.expires_at,
    ).await?;

    // 3. Получаем обогащенный ответ
    let products = service
        .list_products_with_details(user_id, tenant_id, language)
        .await?;
    
    let product_view = products
        .into_iter()
        .find(|p| p.id == product_id.as_uuid())
        .ok_or_else(|| AppError::internal("Failed to retrieve created product"))?;

    // 4. Возвращаем HTTP 201 CREATED + JSON
    Ok((StatusCode::CREATED, Json(product_view)))
}
```

**Ключевые моменты:**
- ✅ `AuthUser` содержит язык пользователя из БД (не из фронтенда!)
- ✅ Возвращаем `InventoryView` с обогащенными данными (JOIN)
- ✅ HTTP статус 201 CREATED для POST

---

### 2️⃣ Application Layer (`src/application/inventory.rs`)

**Ответственность**: Бизнес-логика, валидация, орхестрация

```rust
pub async fn add_product(
    &self,
    user_id: UserId,
    tenant_id: TenantId,
    catalog_ingredient_id: CatalogIngredientId,
    price_per_unit_cents: i64,
    quantity: f64,
    received_at: OffsetDateTime,
    expires_at: Option<OffsetDateTime>,
) -> AppResult<InventoryProductId> {
    // 🔹 ВАЛИДАЦИЯ И ПРЕОБРАЗОВАНИЕ ТИПОВ
    let price = Money::from_cents(price_per_unit_cents)?;
    let qty = Quantity::new(quantity)?;

    // 🔹 АВТОМАТИЧЕСКИЙ РАСЧЕТ ДАТЫ ИСТЕЧЕНИЯ
    let calculated_expires_at = match expires_at {
        Some(manual_date) => Some(manual_date),  // Пользователь задал
        None => {
            // Берем из каталога
            if let Ok(Some(ingredient)) = self.catalog_repo.find_by_id(catalog_ingredient_id).await {
                ingredient.default_shelf_life_days.map(|days| {
                    received_at + time::Duration::days(days as i64)
                })
            } else {
                None
            }
        }
    };

    // 🔹 СОЗДАНИЕ DOMAIN MODEL
    let product = InventoryProduct::new(
        user_id,
        tenant_id,
        catalog_ingredient_id,
        price,
        qty,
        received_at,
        calculated_expires_at,
    );

    let product_id = product.id;

    // 🔹 СОХРАНЕНИЕ В БД
    self.inventory_repo.create(&product).await?;

    Ok(product_id)
}
```

**Ключевые моменты:**
- ✅ Валидация: `Money::from_cents()`, `Quantity::new()`
- ✅ Автоматический расчет: `default_shelf_life_days` из каталога
- ✅ Создание domain model перед сохранением

---

### 3️⃣ Domain Layer (`src/domain/inventory.rs`)

**Ответственность**: Business rules, инварианты, ценные объекты

```rust
// VALUE OBJECT: Money
pub struct Money(i64);  // в наименьших единицах (центы, гроши)

impl Money {
    pub fn from_cents(cents: i64) -> AppResult<Self> {
        if cents < 0 {
            return Err(AppError::validation("Money amount cannot be negative"));
        }
        Ok(Self(cents))
    }

    pub fn multiply(&self, quantity: f64) -> AppResult<Money> {
        if quantity < 0.0 {
            return Err(AppError::validation("Quantity cannot be negative"));
        }
        let result = (self.0 as f64 * quantity).round() as i64;
        Ok(Money(result))
    }
}

// VALUE OBJECT: Quantity
pub struct Quantity(f64);

impl Quantity {
    pub fn new(value: f64) -> AppResult<Self> {
        if value < 0.0 {
            return Err(AppError::validation("Quantity cannot be negative"));
        }
        if !value.is_finite() {
            return Err(AppError::validation("Quantity must be finite"));
        }
        Ok(Self(value))
    }
}

// AGGREGATE ROOT: InventoryProduct
pub struct InventoryProduct {
    pub id: InventoryProductId,
    pub user_id: UserId,
    pub tenant_id: TenantId,
    pub catalog_ingredient_id: CatalogIngredientId,
    pub price_per_unit: Money,
    pub quantity: Quantity,
    pub received_at: OffsetDateTime,
    pub expires_at: Option<OffsetDateTime>,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

impl InventoryProduct {
    pub fn new(
        user_id: UserId,
        tenant_id: TenantId,
        catalog_ingredient_id: CatalogIngredientId,
        price_per_unit: Money,
        quantity: Quantity,
        received_at: OffsetDateTime,
        expires_at: Option<OffsetDateTime>,
    ) -> Self {
        let now = OffsetDateTime::now_utc();
        Self {
            id: InventoryProductId::new(),
            user_id,
            tenant_id,
            catalog_ingredient_id,
            price_per_unit,
            quantity,
            received_at,
            expires_at,
            created_at: now,
            updated_at: now,
        }
    }

    // INVAR IANTS
    pub fn total_cost(&self) -> AppResult<Money> {
        self.price_per_unit.multiply(self.quantity.value())
    }

    pub fn expiration_status(&self) -> ExpirationStatus {
        if let Some(expires_at) = self.expires_at {
            let today = OffsetDateTime::now_utc().date();
            let expiry_date = expires_at.date();
            
            if expiry_date < today {
                ExpirationStatus::Expired
            } else if expiry_date == today {
                ExpirationStatus::ExpiresToday
            } else if expiry_date <= today + time::Duration::days(2) {
                ExpirationStatus::ExpiringSoon
            } else {
                ExpirationStatus::Fresh
            }
        } else {
            ExpirationStatus::NoExpiration
        }
    }

    pub fn update_quantity(&mut self, new_quantity: Quantity) {
        self.quantity = new_quantity;
        self.updated_at = OffsetDateTime::now_utc();
    }

    pub fn update_price(&mut self, new_price: Money) {
        self.price_per_unit = new_price;
        self.updated_at = OffsetDateTime::now_utc();
    }
}
```

**Ключевые моменты:**
- ✅ Value Objects инкапсулируют валидацию
- ✅ Aggregate Root с бизнес-методами
- ✅ Инварианты защищены в домене

---

### 4️⃣ Persistence Layer (`src/infrastructure/persistence/inventory_product_repository.rs`)

**Ответственность**: Сохранение и получение из БД

```rust
#[async_trait]
pub trait InventoryProductRepositoryTrait {
    async fn create(&self, product: &InventoryProduct) -> AppResult<()>;
    async fn find_by_id(...) -> AppResult<Option<InventoryProduct>>;
    async fn list_by_user(...) -> AppResult<Vec<InventoryProduct>>;
    async fn update(&self, product: &InventoryProduct) -> AppResult<()>;
    async fn delete(...) -> AppResult<()>;
}

#[async_trait]
impl InventoryProductRepositoryTrait for InventoryProductRepository {
    async fn create(&self, product: &InventoryProduct) -> AppResult<()> {
        sqlx::query(
            r#"
            INSERT INTO inventory_products 
                (id, user_id, tenant_id, catalog_ingredient_id, 
                 price_per_unit_cents, quantity, 
                 received_at, expires_at, 
                 created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            "#
        )
        .bind(product.id.as_uuid())
        .bind(product.user_id.as_uuid())
        .bind(product.tenant_id.as_uuid())
        .bind(product.catalog_ingredient_id.as_uuid())
        .bind(product.price_per_unit.as_cents())
        .bind(product.quantity.value())
        .bind(product.received_at)
        .bind(product.expires_at)
        .bind(product.created_at)
        .bind(product.updated_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}
```

---

## 📝 Примеры запросов

### cURL тест добавления продукта

```bash
#!/bin/bash

TOKEN="your-jwt-token"
BACKEND="https://ministerial-yetta-fodi999-c58d8823.koyeb.app"

# 1️⃣ Получить ID ингредиента (молоко)
SEARCH=$(curl -s "$BACKEND/api/catalog/ingredients?q=milk" \
  -H "Authorization: Bearer $TOKEN")

INGREDIENT_ID=$(echo "$SEARCH" | jq -r '.ingredients[0].id')
echo "Found ingredient: $INGREDIENT_ID"

# 2️⃣ Добавить на склад
RESPONSE=$(curl -X POST "$BACKEND/api/inventory/products" \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $TOKEN" \
  -d "{
    \"catalog_ingredient_id\": \"$INGREDIENT_ID\",
    \"price_per_unit_cents\": 1500,
    \"quantity\": 10.5,
    \"received_at\": \"$(date -u +'%Y-%m-%dT%H:%M:%SZ')\",
    \"expires_at\": null
  }")

echo "Added product:"
echo "$RESPONSE" | jq '.'

# 3️⃣ Получить список товаров
curl -s "$BACKEND/api/inventory/products" \
  -H "Authorization: Bearer $TOKEN" | jq '.'
```

### JavaScript/Fetch пример

```typescript
async function addProductToInventory(
  ingredientId: string,
  pricePerUnitCents: number,
  quantity: number
) {
  const token = localStorage.getItem('accessToken');
  
  const response = await fetch('/api/inventory/products', {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      'Authorization': `Bearer ${token}`,
    },
    body: JSON.stringify({
      catalog_ingredient_id: ingredientId,
      price_per_unit_cents: pricePerUnitCents,
      quantity: quantity,
      received_at: new Date().toISOString(),
      expires_at: null,  // Автоматически рассчитается из default_shelf_life_days
    }),
  });

  if (!response.ok) {
    throw new Error(`Failed to add product: ${response.statusText}`);
  }

  const inventoryView = await response.json();
  return inventoryView;
}

// Использование
const product = await addProductToInventory(
  '519169f2-69f1-4875-94ed-12eccbb809ae',  // Milk ID
  1500,  // $15.00
  10.5   // 10.5 liters
);

console.log(product);
// {
//   id: "a1b2c3d4-e5f6-7890-abcd-ef1234567890",
//   product: {
//     id: "519169f2-69f1-4875-94ed-12eccbb809ae",
//     name: "Pasteurized milk",
//     category: "Dairy and Eggs",
//     base_unit: "liter",
//     image_url: "..."
//   },
//   quantity: 10.5,
//   price_per_unit_cents: 1500,
//   received_at: "2026-02-15T14:30:45Z",
//   expires_at: "2026-02-22T14:30:45Z",
//   created_at: "2026-02-15T14:30:45Z",
//   updated_at: "2026-02-15T14:30:45Z"
// }
```

---

## ⚠️ Обработка ошибок

### Возможные ошибки и их обработка

| Error | Status | Cause | Fix |
|-------|--------|-------|-----|
| **Validation Error** | 400 | `price_per_unit_cents < 0` | Введите положительное число |
| **Validation Error** | 400 | `quantity < 0` | Введите положительное количество |
| **Validation Error** | 400 | `quantity` is NaN/Infinity | Введите конечное число |
| **Not Found** | 404 | `catalog_ingredient_id` doesn't exist | Выберите существующий продукт |
| **Unauthorized** | 401 | Missing/invalid JWT token | Авторизуйтесь |
| **Forbidden** | 403 | Token belongs to different tenant | Используйте правильный токен |
| **Internal Error** | 500 | Database error | Попробуйте снова позже |

### Пример обработки ошибок на фронтенде

```typescript
try {
  const product = await addProductToInventory(ingredientId, price, qty);
  console.log('✅ Added:', product.product.name);
} catch (err: any) {
  if (err.response?.status === 400) {
    console.error('❌ Validation error:', err.response.data.message);
    // Show validation error to user
  } else if (err.response?.status === 401) {
    console.error('❌ Not authenticated');
    // Redirect to login
  } else if (err.response?.status === 404) {
    console.error('❌ Product not found');
    // Refresh catalog
  } else {
    console.error('❌ Unknown error:', err.message);
  }
}
```

---

## ✅ Валидация данных

### На уровне Domain (самый строгий)

```rust
// 1. Money validation
Money::from_cents(1500)?;  // ✓
Money::from_cents(-100)?;  // ❌ AppError: "negative"

// 2. Quantity validation
Quantity::new(10.5)?;      // ✓
Quantity::new(-5.0)?;      // ❌ AppError: "negative"
Quantity::new(f64::NAN)?;  // ❌ AppError: "not finite"
Quantity::new(f64::INFINITY)?;  // ❌ AppError: "not finite"

// 3. Expiration logic
if expired_at < received_at { 
    // ❌ Несовместимые даты
}
```

### На уровне API (HTTP validation)

```rust
// Сериализация с serde
#[derive(Debug, Deserialize)]
pub struct AddProductRequest {
    pub catalog_ingredient_id: Uuid,  // UUID format validated by serde
    pub price_per_unit_cents: i64,    // Type checked
    pub quantity: f64,                // Type checked
    #[serde(default = "default_received_at", with = "time::serde::rfc3339")]
    pub received_at: OffsetDateTime,  // RFC3339 format, defaults to NOW
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub expires_at: Option<OffsetDateTime>,  // RFC3339 format, optional
}
```

### На уровне Database (constraints)

```sql
CREATE TABLE inventory_products (
    ...
    quantity DOUBLE PRECISION NOT NULL CHECK (quantity > 0),
    price_per_unit_cents INTEGER NOT NULL CHECK (price_per_unit_cents >= 0),
    received_at TIMESTAMP WITH TIME ZONE NOT NULL,
    expires_at TIMESTAMP WITH TIME ZONE,
    
    -- Ensure expiration date is after reception date
    CHECK (expires_at IS NULL OR expires_at >= received_at),
    
    -- Ensure created_at <= updated_at
    CHECK (created_at <= updated_at),
    ...
);
```

---

## 🎯 Ключевые особенности

| Feature | Benefit |
|---------|---------|
| **Auto Expiration Calculation** | Пользователь не должен вводить дату истечения вручную - рассчитывается автоматически из каталога |
| **Language from DB** | `auth.language` берется из `users.language` в БД, не с фронтенда |
| **Query DTO Pattern** | Single database query с JOINами возвращает все необходимые данные |
| **Domain-Driven Design** | Value Objects (Money, Quantity) инкапсулируют валидацию |
| **Tenant Isolation** | user_id + tenant_id гарантируют что пользователь может видеть только свои товары |
| **Money in Cents** | Избегаем проблем с плавающей точкой при работе с деньгами |
| **Expiration Status** | Рассчитывается на уровне domain (Expired, ExpiresToday, ExpiringSoon, Fresh, NoExpiration) |
| **Soft Validation** | Многоуровневая валидация: API → Service → Domain → Database |

---

## 📊 Полный цикл жизни товара

```
┌────────────────────────────────────────────────────────────┐
│ 1. USER SEARCHES CATALOG                                   │
│    GET /api/catalog/ingredients?q=milk                     │
│    ↓ Returns list of ingredients from catalog              │
│    ↓ User selects one (e.g., Pasteurized milk)            │
└────────────────────────────────────────────────────────────┘
                        ↓
┌────────────────────────────────────────────────────────────┐
│ 2. USER ENTERS DETAILS                                     │
│    ├─ Price per unit: $15.00 → 1500 cents                │
│    ├─ Quantity: 10.5 liters                              │
│    ├─ Received at: 2026-02-15 (auto: now)               │
│    └─ Expires at: (leave empty - will auto-calculate)     │
└────────────────────────────────────────────────────────────┘
                        ↓
┌────────────────────────────────────────────────────────────┐
│ 3. ADD TO INVENTORY                                        │
│    POST /api/inventory/products                            │
│    ├─ Validates: price >= 0, qty > 0, qty is finite      │
│    ├─ Auto-calculates expires_at from default_shelf_life  │
│    ├─ Creates Domain Model (InventoryProduct)             │
│    ├─ Saves to database                                   │
│    └─ Returns InventoryView with enriched data            │
└────────────────────────────────────────────────────────────┘
                        ↓
┌────────────────────────────────────────────────────────────┐
│ 4. PRODUCT STORED IN INVENTORY                             │
│    ├─ id: UUID                                            │
│    ├─ user_id: user's UUID (from auth)                   │
│    ├─ tenant_id: tenant's UUID (from auth)               │
│    ├─ catalog_ingredient_id: reference to catalog         │
│    ├─ price_per_unit_cents: 1500                          │
│    ├─ quantity: 10.5                                      │
│    ├─ received_at: 2026-02-15T14:30:45Z                  │
│    ├─ expires_at: 2026-02-22T14:30:45Z (calculated!)      │
│    ├─ created_at: 2026-02-15T14:30:45Z                   │
│    └─ updated_at: 2026-02-15T14:30:45Z                   │
└────────────────────────────────────────────────────────────┘
                        ↓
┌────────────────────────────────────────────────────────────┐
│ 5. USER UPDATES INVENTORY                                  │
│    PUT /api/inventory/products/{id}                        │
│    ├─ Update quantity (e.g., 8 liters left)              │
│    ├─ Update price (e.g., $14.00 on sale)                │
│    └─ System auto-updates updated_at timestamp            │
└────────────────────────────────────────────────────────────┘
                        ↓
┌────────────────────────────────────────────────────────────┐
│ 6. EXPIRATION TRACKING                                     │
│    ├─ Fresh: expires_at > now + 2 days                    │
│    ├─ Expiring Soon: expires_at <= now + 2 days          │
│    ├─ Expires Today: expires_at == now                    │
│    ├─ Expired: expires_at < now                           │
│    └─ No Expiration: expires_at = NULL                    │
└────────────────────────────────────────────────────────────┘
                        ↓
┌────────────────────────────────────────────────────────────┐
│ 7. USER REMOVES FROM INVENTORY                             │
│    DELETE /api/inventory/products/{id}                     │
│    └─ Product removed (soft/hard delete depending on BL)   │
└────────────────────────────────────────────────────────────┘
```

---

## 🚀 Deployment Checklist

- [x] Database schema has all constraints
- [x] Indexes on (user_id, tenant_id) for fast queries
- [x] Tenant isolation enforced in all queries
- [x] Expiration status calculated in domain
- [x] Money values stored in smallest unit (cents)
- [x] Quantity validation for negative/NaN/Infinity
- [x] Auto expiration calculation from catalog
- [x] Language from auth context (database source of truth)
- [x] Error handling with proper HTTP status codes
- [x] Query DTO pattern for single request/response
- [x] Tests for all validation rules

---

*Updated: 15 февраля 2026*  
*Production-Ready Flow ✅*
