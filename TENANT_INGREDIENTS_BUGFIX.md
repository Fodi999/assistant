# 🐛 Bug Fix: custom_unit Type Casting

## Problem
```
DATABASE_ERROR when adding tenant ingredient
```

## Root Cause
```rust
// ❌ BEFORE
VALUES ($1, $2, $3, $4, $5, $6, $7, $8, true)
.bind(req.custom_unit.as_deref())  // Passing &str
```

`custom_unit` column is `unit_type` ENUM, but we were binding `Option<&str>`.  
PostgreSQL couldn't cast string to enum implicitly.

## Solution
```rust
// ✅ AFTER
VALUES ($1, $2, $3, $4, $5, $6::unit_type, $7, $8, true)
.bind(req.custom_unit.as_deref())  // Now with explicit cast
```

Added explicit `::unit_type` cast in SQL query.

## Testing
```bash
POST /api/tenant/ingredients
{
  "catalog_ingredient_id": "uuid",
  "price": 4.50,
  "supplier": "Metro",
  "custom_unit": null  # This was failing
}
```

## Lesson Learned
When working with PostgreSQL ENUMs in SQLx:
- Always use explicit `::enum_name` casts when binding parameters
- Even if value is NULL, PostgreSQL needs to know the type
- SQLx doesn't automatically infer ENUM types from bind parameters

## Related
- Migration: `20240119000002_create_tenant_ingredients.sql`
- Service: `src/application/tenant_ingredient.rs`
- Commit: `ecd5afc`
