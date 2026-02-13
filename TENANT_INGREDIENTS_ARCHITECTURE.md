# 🏗️ Tenant-Specific Ingredients Implementation

## 🎯 Problem Statement

**Current Issue:**
- `price` stored in global `catalog_ingredients` table
- All tenants see same price
- Users can't set their own supplier prices
- Violates SaaS multi-tenant principles

**Why This Is Wrong:**
```
❌ catalog_ingredients
   - name_en
   - price ← WRONG! Different for each tenant
   - supplier ← WRONG! Each tenant has different suppliers
```

## ✅ Correct SaaS Architecture

### Master Catalog (Admin-Managed)
```sql
catalog_ingredients
├── id
├── name_en, name_pl, name_uk, name_ru
├── category_id
├── default_unit
├── default_shelf_life_days
├── image_url
└── is_active
```
**Purpose:** Reference data shared across all tenants

### Tenant Catalog (User-Specific)
```sql
tenant_ingredients
├── id
├── tenant_id ← Links to user's restaurant
├── catalog_ingredient_id ← Links to master catalog
├── price ← TENANT-SPECIFIC
├── supplier ← TENANT-SPECIFIC (Metro, Selgros, etc.)
├── custom_unit ← Override if needed
├── custom_expiration_days ← Override if needed
└── notes ← Personal notes
```
**Purpose:** User's own prices, suppliers, and settings

## 📊 Data Flow

```
Admin Creates Master Data:
┌─────────────────────────────┐
│ POST /api/admin/products    │
│ {                           │
│   "name_en": "Tomato",     │
│   "category_id": "...",    │
│   "default_unit": "kg"     │
│ }                          │
└─────────────────────────────┘
           ↓
┌─────────────────────────────┐
│ catalog_ingredients         │
│ (global, no prices)         │
└─────────────────────────────┘

User Adds to Their Catalog:
┌─────────────────────────────┐
│ POST /api/tenant/ingredients│
│ {                           │
│   "catalog_ingredient_id",  │
│   "price": 12.50,          │
│   "supplier": "Metro"      │
│ }                          │
└─────────────────────────────┘
           ↓
┌─────────────────────────────┐
│ tenant_ingredients          │
│ (tenant-specific prices)    │
└─────────────────────────────┘
```

## 🛠️ Implementation Steps

### 1. Database Migrations

**Migration 1: Remove Price from Catalog**
```sql
-- migrations/20240119000001_remove_price_from_catalog.sql

-- Clean up duplicate "Onions"
UPDATE catalog_ingredients SET is_active = false
WHERE id IN (SELECT id FROM duplicates WHERE rn > 1);

-- Remove price (it's tenant-specific, not global)
ALTER TABLE catalog_ingredients DROP COLUMN price;
```

**Migration 2: Create Tenant Ingredients**
```sql
-- migrations/20240119000002_create_tenant_ingredients.sql

CREATE TABLE tenant_ingredients (
    id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL REFERENCES tenants(id),
    catalog_ingredient_id UUID NOT NULL REFERENCES catalog_ingredients(id),
    
    price DECIMAL(10,2),
    supplier VARCHAR(255),
    custom_unit unit_type,
    custom_expiration_days INTEGER,
    notes TEXT,
    
    is_active BOOLEAN DEFAULT true,
    created_at TIMESTAMP,
    updated_at TIMESTAMP,
    
    UNIQUE (tenant_id, catalog_ingredient_id)
);
```

### 2. Domain Models

**New: `src/domain/tenant_ingredient.rs`**
```rust
pub struct TenantIngredient {
    pub id: TenantIngredientId,
    pub tenant_id: TenantId,
    pub catalog_ingredient_id: CatalogIngredientId,
    pub price: Option<Decimal>,
    pub supplier: Option<String>,
    pub custom_unit: Option<Unit>,
    pub custom_expiration_days: Option<i32>,
    pub notes: Option<String>,
    pub is_active: bool,
}
```

### 3. Application Service

**New: `src/application/tenant_ingredient.rs`**

**Endpoints:**
```rust
POST   /api/tenant/ingredients        // Add from master catalog
GET    /api/tenant/ingredients        // List tenant's ingredients
GET    /api/tenant/ingredients/:id    // Get single ingredient
PUT    /api/tenant/ingredients/:id    // Update price/supplier
DELETE /api/tenant/ingredients/:id    // Remove from tenant catalog
GET    /api/tenant/ingredients/search // Search available catalog
```

**Key Features:**
- ✅ Tenant isolation (JWT tenant_id)
- ✅ Joins master catalog data with tenant data
- ✅ Prevents duplicate additions (UNIQUE constraint)
- ✅ Soft-delete support
- ✅ Search shows if already added

### 4. API Examples

