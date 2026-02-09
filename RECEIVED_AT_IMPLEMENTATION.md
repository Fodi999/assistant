# Inventory Product - Received Date and Expiration Implementation

## ✅ Что добавлено

### 1. **Новое поле `received_at`** (дата поступления)
Позволяет отслеживать когда продукт был получен/куплен

### 2. **Существующее поле `expires_at`** (дата просрочки)
Остается опциональным, но теперь имеет четкий смысл вместе с `received_at`

## 📋 Схема БД

### Migration: `20240112000001_add_received_at_to_inventory.sql`

```sql
ALTER TABLE inventory_products 
ADD COLUMN received_at TIMESTAMPTZ NOT NULL DEFAULT NOW();

CREATE INDEX idx_inventory_products_received ON inventory_products(received_at);
```

## 🎯 API Changes

### POST /api/inventory/products

**Request Body**:
```json
{
  "catalog_ingredient_id": "uuid",
  "price_per_unit_cents": 1050,
  "quantity": 2.5,
  "received_at": "2026-02-09T10:00:00Z",  // 🆕 Дата поступления (optional, default = now)
  "expires_at": "2026-03-09T10:00:00Z"    // 📅 Дата просрочки (optional)
}
```

**Response**:
```json
{
  "id": "uuid",
  "catalog_ingredient_id": "uuid",
  "price_per_unit_cents": 1050,
  "quantity": 2.5,
  "received_at": "2026-02-09T10:00:00Z",  // 🆕 Возвращается в ответе
  "expires_at": "2026-03-09T10:00:00Z",
  "created_at": "2026-02-09T09:00:00Z",
  "updated_at": "2026-02-09T09:00:00Z"
}
```

### GET /api/inventory/products

Теперь возвращает `received_at` для каждого продукта:

```json
[
  {
    "id": "uuid",
    "product": {
      "name": "Пастеризованное молоко",
      "category": "Молочные продукты",
      "base_unit": "liter"
    },
    "quantity": 10.0,
    "price_per_unit_cents": 250,
    "received_at": "2026-02-09T10:00:00Z",  // 🆕 Дата поступления
    "expires_at": "2026-02-16T10:00:00Z",   // Дата просрочки
    "total_cost_cents": 2500
  }
]
```

## 🔧 Domain Model Changes

### `InventoryProduct` struct

```rust
pub struct InventoryProduct {
    pub id: InventoryProductId,
    pub user_id: UserId,
    pub tenant_id: TenantId,
    pub catalog_ingredient_id: CatalogIngredientId,
    pub price_per_unit: Money,
    pub quantity: Quantity,
    pub received_at: OffsetDateTime,        // 🆕 Дата поступления (required)
    pub expires_at: Option<OffsetDateTime>,  // Дата просрочки (optional)
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}
```

### Constructor

```rust
InventoryProduct::new(
    user_id,
    tenant_id,
    catalog_ingredient_id,
    price_per_unit,
    quantity,
    received_at,    // 🆕 Now required
    expires_at,     // Still optional
)
```

## 📊 Use Cases

### 1. Добавление продукта с датой поступления

```bash
curl -X POST https://.../api/inventory/products \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "catalog_ingredient_id": "138e48ba-e4fc-4bf4-8fee-6701397c2b73",
    "price_per_unit_cents": 1500,
    "quantity": 5.0,
    "received_at": "2026-02-09T08:00:00Z",
    "expires_at": "2026-02-23T23:59:59Z"
  }'
```

**Смысл**:
- `received_at`: 9 февраля 2026, 08:00 - продукт поступил на склад
- `expires_at`: 23 февраля 2026, 23:59 - истекает срок годности (14 дней)

### 2. Default received_at (если не указан)

```bash
curl -X POST https://.../api/inventory/products \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "catalog_ingredient_id": "138e48ba-e4fc-4bf4-8fee-6701397c2b73",
    "price_per_unit_cents": 1500,
    "quantity": 5.0,
    "expires_at": "2026-02-23T23:59:59Z"
  }'
```

Если `received_at` не указан → используется `NOW()` (текущее время).

## 🎯 Business Logic

### Calculating Expiration Warnings

Теперь можно вычислять:

1. **Days since received**: `NOW() - received_at`
2. **Days until expiration**: `expires_at - NOW()`
3. **Shelf life**: `expires_at - received_at`
4. **Freshness percentage**: `(expires_at - NOW()) / (expires_at - received_at) * 100`

### Example Query (будущая фича)

```sql
SELECT 
    ip.id,
    COALESCE(cit.name, 'Unknown') as name,
    ip.received_at,
    ip.expires_at,
    DATE_PART('day', ip.expires_at - ip.received_at) as shelf_life_days,
    DATE_PART('day', ip.expires_at - NOW()) as days_until_expiration,
    CASE 
        WHEN ip.expires_at < NOW() THEN 'expired'
        WHEN ip.expires_at < NOW() + INTERVAL '1 day' THEN 'expiring_today'
        WHEN ip.expires_at < NOW() + INTERVAL '3 days' THEN 'expiring_soon'
        ELSE 'fresh'
    END as status
FROM inventory_products ip
WHERE user_id = $1
ORDER BY ip.expires_at ASC NULLS LAST;
```

## 🧪 Testing

### Test 1: Add product with dates

```bash
TOKEN="your-token"

curl -X POST https://ministerial-yetta-fodi999-c58d8823.koyeb.app/api/inventory/products \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "catalog_ingredient_id": "519169f2-69f1-4875-94ed-12eccbb809ae",
    "price_per_unit_cents": 250,
    "quantity": 2.0,
    "received_at": "2026-02-09T09:00:00Z",
    "expires_at": "2026-02-16T23:59:59Z"
  }' | jq '.'
```

Expected response includes `received_at` field.

### Test 2: List products with dates

```bash
curl -H "Authorization: Bearer $TOKEN" \
  https://ministerial-yetta-fodi999-c58d8823.koyeb.app/api/inventory/products | jq '.[].received_at'
```

Should see all received dates.

## 🚀 Deployment Checklist

- [x] Migration created (`20240112000001_add_received_at_to_inventory.sql`)
- [x] Domain model updated (`InventoryProduct`)
- [x] Repository updated (`inventory_product_repository.rs`)
- [x] Service updated (`inventory.rs`)
- [x] HTTP handler updated (`interfaces/http/inventory.rs`)
- [x] Assistant command updated (`AddProductPayload`)
- [x] Compilation successful
- [ ] Run migration on production
- [ ] Deploy to Koyeb
- [ ] Test API endpoints

## 📝 Migration Notes

**Migration будет выполнена автоматически** при запуске сервера:
- Колонка `received_at` добавится с `DEFAULT NOW()`
- Существующие записи получат `received_at` = текущее время при миграции
- Индекс создастся автоматически

**Backward compatibility**: ✅
- Существующие записи не сломаются
- API теперь принимает `received_at` (optional with default)
- Frontend может не передавать `received_at` → будет NOW()

## 🎉 Result

После deployment API будет:
1. ✅ Принимать `received_at` и `expires_at` при добавлении продукта
2. ✅ Возвращать обе даты в GET запросах
3. ✅ Использовать `received_at` для расчета свежести (future feature)
4. ✅ Показывать дату поступления и просрочки в UI
