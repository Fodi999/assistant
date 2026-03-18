-- One-time sync: copy fiber/density/portion/shelf_life from products → catalog_ingredients
-- These fields were written to products by admin nutrition endpoints but
-- never synced to catalog_ingredients, causing data-quality audit to show them as missing.

-- Sync fiber from nutrition_macros → catalog_ingredients
UPDATE catalog_ingredients ci
SET fiber_per_100g = nm.fiber_g::numeric
FROM nutrition_macros nm
WHERE nm.product_id = ci.id
  AND nm.fiber_g IS NOT NULL
  AND ci.fiber_per_100g IS NULL;

-- Sync density, typical_portion, shelf_life from products → catalog_ingredients
UPDATE catalog_ingredients ci
SET
    density_g_per_ml  = COALESCE(p.density_g_per_ml::numeric, ci.density_g_per_ml),
    typical_portion_g = COALESCE(p.typical_portion_g::numeric, ci.typical_portion_g),
    shelf_life_days   = COALESCE(p.shelf_life_days, ci.shelf_life_days)
FROM products p
WHERE p.id = ci.id
  AND (
    (p.density_g_per_ml IS NOT NULL AND ci.density_g_per_ml IS NULL) OR
    (p.typical_portion_g IS NOT NULL AND ci.typical_portion_g IS NULL) OR
    (p.shelf_life_days IS NOT NULL AND ci.shelf_life_days IS NULL)
  );