**Add Ingredient to Tenant Catalog:**
```bash
POST /api/tenant/ingredients
Authorization: Bearer <user_jwt>
{
  "catalog_ingredient_id": "uuid-from-master-catalog",
  "price": 12.50,
  "supplier": "Metro Warsaw",
  "notes": "Buy on Tuesdays for discount"
}
```

**List Tenant's Ingredients:**
```bash
GET /api/tenant/ingredients
Authorization: Bearer <user_jwt>

Response:
[
  {
    "id": "...",
    "catalog_ingredient_id": "...",
    "catalog_name_en": "Tomato",
    "catalog_name_pl": "Pomidor",
    "category_id": "...",
    "default_unit": "kilogram",
    "image_url": "https://...",
    
    // Tenant-specific
    "price": 12.50,
    "supplier": "Metro Warsaw",
    "custom_unit": null,
    "notes": "Buy on Tuesdays"
  }
]
```

**Update Price/Supplier:**
```bash
PUT /api/tenant/ingredients/{id}
{
  "price": 13.00,
  "supplier": "Selgros"
}
```

**Search Available Catalog:**
```bash
GET /api/tenant/ingredients/search?q=tomato

Response:
[
  {
    "id": "...",
    "name_en": "Tomato",
    "category_id": "...",
    "already_added": false  // ← Can add
  },
  {
    "name_en": "Cherry Tomato",
    "already_added": true   // ← Already in catalog
  }
]
```

## 🔐 Security & Tenant Isolation

**JWT Claims:**
```json
{
  "sub": "user_id",
  "tenant_id": "restaurant_id",  // ← Used for filtering
  "iss": "restaurant-backend",
  "exp": 1234567890
}
```

**Query Pattern:**
```sql
SELECT * FROM tenant_ingredients
WHERE tenant_id = $1  -- From JWT
  AND is_active = true
```

**Prevents:**
- ❌ Tenant A seeing Tenant B's prices
- ❌ Price manipulation across tenants
- ❌ Unauthorized catalog modifications

## 📈 Benefits

### For Multi-Tenancy:
✅ **Data Isolation** - Each tenant has own prices
✅ **Scalability** - Master catalog shared, tenant data separated
✅ **Flexibility** - Users set own prices without affecting others

### For Business Logic:
✅ **Accurate Costing** - Use actual supplier prices
✅ **Multiple Suppliers** - Track where each ingredient comes from
✅ **Custom Settings** - Override units, expiration for specific needs

### For Admin:
✅ **Clean Master Data** - No tenant-specific pollution
✅ **Easy Updates** - Change catalog without affecting prices
✅ **Analytics** - See what ingredients are popular across tenants

## 🎯 Migration Strategy

**Phase 1: Create New Tables** ✅
- Add `tenant_ingredients` table
- Keep existing `catalog_ingredients`

**Phase 2: Migrate Existing Data**
```sql
-- Move existing inventory prices to tenant_ingredients
INSERT INTO tenant_ingredients (
    tenant_id, catalog_ingredient_id, price
)
SELECT
    ip.tenant_id,
    ip.catalog_ingredient_id,
    ip.price
FROM inventory_products ip
WHERE ip.price IS NOT NULL;
```

**Phase 3: Remove Old Price Column**
```sql
ALTER TABLE catalog_ingredients DROP COLUMN price;
```

**Phase 4: Update Application Code**
- Remove price from `AdminCatalogService`
- Add `TenantIngredientService`
- Update frontend to use new endpoints

## 🧪 Testing

**Test Scenarios:**
```bash
# 1. Add ingredient with price
POST /api/tenant/ingredients {
  "catalog_ingredient_id": "...",
  "price": 10.00,
  "supplier": "Metro"
}
→ SUCCESS

# 2. Try to add same ingredient again
POST /api/tenant/ingredients { same payload }
→ 409 Conflict: "Already added"

# 3. Different tenant can add same ingredient
POST /api/tenant/ingredients (different JWT)
→ SUCCESS (different tenant_id)

# 4. Update price
PUT /api/tenant/ingredients/{id} {
  "price": 12.00
}
→ SUCCESS, doesn't affect other tenants

# 5. Search shows added status
GET /api/tenant/ingredients/search?q=salt
→ Shows "already_added": true/false
```

## 📚 Related Documentation

- [Multi-Tenant Architecture](./ARCHITECTURE.md)
- [Catalog Uniqueness](./CATALOG_UNIQUENESS_SUCCESS.md)
- [Inventory System](./INVENTORY_IMPLEMENTATION.md)

---

**Status:** 🚧 In Progress  
**Next Steps:**
1. Apply migrations
2. Remove price from admin catalog API
3. Add tenant ingredient HTTP handlers
4. Update frontend to use new endpoints
5. Migrate existing inventory prices
