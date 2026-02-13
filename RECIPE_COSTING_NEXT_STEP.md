# 🎯 Next Step: Recipe Integration with Tenant Ingredients

## Current State ✅

**Implemented:**
- ✅ Master catalog (admin-managed)
- ✅ Tenant ingredients (user-specific prices)
- ✅ Soft-delete with partial unique index
- ✅ Multi-tenant isolation
- ✅ CRUD API for tenant ingredients
- ✅ Duplicate protection
- ✅ R2 image storage

## Architecture Evolution

### Before (Current)
```sql
recipes
├── recipe_ingredients
    ├── catalog_ingredient_id  ❌ No price info
    └── quantity
```

**Problem:** No costing! Can't calculate recipe cost.

### After (Target)
```sql
recipes
├── recipe_ingredients
    ├── tenant_ingredient_id  ✅ Links to tenant's price
    ├── quantity
    └── price_snapshot  ✅ Historical price at time of recipe creation
```

**Benefits:**
- ✅ Accurate costing per tenant
- ✅ Historical price tracking
- ✅ Different costs for different restaurants
- ✅ Recipe profitability analysis

## Implementation Plan

### 1. Update Recipe Ingredients Schema

**Migration: 20240121000001_recipe_use_tenant_ingredients.sql**

```sql
-- Add tenant_ingredient_id column
ALTER TABLE recipe_ingredients 
ADD COLUMN tenant_ingredient_id UUID REFERENCES tenant_ingredients(id);

-- Add price snapshot (capture price at recipe creation time)
ALTER TABLE recipe_ingredients 
ADD COLUMN price_snapshot DECIMAL(10,2);

-- Keep catalog_ingredient_id for backward compatibility during migration
-- Will remove it later after data migration

-- Add index
CREATE INDEX idx_recipe_ingredients_tenant 
ON recipe_ingredients(tenant_ingredient_id);
```

### 2. Data Migration Strategy

**Step 1:** For existing recipes, find matching tenant_ingredients:
```sql
UPDATE recipe_ingredients ri
SET tenant_ingredient_id = ti.id,
    price_snapshot = ti.price
FROM tenant_ingredients ti
WHERE ri.catalog_ingredient_id = ti.catalog_ingredient_id
  AND ri.recipe_id IN (
    SELECT id FROM recipes WHERE tenant_id = ti.tenant_id
  )
  AND ti.is_active = true;
```

**Step 2:** For ingredients not yet in tenant catalog, add them:
```sql
INSERT INTO tenant_ingredients (
  tenant_id, catalog_ingredient_id, is_active
)
SELECT DISTINCT 
  r.tenant_id,
  ri.catalog_ingredient_id,
  true
FROM recipe_ingredients ri
JOIN recipes r ON r.id = ri.recipe_id
WHERE ri.tenant_ingredient_id IS NULL
  AND NOT EXISTS (
    SELECT 1 FROM tenant_ingredients ti
    WHERE ti.tenant_id = r.tenant_id
      AND ti.catalog_ingredient_id = ri.catalog_ingredient_id
      AND ti.is_active = true
  );
```

**Step 3:** Make tenant_ingredient_id NOT NULL:
```sql
ALTER TABLE recipe_ingredients 
ALTER COLUMN tenant_ingredient_id SET NOT NULL;

-- Drop old column
ALTER TABLE recipe_ingredients 
DROP COLUMN catalog_ingredient_id;
```

### 3. Update Domain Models

**src/domain/recipe.rs:**
```rust
pub struct RecipeIngredient {
    pub id: RecipeIngredientId,
    pub recipe_id: RecipeId,
    pub tenant_ingredient_id: TenantIngredientId,  // Changed!
    pub quantity: Quantity,
    pub price_snapshot: Option<Decimal>,  // New!
    pub created_at: DateTime<Utc>,
}

impl RecipeIngredient {
    /// Calculate cost for this ingredient line
    pub fn calculate_cost(&self) -> Option<Decimal> {
        self.price_snapshot.map(|price| price * self.quantity.amount)
    }
}

pub struct Recipe {
    // ... existing fields
    pub ingredients: Vec<RecipeIngredientWithDetails>,
}

impl Recipe {
    /// Calculate total cost of recipe
    pub fn calculate_cost(&self) -> Decimal {
        self.ingredients
            .iter()
            .filter_map(|i| i.calculate_cost())
            .sum()
    }
    
    /// Calculate cost per serving
    pub fn cost_per_serving(&self) -> Option<Decimal> {
        if self.servings > 0 {
            Some(self.calculate_cost() / Decimal::from(self.servings))
        } else {
            None
        }
    }
}
```

### 4. Update Application Service

