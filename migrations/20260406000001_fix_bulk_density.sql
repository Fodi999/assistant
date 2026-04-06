-- ══════════════════════════════════════════════════════════════════════════════
-- FIX: Use BULK density (насыпная плотность) for volume↔mass conversions
-- ══════════════════════════════════════════════════════════════════════════════
--
-- Problem: some products had their density_g_per_ml set to TRUE (material)
-- density instead of BULK (packing) density. For kitchen volume conversions
-- (cup, tbsp, tsp → grams) we need bulk density — the weight of the product
-- as it sits in a measuring cup, including air gaps.
--
-- Example: almonds true density ≈ 1.15 g/ml → 1 cup = 272 g (WRONG)
--          almonds bulk density ≈ 0.60 g/ml → 1 cup = 142 g (CORRECT)
--
-- Sources: USDA FoodData Central, King Arthur Baking, various culinary references.
-- ══════════════════════════════════════════════════════════════════════════════

-- ── Nuts & Seeds (whole, not ground) ─────────────────────────────────────────
-- These are the most affected because true density is much higher than bulk.

-- Almonds (whole): bulk ~0.60 g/ml → 1 cup ≈ 142 g (USDA: 143g)
UPDATE catalog_ingredients SET density_g_per_ml = 0.60
WHERE LOWER(name_en) LIKE '%almond%'
  AND LOWER(name_en) NOT LIKE '%flour%'
  AND LOWER(name_en) NOT LIKE '%milk%'
  AND LOWER(name_en) NOT LIKE '%butter%'
  AND LOWER(name_en) NOT LIKE '%extract%'
  AND density_g_per_ml > 0.80;

UPDATE products SET density_g_per_ml = 0.60
WHERE LOWER(name_en) LIKE '%almond%'
  AND LOWER(name_en) NOT LIKE '%flour%'
  AND LOWER(name_en) NOT LIKE '%milk%'
  AND LOWER(name_en) NOT LIKE '%butter%'
  AND LOWER(name_en) NOT LIKE '%extract%'
  AND density_g_per_ml > 0.80;

-- Walnuts (halves): bulk ~0.52 g/ml → 1 cup ≈ 120 g (USDA: 117g)
UPDATE catalog_ingredients SET density_g_per_ml = 0.50
WHERE LOWER(name_en) LIKE '%walnut%'
  AND LOWER(name_en) NOT LIKE '%oil%'
  AND density_g_per_ml > 0.70;

UPDATE products SET density_g_per_ml = 0.50
WHERE LOWER(name_en) LIKE '%walnut%'
  AND LOWER(name_en) NOT LIKE '%oil%'
  AND density_g_per_ml > 0.70;

-- Cashews (whole): bulk ~0.58 g/ml → 1 cup ≈ 137 g (USDA: 137g)
UPDATE catalog_ingredients SET density_g_per_ml = 0.58
WHERE LOWER(name_en) LIKE '%cashew%'
  AND LOWER(name_en) NOT LIKE '%butter%'
  AND LOWER(name_en) NOT LIKE '%milk%'
  AND density_g_per_ml > 0.80;

UPDATE products SET density_g_per_ml = 0.58
WHERE LOWER(name_en) LIKE '%cashew%'
  AND LOWER(name_en) NOT LIKE '%butter%'
  AND LOWER(name_en) NOT LIKE '%milk%'
  AND density_g_per_ml > 0.80;

-- Hazelnuts (whole): bulk ~0.56 g/ml → 1 cup ≈ 135 g (USDA: 135g)
UPDATE catalog_ingredients SET density_g_per_ml = 0.57
WHERE LOWER(name_en) LIKE '%hazelnut%'
  AND LOWER(name_en) NOT LIKE '%butter%'
  AND density_g_per_ml > 0.80;

UPDATE products SET density_g_per_ml = 0.57
WHERE LOWER(name_en) LIKE '%hazelnut%'
  AND LOWER(name_en) NOT LIKE '%butter%'
  AND density_g_per_ml > 0.80;

-- Pecans (halves): bulk ~0.45 g/ml → 1 cup ≈ 99 g (USDA: 99g)
UPDATE catalog_ingredients SET density_g_per_ml = 0.42
WHERE LOWER(name_en) LIKE '%pecan%'
  AND density_g_per_ml > 0.70;

UPDATE products SET density_g_per_ml = 0.42
WHERE LOWER(name_en) LIKE '%pecan%'
  AND density_g_per_ml > 0.70;

