-- ══════════════════════════════════════════════════════════════════════════════
-- BACKFILL: Sync ingredient_states 'raw' nutrition from catalog_ingredients
--
-- Problem: When states were generated, some products had NULL nutrition.
-- unwrap_or(0) turned NULL → 0 in all states.
-- Later nutrition was filled via AI autofill, but states were NOT re-synced.
--
-- Fix: UPDATE raw state to match current catalog_ingredients values.
-- Also fix other states proportionally where they are still 0.
-- ══════════════════════════════════════════════════════════════════════════════

-- ── 1. Fix RAW state: copy nutrition directly from catalog_ingredients ──
UPDATE ingredient_states AS ist
SET
    calories_per_100g = ci.calories_per_100g,
    protein_per_100g  = ci.protein_per_100g::float8,
    fat_per_100g      = ci.fat_per_100g::float8,
    carbs_per_100g    = ci.carbs_per_100g::float8,
    fiber_per_100g    = ci.fiber_per_100g::float8
FROM catalog_ingredients AS ci
WHERE ist.ingredient_id = ci.id
  AND ist.state = 'raw'
  AND ci.calories_per_100g IS NOT NULL
  AND (ist.calories_per_100g IS NULL OR ist.calories_per_100g = 0);

-- ── 2. Fix BOILED/STEAMED: ~90-95% of raw (water-based cooking) ──
UPDATE ingredient_states AS ist
SET
    calories_per_100g = ROUND(ci.calories_per_100g * 0.95),
    protein_per_100g  = ROUND(ci.protein_per_100g::numeric * 0.92, 1),
    fat_per_100g      = ROUND(ci.fat_per_100g::numeric * 0.90, 1),
    carbs_per_100g    = ROUND(ci.carbs_per_100g::numeric * 0.95, 1),
    fiber_per_100g    = ci.fiber_per_100g::float8
FROM catalog_ingredients AS ci
WHERE ist.ingredient_id = ci.id
  AND ist.state IN ('boiled', 'steamed')
  AND ci.calories_per_100g IS NOT NULL
  AND (ist.calories_per_100g IS NULL OR ist.calories_per_100g = 0);

-- ── 3. Fix BAKED/GRILLED: ~concentrated, slight increase ──
UPDATE ingredient_states AS ist
SET
    calories_per_100g = ROUND(ci.calories_per_100g * 1.10),
    protein_per_100g  = ROUND(ci.protein_per_100g::numeric * 1.05, 1),
    fat_per_100g      = ROUND(ci.fat_per_100g::numeric * 0.95, 1),
    carbs_per_100g    = ROUND(ci.carbs_per_100g::numeric * 1.0, 1),
    fiber_per_100g    = ci.fiber_per_100g::float8
FROM catalog_ingredients AS ci
WHERE ist.ingredient_id = ci.id
  AND ist.state IN ('baked', 'grilled')
  AND ci.calories_per_100g IS NOT NULL
  AND (ist.calories_per_100g IS NULL OR ist.calories_per_100g = 0);

-- ── 4. Fix FRIED: keep existing oil_absorption values, add base nutrition ──
UPDATE ingredient_states AS ist
SET
    calories_per_100g = ROUND(ci.calories_per_100g * 1.30) + COALESCE(ist.calories_per_100g, 0),
    protein_per_100g  = ROUND(ci.protein_per_100g::numeric * 0.95, 1),
    fat_per_100g      = ROUND(ci.fat_per_100g::numeric * 1.50, 1) + COALESCE(ist.oil_absorption_g, 0),
    carbs_per_100g    = ROUND(ci.carbs_per_100g::numeric * 1.0, 1),
    fiber_per_100g    = ci.fiber_per_100g::float8
FROM catalog_ingredients AS ci
WHERE ist.ingredient_id = ci.id
  AND ist.state = 'fried'
  AND ci.calories_per_100g IS NOT NULL
  AND ist.protein_per_100g = 0;

-- ── 5. Fix SMOKED: concentrated + smoke ──
UPDATE ingredient_states AS ist
SET
    calories_per_100g = ROUND(ci.calories_per_100g * 1.20),
    protein_per_100g  = ROUND(ci.protein_per_100g::numeric * 1.15, 1),
    fat_per_100g      = ROUND(ci.fat_per_100g::numeric * 1.10, 1),
    carbs_per_100g    = ROUND(ci.carbs_per_100g::numeric * 1.0, 1),
    fiber_per_100g    = ci.fiber_per_100g::float8
FROM catalog_ingredients AS ci
WHERE ist.ingredient_id = ci.id
  AND ist.state = 'smoked'
  AND ci.calories_per_100g IS NOT NULL
  AND (ist.calories_per_100g IS NULL OR ist.calories_per_100g = 0);

-- ── 6. Fix FROZEN: same as raw ──
UPDATE ingredient_states AS ist
SET
    calories_per_100g = ci.calories_per_100g,
    protein_per_100g  = ci.protein_per_100g::float8,
    fat_per_100g      = ci.fat_per_100g::float8,
    carbs_per_100g    = ci.carbs_per_100g::float8,
    fiber_per_100g    = ci.fiber_per_100g::float8
FROM catalog_ingredients AS ci
WHERE ist.ingredient_id = ci.id
  AND ist.state = 'frozen'
  AND ci.calories_per_100g IS NOT NULL
  AND (ist.calories_per_100g IS NULL OR ist.calories_per_100g = 0);

-- ── 7. Fix DRIED: highly concentrated ──
UPDATE ingredient_states AS ist
SET
    calories_per_100g = ROUND(ci.calories_per_100g * 3.0),
    protein_per_100g  = ROUND(ci.protein_per_100g::numeric * 3.5, 1),
    fat_per_100g      = ROUND(ci.fat_per_100g::numeric * 2.5, 1),
    carbs_per_100g    = ROUND(ci.carbs_per_100g::numeric * 3.0, 1),
    fiber_per_100g    = ROUND(COALESCE(ci.fiber_per_100g, 0)::numeric * 3.0, 1)
FROM catalog_ingredients AS ci
WHERE ist.ingredient_id = ci.id
  AND ist.state = 'dried'
  AND ci.calories_per_100g IS NOT NULL
  AND (ist.calories_per_100g IS NULL OR ist.calories_per_100g = 0);

-- ── 8. Fix PICKLED: slight changes ──
UPDATE ingredient_states AS ist
SET
    calories_per_100g = ROUND(ci.calories_per_100g * 0.85),
    protein_per_100g  = ROUND(ci.protein_per_100g::numeric * 0.90, 1),
    fat_per_100g      = ROUND(ci.fat_per_100g::numeric * 0.85, 1),
    carbs_per_100g    = ROUND(ci.carbs_per_100g::numeric * 1.20, 1),
    fiber_per_100g    = ci.fiber_per_100g::float8
FROM catalog_ingredients AS ci
WHERE ist.ingredient_id = ci.id
  AND ist.state = 'pickled'
  AND ci.calories_per_100g IS NOT NULL
  AND (ist.calories_per_100g IS NULL OR ist.calories_per_100g = 0);