**src/application/recipe.rs:**
```rust
pub async fn add_ingredient(
    &self,
    tenant_id: Uuid,
    recipe_id: Uuid,
    req: AddRecipeIngredientRequest,
) -> AppResult<RecipeIngredient> {
    // Verify tenant_ingredient belongs to this tenant
    let tenant_ingredient = self.tenant_ingredient_service
        .get_ingredient(tenant_id, req.tenant_ingredient_id)
        .await?;
    
    // Snapshot the current price
    let price_snapshot = tenant_ingredient.price;
    
    sqlx::query_as!(
        RecipeIngredient,
        r#"
        INSERT INTO recipe_ingredients (
            id, recipe_id, tenant_ingredient_id, 
            quantity, unit, price_snapshot
        )
        VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING *
        "#,
        Uuid::new_v4(),
        recipe_id,
        req.tenant_ingredient_id,
        req.quantity,
        req.unit,
        price_snapshot
    )
    .fetch_one(&self.pool)
    .await?;
}

pub async fn get_recipe_with_cost(
    &self,
    tenant_id: Uuid,
    recipe_id: Uuid,
) -> AppResult<RecipeWithCost> {
    let recipe = sqlx::query_as!(
        RecipeWithCost,
        r#"
        SELECT
            r.*,
            COUNT(ri.id) as ingredient_count,
            SUM(ri.price_snapshot * ri.quantity) as total_cost,
            SUM(ri.price_snapshot * ri.quantity) / NULLIF(r.servings, 0) as cost_per_serving
        FROM recipes r
        LEFT JOIN recipe_ingredients ri ON ri.recipe_id = r.id
        WHERE r.id = $1 AND r.tenant_id = $2
        GROUP BY r.id
        "#,
        recipe_id,
        tenant_id
    )
    .fetch_one(&self.pool)
    .await?;
    
    Ok(recipe)
}
```

### 5. API Endpoints

**POST /api/recipes/:id/ingredients**
```json
{
  "tenant_ingredient_id": "uuid",
  "quantity": 0.5,
  "unit": "kilogram"
}
```

**GET /api/recipes/:id/cost**
```json
{
  "recipe_id": "uuid",
  "recipe_name": "Tomato Soup",
  "servings": 4,
  "total_cost": 12.50,
  "cost_per_serving": 3.13,
  "ingredients": [
    {
      "name": "Tomato",
      "quantity": 2.0,
      "unit": "kilogram",
      "price_snapshot": 3.20,
      "line_cost": 6.40
    },
    {
      "name": "Onion",
      "quantity": 0.5,
      "unit": "kilogram",
      "price_snapshot": 2.50,
      "line_cost": 1.25
    }
  ]
}
```

## Benefits of This Architecture

### 1. Multi-Tenant Costing ✅
```
Restaurant A (Warsaw):
  Tomato Soup = 12.50 PLN (Metro prices)

Restaurant B (Krakow):
  Tomato Soup = 10.20 PLN (Selgros prices)

Same recipe, different costs!
```

### 2. Historical Price Tracking ✅
```sql
-- See how recipe cost changed over time
SELECT 
  recipe_id,
  created_at,
  SUM(price_snapshot * quantity) as cost_at_creation
FROM recipe_ingredients
GROUP BY recipe_id, created_at
ORDER BY created_at DESC;
```

### 3. Menu Engineering ✅
```
High Profit Recipes:
- Caesar Salad: Cost 8 PLN, Sell 35 PLN → 77% margin
- Tomato Soup: Cost 12 PLN, Sell 28 PLN → 57% margin

Low Profit Recipes:
- Ribeye Steak: Cost 65 PLN, Sell 85 PLN → 24% margin
```

### 4. Inventory Costing ✅
```
When adding to inventory:
  unit_price comes from tenant_ingredients.price

When using in recipe:
  cost comes from recipe_ingredients.price_snapshot
```

## Testing Scenarios

### Scenario 1: Create Recipe with Costing
```bash
# Add ingredients to tenant catalog
POST /api/tenant/ingredients
{ "catalog_ingredient_id": "tomato", "price": 3.20 }

# Create recipe
POST /api/recipes
{ "name": "Tomato Soup", "servings": 4 }

# Add ingredient to recipe
POST /api/recipes/{id}/ingredients
{ "tenant_ingredient_id": "tenant-tomato-id", "quantity": 2.0 }

# Get recipe cost
GET /api/recipes/{id}/cost
→ total_cost: 6.40, cost_per_serving: 1.60
```

### Scenario 2: Price Change Impact
```bash
# Update price in tenant catalog
PUT /api/tenant/ingredients/{id}
{ "price": 4.00 }  # Increased!

# Old recipes keep old price snapshot
GET /api/recipes/old-recipe/cost
→ still uses 3.20 (historical)

# New recipes use new price
POST /api/recipes/{new}/ingredients
{ "tenant_ingredient_id": "tenant-tomato-id", "quantity": 2.0 }
→ will snapshot 4.00
```

### Scenario 3: Cross-Tenant Isolation
```bash
# Tenant A adds ingredient
POST /api/tenant/ingredients
{ "catalog_ingredient_id": "tomato", "price": 3.20 }

# Tenant B tries to use Tenant A's ingredient
POST /api/recipes/{recipe}/ingredients
{ "tenant_ingredient_id": "tenant-a-tomato-id" }  # Tenant B JWT
→ 404 NOT_FOUND (tenant isolation)
```

## Migration Timeline

1. **Phase 1 (Now):** Create migration files
2. **Phase 2:** Test migration on staging data
3. **Phase 3:** Deploy to production
4. **Phase 4:** Update frontend to show costs
5. **Phase 5:** Add menu engineering analytics

## Next Commands

```bash
# Create migration
touch migrations/20240121000001_recipe_use_tenant_ingredients.sql

# Update domain models
vim src/domain/recipe.rs

# Update services
vim src/application/recipe.rs

# Add costing endpoints
vim src/interfaces/http/recipe.rs

# Test
curl POST /api/recipes/{id}/cost
```

---

**Status:** Ready to implement  
**Priority:** HIGH - This is the core value proposition of the system  
**Complexity:** Medium - Clear path, good foundation already exists