-- Pine nuts: bulk ~0.56 g/ml → 1 cup ≈ 135 g (USDA: 135g)
UPDATE catalog_ingredients SET density_g_per_ml = 0.57
WHERE LOWER(name_en) LIKE '%pine nut%'
  AND density_g_per_ml > 0.80;

UPDATE products SET density_g_per_ml = 0.57
WHERE LOWER(name_en) LIKE '%pine nut%'
  AND density_g_per_ml > 0.80;

-- Pistachios (shelled): bulk ~0.53 g/ml → 1 cup ≈ 123 g (USDA: 123g)
UPDATE catalog_ingredients SET density_g_per_ml = 0.52
WHERE LOWER(name_en) LIKE '%pistachio%'
  AND density_g_per_ml > 0.80;

UPDATE products SET density_g_per_ml = 0.52
WHERE LOWER(name_en) LIKE '%pistachio%'
  AND density_g_per_ml > 0.80;

-- Peanuts (whole): bulk ~0.57 g/ml → 1 cup ≈ 146 g (USDA: 146g)
UPDATE catalog_ingredients SET density_g_per_ml = 0.62
WHERE LOWER(name_en) LIKE '%peanut%'
  AND LOWER(name_en) NOT LIKE '%butter%'
  AND LOWER(name_en) NOT LIKE '%oil%'
  AND density_g_per_ml > 0.80;

UPDATE products SET density_g_per_ml = 0.62
WHERE LOWER(name_en) LIKE '%peanut%'
  AND LOWER(name_en) NOT LIKE '%butter%'
  AND LOWER(name_en) NOT LIKE '%oil%'
  AND density_g_per_ml > 0.80;

-- Macadamia nuts: bulk ~0.56 g/ml → 1 cup ≈ 134 g (USDA: 134g)
UPDATE catalog_ingredients SET density_g_per_ml = 0.57
WHERE LOWER(name_en) LIKE '%macadamia%'
  AND density_g_per_ml > 0.80;

UPDATE products SET density_g_per_ml = 0.57
WHERE LOWER(name_en) LIKE '%macadamia%'
  AND density_g_per_ml > 0.80;

-- Sunflower seeds: bulk ~0.53 g/ml → 1 cup ≈ 128 g
UPDATE catalog_ingredients SET density_g_per_ml = 0.54
WHERE LOWER(name_en) LIKE '%sunflower seed%'
  AND density_g_per_ml > 0.80;

UPDATE products SET density_g_per_ml = 0.54
WHERE LOWER(name_en) LIKE '%sunflower seed%'
  AND density_g_per_ml > 0.80;

-- Pumpkin seeds: bulk ~0.55 g/ml → 1 cup ≈ 129 g
UPDATE catalog_ingredients SET density_g_per_ml = 0.55
WHERE LOWER(name_en) LIKE '%pumpkin seed%'
  AND density_g_per_ml > 0.80;

UPDATE products SET density_g_per_ml = 0.55
WHERE LOWER(name_en) LIKE '%pumpkin seed%'
  AND density_g_per_ml > 0.80;

-- Sesame seeds: bulk ~0.58 g/ml → 1 cup ≈ 144 g
UPDATE catalog_ingredients SET density_g_per_ml = 0.61
WHERE LOWER(name_en) LIKE '%sesame seed%'
  AND LOWER(name_en) NOT LIKE '%oil%'
  AND density_g_per_ml > 0.80;

UPDATE products SET density_g_per_ml = 0.61
WHERE LOWER(name_en) LIKE '%sesame seed%'
  AND LOWER(name_en) NOT LIKE '%oil%'
  AND density_g_per_ml > 0.80;

-- Chia seeds: bulk ~0.64 g/ml → 1 cup ≈ 152 g
UPDATE catalog_ingredients SET density_g_per_ml = 0.64
WHERE LOWER(name_en) LIKE '%chia%'
  AND density_g_per_ml > 0.90;

UPDATE products SET density_g_per_ml = 0.64
WHERE LOWER(name_en) LIKE '%chia%'
  AND density_g_per_ml > 0.90;

-- Flax seeds: bulk ~0.58 g/ml → 1 cup ≈ 168 g
UPDATE catalog_ingredients SET density_g_per_ml = 0.71
WHERE LOWER(name_en) LIKE '%flax%'
  AND LOWER(name_en) NOT LIKE '%oil%'
  AND density_g_per_ml > 0.90;

UPDATE products SET density_g_per_ml = 0.71
WHERE LOWER(name_en) LIKE '%flax%'
  AND LOWER(name_en) NOT LIKE '%oil%'
  AND density_g_per_ml > 0.90;

