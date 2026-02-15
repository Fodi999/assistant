# 📦 Inventory System - Quick Reference Guide

## 🔷 Endpoints Summary

| Method | Endpoint | Purpose | Auth | Returns |
|--------|----------|---------|------|---------|
| **POST** | `/api/inventory/products` | Add product | JWT | InventoryView (201) |
| **GET** | `/api/inventory/products` | List all | JWT | Vec<InventoryView> (200) |
| **PUT** | `/api/inventory/products/{id}` | Update qty/price | JWT | (204) |
| **DELETE** | `/api/inventory/products/{id}` | Remove | JWT | (204) |
| **GET** | `/api/inventory/status` | Expiration stats | JWT | InventoryStatus (200) |

---

## 📥 Add Product Request

### Request Body

```json
{
  "catalog_ingredient_id": "519169f2-69f1-4875-94ed-12eccbb809ae",
  "price_per_unit_cents": 1500,
  "quantity": 10.5,
  "received_at": "2026-02-15T12:00:00Z",
  "expires_at": null
}
```

### Fields Explained

| Field | Type | Required | Notes |
|-------|------|----------|-------|
| `catalog_ingredient_id` | UUID | ✅ | Must exist in catalog_ingredients |
| `price_per_unit_cents` | i64 | ✅ | Must be >= 0 (in smallest unit: cents, grosze) |
| `quantity` | f64 | ✅ | Must be > 0 and finite |
| `received_at` | ISO8601 | ✅ | When product was purchased/received |
| `expires_at` | ISO8601 | ❌ | If null → auto-calculated from catalog |

---

## 📤 Add Product Response (HTTP 201)

```json
{
  "id": "a1b2c3d4-e5f6-7890-abcd-ef1234567890",
  "product": {
    "id": "519169f2-69f1-4875-94ed-12eccbb809ae",
    "name": "Pasteurized milk",
    "category": "Dairy and Eggs",
    "base_unit": "liter",
    "image_url": "https://..."
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

## 🔄 Data Flow

```
Frontend Input
    ↓
┌─────────────────────────────────┐
│ Validation                      │
│ - price >= 0                    │
│ - quantity > 0 and finite       │
│ - ingredient exists             │
└─────────────────────────────────┘
    ↓
┌─────────────────────────────────┐
│ Auto-Calculate                  │
│ expires_at = received_at +      │
│   default_shelf_life_days       │
└─────────────────────────────────┘
    ↓
┌─────────────────────────────────┐
│ Create Domain Model             │
│ InventoryProduct::new()         │
│ ├─ id: UUID (generated)         │
│ ├─ user_id: from auth           │
│ ├─ tenant_id: from auth         │
│ └─ ... all fields               │
└─────────────────────────────────┘
    ↓
┌─────────────────────────────────┐
│ Save to Database                │
│ INSERT inventory_products...    │
└─────────────────────────────────┘
    ↓
┌─────────────────────────────────┐
│ Fetch & Enrich                  │
│ SELECT ... FROM inventory       │
│ JOIN catalog_ingredients        │
│ JOIN catalog_*_translations     │
│ JOIN catalog_categories         │
└─────────────────────────────────┘
    ↓
Return InventoryView (JSON)
```

---

## 💰 Money Handling

**Rule**: Always store money in smallest unit (cents, grosze, etc.)

```
User Input:  $15.00
Frontend:    15.00 * 100 = 1500 cents
Backend:     Money(1500)
Database:    price_per_unit_cents: 1500
API:         "price_per_unit_cents": 1500
Frontend:    1500 / 100 = $15.00
```

**Conversions**:
- `Money::from_cents(1500)` → Money object
- `Money::from_major(15.00)` → Money object
- `money.as_cents()` → 1500
- `money.as_major()` → 15.00

---

## 📅 Expiration Status

Calculated based on `expires_at` date:

| Status | Condition | Color | Action |
|--------|-----------|-------|--------|
| **Fresh** | `expires_at > today + 2 days` | 🟢 Green | No action |
| **Expiring Soon** | `today < expires_at <= today + 2 days` | 🟡 Yellow | Warn |
| **Expires Today** | `expires_at == today` | 🔴 Red | Use first |
| **Expired** | `expires_at < today` | ⚫ Black | Remove |
| **No Expiration** | `expires_at = null` | ⚪ Gray | No shelf life |

---

## 🔒 Tenant Isolation

Every query filters by both `user_id` AND `tenant_id`:

```sql
WHERE ip.user_id = $1 AND ip.tenant_id = $2
```

**This ensures**:
- User can only see their own products
- User can only see products in their tenant
- No data leakage between tenants

---

## 🌍 Language Handling

**Language source**: `users.language` (from database, via AuthUser)

```
Flow:
1. JWT token contains user_id
2. Middleware fetches user record from DB
3. Reads language field
4. Passes via AuthUser context
5. Service uses it for translations

SQL with fallback:
COALESCE(
  cit_user.name,      -- Try user's language
  cit_en.name,        -- Fallback to English
  'Unknown'           -- Last resort
)
```

---

## ✅ Validation Levels

### Level 1: HTTP (serde)
```
- UUID format
- Number types
- ISO8601 dates
```

### Level 2: Service (Business Logic)
```
- Money >= 0
- Quantity > 0
- Quantity is finite
- Ingredient exists
```

### Level 3: Domain (Invariants)
```
- received_at <= expires_at
- price_per_unit >= 0
- quantity > 0
```

### Level 4: Database (Constraints)
```
- CHECK (quantity > 0)
- CHECK (price_per_unit_cents >= 0)
- CHECK (expires_at >= received_at)
- Foreign key constraints
```

---

## 🚨 Error Responses

### 400 Bad Request
```json
{
  "error": "Validation error",
  "message": "Quantity cannot be negative"
}
```

### 401 Unauthorized
```json
{
  "error": "Authentication failed",
  "message": "Missing or invalid authorization header"
}
```

### 404 Not Found
```json
{
  "error": "Not found",
  "message": "Catalog ingredient not found"
}
```

### 500 Internal Server Error
```json
{
  "error": "Internal server error",
  "message": "Database connection failed"
}
```

---

## 🧪 Testing Checklist

- [ ] Add product with all fields
- [ ] Add product with expires_at = null (auto-calculate)
- [ ] Negative price → 400 error
- [ ] Negative quantity → 400 error
- [ ] Quantity = 0 → 400 error
- [ ] Quantity = NaN → 400 error
- [ ] Non-existent ingredient → 404 error
- [ ] Invalid JWT → 401 error
- [ ] Product belongs to different user → 404 error
- [ ] List shows only user's products
- [ ] Update quantity works
- [ ] Update price works
- [ ] Delete product works
- [ ] Expiration status calculated correctly
- [ ] Language translations work (user lang → en fallback)

---

## 📊 Performance Notes

| Operation | Time | Notes |
|-----------|------|-------|
| Add product | ~100ms | INSERT + SELECT JOIN |
| List products | ~50ms | Single SQL query with JOINs |
| Expiration check | Instant | Calculated in memory |
| Total cost | Instant | Multiplying Money values |

**Indexes**:
- `(user_id, tenant_id)` - for listing user's products
- `(expires_at)` - for finding expiring products
- `(user_id, tenant_id, id)` - unique constraint

---

## 🔗 Related Endpoints

- **Search Catalog**: `GET /api/catalog/ingredients?q=milk`
- **Get Catalog Details**: `GET /api/catalog/ingredients/{id}`
- **Get Expiration Stats**: `GET /api/inventory/status`
- **Update Quantity**: `PUT /api/inventory/products/{id}`
- **Remove from Inventory**: `DELETE /api/inventory/products/{id}`

---

## 📚 Code Locations

| Component | File |
|-----------|------|
| HTTP Handler | `src/interfaces/http/inventory.rs` |
| Service Logic | `src/application/inventory.rs` |
| Domain Model | `src/domain/inventory.rs` |
| Database | `src/infrastructure/persistence/inventory_product_repository.rs` |
| Database Schema | `migrations/20240106000001_inventory_products.sql` |

---

## 🎯 Key Design Decisions

1. **Money in Cents**: Avoid floating-point math for currency
2. **Auto Expiration**: Use catalog defaults, user can override
3. **Query DTO**: Single SQL query returns all needed data
4. **Value Objects**: Money and Quantity encapsulate validation
5. **Tenant Isolation**: Every query filters by user + tenant
6. **Language from DB**: Not from frontend (server is source of truth)
7. **Domain-Driven**: Business rules in domain layer, not database
8. **Soft Validation**: Multiple validation levels catch errors early

---

*Updated: 15 февраля 2026*  
*Quick Reference ✅*