-- ── Grains & Legumes (dry) ───────────────────────────────────────────────────

-- Lentils (dry): bulk ~0.80 g/ml → 1 cup ≈ 192 g (USDA: 192g)
UPDATE catalog_ingredients SET density_g_per_ml = 0.81
WHERE LOWER(name_en) LIKE '%lentil%'
  AND density_g_per_ml > 1.10;

UPDATE products SET density_g_per_ml = 0.81
WHERE LOWER(name_en) LIKE '%lentil%'
  AND density_g_per_ml > 1.10;

-- Chickpeas (dry): bulk ~0.78 g/ml → 1 cup ≈ 200 g
UPDATE catalog_ingredients SET density_g_per_ml = 0.85
WHERE LOWER(name_en) LIKE '%chickpea%'
  AND density_g_per_ml > 1.10;

UPDATE products SET density_g_per_ml = 0.85
WHERE LOWER(name_en) LIKE '%chickpea%'
  AND density_g_per_ml > 1.10;

-- Beans (dry, various): bulk ~0.78 g/ml → 1 cup ≈ 184 g
UPDATE catalog_ingredients SET density_g_per_ml = 0.78
WHERE (LOWER(name_en) LIKE '%bean%' OR LOWER(name_en) LIKE '%kidney%' OR LOWER(name_en) LIKE '%black bean%')
  AND LOWER(name_en) NOT LIKE '%green bean%'
  AND LOWER(name_en) NOT LIKE '%string bean%'
  AND LOWER(name_en) NOT LIKE '%coffee%'
  AND density_g_per_ml > 1.10;

UPDATE products SET density_g_per_ml = 0.78
WHERE (LOWER(name_en) LIKE '%bean%' OR LOWER(name_en) LIKE '%kidney%' OR LOWER(name_en) LIKE '%black bean%')
  AND LOWER(name_en) NOT LIKE '%green bean%'
  AND LOWER(name_en) NOT LIKE '%string bean%'
  AND LOWER(name_en) NOT LIKE '%coffee%'
  AND density_g_per_ml > 1.10;

-- Quinoa (dry): bulk ~0.73 g/ml → 1 cup ≈ 170 g (USDA: 170g)
UPDATE catalog_ingredients SET density_g_per_ml = 0.72
WHERE LOWER(name_en) LIKE '%quinoa%'
  AND density_g_per_ml > 1.00;

UPDATE products SET density_g_per_ml = 0.72
WHERE LOWER(name_en) LIKE '%quinoa%'
  AND density_g_per_ml > 1.00;

-- Couscous (dry): bulk ~0.65 g/ml → 1 cup ≈ 173 g
UPDATE catalog_ingredients SET density_g_per_ml = 0.73
WHERE LOWER(name_en) LIKE '%couscous%'
  AND density_g_per_ml > 1.00;

UPDATE products SET density_g_per_ml = 0.73
WHERE LOWER(name_en) LIKE '%couscous%'
  AND density_g_per_ml > 1.00;

-- ── Dried fruit ──────────────────────────────────────────────────────────────

-- Raisins: bulk ~0.65 g/ml → 1 cup ≈ 145 g (USDA: 145g)
UPDATE catalog_ingredients SET density_g_per_ml = 0.61
WHERE LOWER(name_en) LIKE '%raisin%'
  AND density_g_per_ml > 0.90;

UPDATE products SET density_g_per_ml = 0.61
WHERE LOWER(name_en) LIKE '%raisin%'
  AND density_g_per_ml > 0.90;

-- Dried cranberries: bulk ~0.50 g/ml → 1 cup ≈ 120 g
UPDATE catalog_ingredients SET density_g_per_ml = 0.51
WHERE LOWER(name_en) LIKE '%cranberr%' AND LOWER(name_en) LIKE '%dried%'
  AND density_g_per_ml > 0.80;

UPDATE products SET density_g_per_ml = 0.51
WHERE LOWER(name_en) LIKE '%cranberr%' AND LOWER(name_en) LIKE '%dried%'
  AND density_g_per_ml > 0.80;

-- Dates (pitted): bulk ~0.66 g/ml → 1 cup ≈ 147 g (USDA: 147g)
UPDATE catalog_ingredients SET density_g_per_ml = 0.62
WHERE LOWER(name_en) LIKE '%date%'
  AND LOWER(name_en) NOT LIKE '%update%'
  AND density_g_per_ml > 0.90;

UPDATE products SET density_g_per_ml = 0.62
WHERE LOWER(name_en) LIKE '%date%'
  AND LOWER(name_en) NOT LIKE '%update%'
  AND density_g_per_ml > 0.90;

-- ── Shredded / flaked / coconut ──────────────────────────────────────────────

-- Coconut (shredded, desiccated): bulk ~0.34 g/ml → 1 cup ≈ 80 g (USDA: 80g)
UPDATE catalog_ingredients SET density_g_per_ml = 0.34
WHERE LOWER(name_en) LIKE '%coconut%'
  AND (LOWER(name_en) LIKE '%shred%' OR LOWER(name_en) LIKE '%flake%' OR LOWER(name_en) LIKE '%desicc%')
  AND density_g_per_ml > 0.60;

UPDATE products SET density_g_per_ml = 0.34
WHERE LOWER(name_en) LIKE '%coconut%'
  AND (LOWER(name_en) LIKE '%shred%' OR LOWER(name_en) LIKE '%flake%' OR LOWER(name_en) LIKE '%desicc%')
  AND density_g_per_ml > 0.60;

-- ── Grated cheese (not liquid) ───────────────────────────────────────────────

-- Grated/shredded parmesan: bulk ~0.45 g/ml → 1 cup ≈ 100 g (USDA: 100g)
-- (block parmesan density ~1.2 is correct for weight-based but not for cups)
UPDATE catalog_ingredients SET density_g_per_ml = 0.42
WHERE LOWER(name_en) LIKE '%parmesan%'
  AND density_g_per_ml > 0.80;

UPDATE products SET density_g_per_ml = 0.42
WHERE LOWER(name_en) LIKE '%parmesan%'
  AND density_g_per_ml > 0.80;

-- Shredded cheddar/mozzarella: bulk ~0.47 g/ml → 1 cup ≈ 113 g (USDA: 113g)
UPDATE catalog_ingredients SET density_g_per_ml = 0.48
WHERE (LOWER(name_en) LIKE '%cheddar%' OR LOWER(name_en) LIKE '%mozzarella%')
  AND density_g_per_ml > 0.80;

UPDATE products SET density_g_per_ml = 0.48
WHERE (LOWER(name_en) LIKE '%cheddar%' OR LOWER(name_en) LIKE '%mozzarella%')
  AND density_g_per_ml > 0.80;

-- ── Powders with wrong density ───────────────────────────────────────────────

-- Cocoa powder: bulk ~0.55 g/ml → 1 cup ≈ 86 g (USDA: 86g)
-- (keeping existing 0.72 only if it was overwritten to something unreasonable)
UPDATE catalog_ingredients SET density_g_per_ml = 0.36
WHERE LOWER(name_en) LIKE '%cocoa%'
  AND LOWER(name_en) NOT LIKE '%butter%'
  AND density_g_per_ml > 0.90;

UPDATE products SET density_g_per_ml = 0.36
WHERE LOWER(name_en) LIKE '%cocoa%'
  AND LOWER(name_en) NOT LIKE '%butter%'
  AND density_g_per_ml > 0.90;

-- Corn starch: bulk ~0.56 g/ml → 1 cup ≈ 128 g (USDA: 128g)
UPDATE catalog_ingredients SET density_g_per_ml = 0.54
WHERE LOWER(name_en) LIKE '%corn starch%' OR LOWER(name_en) LIKE '%cornstarch%'
  AND density_g_per_ml > 0.80;

UPDATE products SET density_g_per_ml = 0.54
WHERE LOWER(name_en) LIKE '%corn starch%' OR LOWER(name_en) LIKE '%cornstarch%'
  AND density_g_per_ml > 0.80;

-- Almond flour/meal: bulk ~0.42 g/ml → 1 cup ≈ 96 g (USDA: 96g)
UPDATE catalog_ingredients SET density_g_per_ml = 0.41
WHERE LOWER(name_en) LIKE '%almond flour%' OR LOWER(name_en) LIKE '%almond meal%'
  AND density_g_per_ml > 0.60;

UPDATE products SET density_g_per_ml = 0.41
WHERE LOWER(name_en) LIKE '%almond flour%' OR LOWER(name_en) LIKE '%almond meal%'
  AND density_g_per_ml > 0.60;

-- Coconut flour: bulk ~0.50 g/ml → 1 cup ≈ 112 g
UPDATE catalog_ingredients SET density_g_per_ml = 0.47
WHERE LOWER(name_en) LIKE '%coconut flour%'
  AND density_g_per_ml > 0.70;

UPDATE products SET density_g_per_ml = 0.47
WHERE LOWER(name_en) LIKE '%coconut flour%'
  AND density_g_per_ml > 0.70;
