-- ══════════════════════════════════════════════════════════════════════════════
-- SEED products FROM catalog_ingredients + fill nutrition tables
-- Migration: 20260312000002_seed_products_from_catalog.sql
-- ══════════════════════════════════════════════════════════════════════════════

-- ── Step 1: copy base product data ───────────────────────────────────────────
INSERT INTO products (
    id, slug, name_en, name_pl, name_ru, name_uk,
    category_id, product_type, unit, image_url, description,
    density_g_per_ml, typical_portion_g, edible_yield_percent,
    shelf_life_days, wild_farmed, water_type, sushi_grade,
    substitution_group, availability_months, created_at, updated_at
)
SELECT
    id,
    COALESCE(slug, lower(regexp_replace(name_en, '[^a-zA-Z0-9]+', '-', 'g'))),
    name_en,
    name_pl,
    name_ru,
    name_uk,
    category_id,
    COALESCE(product_type, 'other'),
    CASE default_unit::text
        WHEN 'kilogram' THEN 'kg'
        WHEN 'liter'    THEN 'l'
        WHEN 'piece'    THEN 'pcs'
        WHEN 'bunch'    THEN 'bunch'
        WHEN 'bottle'   THEN 'bottle'
        WHEN 'can'      THEN 'can'
        WHEN 'package'  THEN 'pkg'
        ELSE default_unit::text
    END,
    image_url,
    COALESCE(description_ru, description),
    density_g_per_ml::real,
    typical_portion_g::real,
    edible_yield_percent::real,
    COALESCE(shelf_life_days, default_shelf_life_days),
    CASE wild_farmed
        WHEN 'wild'   THEN 'wild'
        WHEN 'farmed' THEN 'farmed'
        WHEN 'both'   THEN 'both'
        ELSE NULL
    END,
    CASE water_type
        WHEN 'sea'         THEN 'salt'
        WHEN 'freshwater'  THEN 'fresh'
        WHEN 'both'        THEN 'both'
        ELSE NULL
    END,
    COALESCE(sushi_grade, false),
    substitution_group,
    availability_months,
    created_at,
    updated_at
FROM catalog_ingredients
WHERE is_active = true
ON CONFLICT (id) DO NOTHING;

-- ── Step 2: nutrition_macros ──────────────────────────────────────────────────
INSERT INTO nutrition_macros (
    product_id, calories_kcal, protein_g, fat_g, carbs_g, fiber_g, sugar_g
)
SELECT
    id,
    calories_per_100g::real,
    protein_per_100g::real,
    fat_per_100g::real,
    carbs_per_100g::real,
    fiber_per_100g::real,
    sugar_per_100g::real
FROM catalog_ingredients
WHERE is_active = true
  AND (calories_per_100g IS NOT NULL
    OR protein_per_100g IS NOT NULL
    OR fat_per_100g IS NOT NULL
    OR carbs_per_100g IS NOT NULL)
ON CONFLICT (product_id) DO NOTHING;

-- ── Step 3: nutrition_vitamins — seafood specific data ────────────────────────
-- Salmon: rich in D, B12, B3
INSERT INTO nutrition_vitamins (product_id, vitamin_d, vitamin_b12, vitamin_b3, vitamin_b6, vitamin_a, vitamin_e)
SELECT id, 11.0, 3.2, 7.9, 0.8, 12.0, 3.6
FROM products WHERE slug = 'salmon'
ON CONFLICT (product_id) DO NOTHING;

-- Tuna: B12, D, B3
INSERT INTO nutrition_vitamins (product_id, vitamin_d, vitamin_b12, vitamin_b3, vitamin_b6, vitamin_a)
SELECT id, 5.7, 9.4, 15.0, 0.9, 20.0
FROM products WHERE slug = 'tuna'
ON CONFLICT (product_id) DO NOTHING;

-- Mackerel: D, B12
INSERT INTO nutrition_vitamins (product_id, vitamin_d, vitamin_b12, vitamin_b3, vitamin_b6)
SELECT id, 16.1, 8.7, 9.1, 0.5
FROM products WHERE slug = 'mackerel'
ON CONFLICT (product_id) DO NOTHING;

-- Herring: D, B12
INSERT INTO nutrition_vitamins (product_id, vitamin_d, vitamin_b12, vitamin_b3)
SELECT id, 19.2, 13.1, 4.1
FROM products WHERE slug = 'herring'
ON CONFLICT (product_id) DO NOTHING;

-- Cod: B12
INSERT INTO nutrition_vitamins (product_id, vitamin_b12, vitamin_b3, vitamin_d)
SELECT id, 0.9, 2.5, 1.0
FROM products WHERE slug = 'cod'
ON CONFLICT (product_id) DO NOTHING;

-- Spinach: K, A, C, folate
INSERT INTO nutrition_vitamins (product_id, vitamin_k, vitamin_a, vitamin_c, vitamin_b9, vitamin_e)
SELECT id, 482.9, 469.0, 28.1, 194.0, 2.0
FROM products WHERE slug = 'spinach'
ON CONFLICT (product_id) DO NOTHING;

-- Broccoli: C, K
INSERT INTO nutrition_vitamins (product_id, vitamin_c, vitamin_k, vitamin_a, vitamin_b9)
SELECT id, 89.2, 101.6, 31.0, 63.0
FROM products WHERE slug = 'broccoli'
ON CONFLICT (product_id) DO NOTHING;

-- Avocado: K, E, B5
INSERT INTO nutrition_vitamins (product_id, vitamin_k, vitamin_e, vitamin_b5, vitamin_b6, vitamin_c)
SELECT id, 21.0, 2.1, 1.4, 0.3, 10.0
FROM products WHERE slug = 'avocado'
ON CONFLICT (product_id) DO NOTHING;

-- Chicken breast: B3, B6
INSERT INTO nutrition_vitamins (product_id, vitamin_b3, vitamin_b6, vitamin_b12)
SELECT id, 13.7, 1.0, 0.3
FROM products WHERE slug = 'chicken-breast'
ON CONFLICT (product_id) DO NOTHING;

-- Beef: B12, B3, B6
INSERT INTO nutrition_vitamins (product_id, vitamin_b12, vitamin_b3, vitamin_b6, vitamin_a)
SELECT id, 2.6, 7.9, 0.9, 9.0
FROM products WHERE slug = 'beef'
ON CONFLICT (product_id) DO NOTHING;

-- Milk: B12, D, B2
INSERT INTO nutrition_vitamins (product_id, vitamin_b12, vitamin_d, vitamin_b2, vitamin_a, vitamin_b5)
SELECT id, 0.5, 1.3, 0.2, 46.0, 0.4
FROM products WHERE slug = 'milk'
ON CONFLICT (product_id) DO NOTHING;

-- Eggs: D, B12, A, E
INSERT INTO nutrition_vitamins (product_id, vitamin_d, vitamin_b12, vitamin_a, vitamin_e, vitamin_b2, vitamin_b5)
SELECT id, 2.0, 1.1, 149.0, 1.1, 0.5, 1.4
FROM products WHERE slug = 'chicken-eggs'
ON CONFLICT (product_id) DO NOTHING;

-- Orange: C, folate
INSERT INTO nutrition_vitamins (product_id, vitamin_c, vitamin_b9, vitamin_a)
SELECT id, 53.2, 30.0, 11.0
FROM products WHERE slug = 'orange'
ON CONFLICT (product_id) DO NOTHING;

-- Lemon: C
INSERT INTO nutrition_vitamins (product_id, vitamin_c, vitamin_b9)
SELECT id, 53.0, 11.0
FROM products WHERE slug = 'lemon'
ON CONFLICT (product_id) DO NOTHING;

-- Garlic: C, B6
INSERT INTO nutrition_vitamins (product_id, vitamin_c, vitamin_b6, vitamin_b1)
SELECT id, 31.2, 1.2, 0.2
FROM products WHERE slug = 'garlic'
ON CONFLICT (product_id) DO NOTHING;

-- ── Step 4: nutrition_minerals ────────────────────────────────────────────────
-- Salmon
INSERT INTO nutrition_minerals (product_id, phosphorus, potassium, selenium, magnesium, sodium, calcium, iron, zinc)
SELECT id, 371.0, 628.0, 36.5, 29.0, 59.0, 12.0, 0.8, 0.6
FROM products WHERE slug = 'salmon'
ON CONFLICT (product_id) DO NOTHING;

-- Tuna
INSERT INTO nutrition_minerals (product_id, phosphorus, potassium, selenium, magnesium, sodium, iron, zinc)
SELECT id, 326.0, 444.0, 76.0, 64.0, 47.0, 1.0, 0.6
FROM products WHERE slug = 'tuna'
ON CONFLICT (product_id) DO NOTHING;

-- Mackerel
INSERT INTO nutrition_minerals (product_id, phosphorus, potassium, selenium, magnesium, sodium, calcium, iron)
SELECT id, 217.0, 314.0, 44.1, 60.0, 83.0, 12.0, 1.6
FROM products WHERE slug = 'mackerel'
ON CONFLICT (product_id) DO NOTHING;

-- Shrimp
INSERT INTO nutrition_minerals (product_id, phosphorus, potassium, selenium, magnesium, sodium, calcium, iron, zinc)
SELECT id, 205.0, 185.0, 33.7, 35.0, 119.0, 52.0, 0.5, 1.1
FROM products WHERE slug = 'shrimp'
ON CONFLICT (product_id) DO NOTHING;

-- Spinach
INSERT INTO nutrition_minerals (product_id, iron, calcium, magnesium, potassium, phosphorus, manganese, sodium)
SELECT id, 2.7, 99.0, 79.0, 558.0, 49.0, 0.9, 79.0
FROM products WHERE slug = 'spinach'
ON CONFLICT (product_id) DO NOTHING;

-- Broccoli
INSERT INTO nutrition_minerals (product_id, calcium, iron, potassium, magnesium, phosphorus, manganese)
SELECT id, 47.0, 0.7, 316.0, 21.0, 66.0, 0.2
FROM products WHERE slug = 'broccoli'
ON CONFLICT (product_id) DO NOTHING;

-- Avocado
INSERT INTO nutrition_minerals (product_id, potassium, magnesium, phosphorus, iron, calcium, copper)
SELECT id, 485.0, 29.0, 52.0, 0.6, 12.0, 0.2
FROM products WHERE slug = 'avocado'
ON CONFLICT (product_id) DO NOTHING;

-- Beef
INSERT INTO nutrition_minerals (product_id, iron, zinc, phosphorus, potassium, selenium, magnesium, sodium)
SELECT id, 2.7, 5.8, 220.0, 318.0, 18.9, 21.0, 55.0
FROM products WHERE slug = 'beef'
ON CONFLICT (product_id) DO NOTHING;

-- Chicken breast
INSERT INTO nutrition_minerals (product_id, phosphorus, potassium, selenium, magnesium, sodium, iron, zinc)
SELECT id, 220.0, 340.0, 27.6, 29.0, 74.0, 0.7, 0.9
FROM products WHERE slug = 'chicken-breast'
ON CONFLICT (product_id) DO NOTHING;

-- Eggs
INSERT INTO nutrition_minerals (product_id, phosphorus, selenium, iron, zinc, calcium, potassium, sodium)
SELECT id, 198.0, 30.7, 1.8, 1.3, 50.0, 126.0, 124.0
FROM products WHERE slug = 'chicken-eggs'
ON CONFLICT (product_id) DO NOTHING;

-- Milk
INSERT INTO nutrition_minerals (product_id, calcium, phosphorus, potassium, magnesium, sodium, selenium, zinc)
SELECT id, 113.0, 84.0, 143.0, 10.0, 43.0, 3.1, 0.4
FROM products WHERE slug = 'milk'
ON CONFLICT (product_id) DO NOTHING;

-- Almonds
INSERT INTO nutrition_minerals (product_id, calcium, magnesium, phosphorus, potassium, iron, zinc, copper, manganese, selenium)
SELECT id, 264.0, 270.0, 481.0, 733.0, 3.7, 3.1, 1.0, 2.2, 2.5
FROM products WHERE slug = 'almonds'
ON CONFLICT (product_id) DO NOTHING;

-- Walnuts
INSERT INTO nutrition_minerals (product_id, magnesium, phosphorus, potassium, iron, zinc, copper, manganese, calcium, selenium)
SELECT id, 158.0, 346.0, 441.0, 2.9, 3.1, 1.6, 3.4, 98.0, 4.9
FROM products WHERE slug = 'walnuts'
ON CONFLICT (product_id) DO NOTHING;

-- Garlic
INSERT INTO nutrition_minerals (product_id, calcium, phosphorus, potassium, iron, zinc, magnesium, selenium, manganese)
SELECT id, 181.0, 153.0, 401.0, 1.7, 1.2, 25.0, 14.2, 1.7
FROM products WHERE slug = 'garlic'
ON CONFLICT (product_id) DO NOTHING;

-- ── Step 5: nutrition_fatty_acids ─────────────────────────────────────────────
-- Salmon: high omega-3
INSERT INTO nutrition_fatty_acids (product_id, saturated_fat, monounsaturated_fat, polyunsaturated_fat, omega3, omega6, epa, dha)
SELECT id, 3.1, 3.8, 5.9, 2.2, 0.4, 0.86, 1.24
FROM products WHERE slug = 'salmon'
ON CONFLICT (product_id) DO NOTHING;

-- Mackerel
INSERT INTO nutrition_fatty_acids (product_id, saturated_fat, monounsaturated_fat, polyunsaturated_fat, omega3, omega6, epa, dha)
SELECT id, 3.3, 5.3, 3.3, 2.7, 0.2, 0.90, 1.40
FROM products WHERE slug = 'mackerel'
ON CONFLICT (product_id) DO NOTHING;

-- Herring
INSERT INTO nutrition_fatty_acids (product_id, saturated_fat, monounsaturated_fat, polyunsaturated_fat, omega3, omega6, epa, dha)
SELECT id, 2.1, 3.7, 2.1, 1.7, 0.1, 0.71, 0.87
FROM products WHERE slug = 'herring'
ON CONFLICT (product_id) DO NOTHING;

-- Tuna
INSERT INTO nutrition_fatty_acids (product_id, saturated_fat, monounsaturated_fat, polyunsaturated_fat, omega3, omega6, epa, dha)
SELECT id, 1.3, 0.9, 1.6, 1.4, 0.04, 0.36, 0.89
FROM products WHERE slug = 'tuna'
ON CONFLICT (product_id) DO NOTHING;

-- Trout
INSERT INTO nutrition_fatty_acids (product_id, saturated_fat, monounsaturated_fat, polyunsaturated_fat, omega3, omega6, epa, dha)
SELECT id, 1.7, 2.1, 2.0, 1.1, 0.3, 0.35, 0.65
FROM products WHERE slug = 'trout'
ON CONFLICT (product_id) DO NOTHING;

-- Cod: lean fish
INSERT INTO nutrition_fatty_acids (product_id, saturated_fat, monounsaturated_fat, polyunsaturated_fat, omega3, omega6, epa, dha)
SELECT id, 0.1, 0.1, 0.2, 0.2, 0.01, 0.07, 0.10
FROM products WHERE slug = 'cod'
ON CONFLICT (product_id) DO NOTHING;

-- Avocado: monounsaturated
INSERT INTO nutrition_fatty_acids (product_id, saturated_fat, monounsaturated_fat, polyunsaturated_fat, omega3, omega6)
SELECT id, 2.1, 9.8, 1.8, 0.11, 1.67
FROM products WHERE slug = 'avocado'
ON CONFLICT (product_id) DO NOTHING;

-- Olive oil
INSERT INTO nutrition_fatty_acids (product_id, saturated_fat, monounsaturated_fat, polyunsaturated_fat, omega3, omega6)
SELECT id, 13.8, 72.9, 10.5, 0.76, 9.76
FROM products WHERE slug = 'olive-oil'
ON CONFLICT (product_id) DO NOTHING;

-- Almonds
INSERT INTO nutrition_fatty_acids (product_id, saturated_fat, monounsaturated_fat, polyunsaturated_fat, omega3, omega6)
SELECT id, 3.8, 31.6, 12.3, 0.006, 12.1
FROM products WHERE slug = 'almonds'
ON CONFLICT (product_id) DO NOTHING;

-- Walnuts: high omega-3
INSERT INTO nutrition_fatty_acids (product_id, saturated_fat, monounsaturated_fat, polyunsaturated_fat, omega3, omega6)
SELECT id, 6.1, 8.9, 47.2, 9.1, 38.1
FROM products WHERE slug = 'walnuts'
ON CONFLICT (product_id) DO NOTHING;

-- Butter
INSERT INTO nutrition_fatty_acids (product_id, saturated_fat, monounsaturated_fat, polyunsaturated_fat, omega3, omega6)
SELECT id, 51.4, 21.0, 3.0, 0.3, 2.7
FROM products WHERE slug = 'butter'
ON CONFLICT (product_id) DO NOTHING;

-- ── Step 6: diet_flags ────────────────────────────────────────────────────────
-- Vegetables: vegan + vegetarian + various
INSERT INTO diet_flags (product_id, vegan, vegetarian, gluten_free, mediterranean, low_carb)
SELECT ci.id, true, true, true, true,
    CASE WHEN ci.carbs_per_100g::real < 10 THEN true ELSE false END
FROM catalog_ingredients ci
JOIN products p ON p.id = ci.id
WHERE ci.product_type = 'vegetable' AND ci.is_active = true
ON CONFLICT (product_id) DO NOTHING;

-- Fruits: vegan + vegetarian
INSERT INTO diet_flags (product_id, vegan, vegetarian, gluten_free)
SELECT ci.id, true, true, true
FROM catalog_ingredients ci
JOIN products p ON p.id = ci.id
WHERE ci.product_type = 'fruit' AND ci.is_active = true
ON CONFLICT (product_id) DO NOTHING;

-- Grains: vegetarian (most gluten-containing)
INSERT INTO diet_flags (product_id, vegan, vegetarian, gluten_free, low_carb)
SELECT ci.id,
    true, true,
    CASE WHEN ci.slug IN ('rice','buckwheat','oatmeal') THEN true ELSE false END,
    false
FROM catalog_ingredients ci
JOIN products p ON p.id = ci.id
WHERE ci.product_type = 'grain' AND ci.is_active = true
ON CONFLICT (product_id) DO NOTHING;

-- Legumes: vegan + vegetarian + gluten_free
INSERT INTO diet_flags (product_id, vegan, vegetarian, gluten_free, mediterranean)
SELECT ci.id, true, true, true, true
FROM catalog_ingredients ci
JOIN products p ON p.id = ci.id
WHERE ci.product_type = 'legume' AND ci.is_active = true
ON CONFLICT (product_id) DO NOTHING;

-- Nuts: vegan + keto + paleo
INSERT INTO diet_flags (product_id, vegan, vegetarian, gluten_free, keto, paleo, low_carb)
SELECT ci.id, true, true, true, true, true, true
FROM catalog_ingredients ci
JOIN products p ON p.id = ci.id
WHERE ci.product_type = 'nut' AND ci.is_active = true
ON CONFLICT (product_id) DO NOTHING;

-- Seafood: gluten_free + paleo + mediterranean + keto (lean), low_carb
INSERT INTO diet_flags (product_id, vegan, vegetarian, gluten_free, paleo, mediterranean, keto, low_carb)
SELECT ci.id, false, false, true, true, true, true, true
FROM catalog_ingredients ci
JOIN products p ON p.id = ci.id
WHERE ci.product_type = 'seafood' AND ci.is_active = true
ON CONFLICT (product_id) DO NOTHING;

-- Meat: gluten_free + paleo + keto + low_carb
INSERT INTO diet_flags (product_id, vegan, vegetarian, gluten_free, paleo, keto, low_carb)
SELECT ci.id, false, false, true, true, true, true
FROM catalog_ingredients ci
JOIN products p ON p.id = ci.id
WHERE ci.product_type = 'meat' AND ci.is_active = true
ON CONFLICT (product_id) DO NOTHING;

-- Dairy: vegetarian + gluten_free
INSERT INTO diet_flags (product_id, vegan, vegetarian, gluten_free)
SELECT ci.id, false, true, true
FROM catalog_ingredients ci
JOIN products p ON p.id = ci.id
WHERE ci.product_type = 'dairy' AND ci.is_active = true
ON CONFLICT (product_id) DO NOTHING;

-- Spices/herbs: vegan
INSERT INTO diet_flags (product_id, vegan, vegetarian, gluten_free)
SELECT ci.id, true, true, true
FROM catalog_ingredients ci
JOIN products p ON p.id = ci.id
WHERE ci.product_type = 'spice' AND ci.is_active = true
ON CONFLICT (product_id) DO NOTHING;

-- ── Step 7: product_allergens ─────────────────────────────────────────────────
-- Map allergen_type[] array from catalog_ingredients to boolean columns
INSERT INTO product_allergens (product_id, milk, fish, shellfish, nuts, soy, gluten, eggs)
SELECT
    ci.id,
    ('milk'     = ANY(ci.allergens::text[])),
    ('fish'     = ANY(ci.allergens::text[])),
    ('shellfish'= ANY(ci.allergens::text[])),
    ('nuts'     = ANY(ci.allergens::text[])),
    ('soy'      = ANY(ci.allergens::text[])),
    ('gluten'   = ANY(ci.allergens::text[])),
    ('eggs'     = ANY(ci.allergens::text[]))
FROM catalog_ingredients ci
JOIN products p ON p.id = ci.id
WHERE ci.is_active = true
ON CONFLICT (product_id) DO NOTHING;

-- ── Step 8: food_properties — GI/GL ──────────────────────────────────────────
INSERT INTO food_properties (product_id, glycemic_index, glycemic_load, ph)
VALUES
-- vegetables
((SELECT id FROM products WHERE slug='potato'),       78, 14.4, 6.1),
((SELECT id FROM products WHERE slug='carrot'),       35,  4.0, 6.0),
((SELECT id FROM products WHERE slug='tomato'),       15,  0.7, 4.3),
((SELECT id FROM products WHERE slug='cucumber'),     15,  0.5, 6.0),
((SELECT id FROM products WHERE slug='broccoli'),     15,  1.5, 6.5),
((SELECT id FROM products WHERE slug='spinach'),      15,  0.4, 6.8),
((SELECT id FROM products WHERE slug='garlic'),       30,  1.5, 6.0),
((SELECT id FROM products WHERE slug='onion'),        15,  1.2, 5.5),
((SELECT id FROM products WHERE slug='bell-pepper'),  15,  1.2, 6.0),
-- fruits
((SELECT id FROM products WHERE slug='apple'),        38,  6.2, 3.5),
((SELECT id FROM products WHERE slug='banana'),       51, 12.4, 5.0),
((SELECT id FROM products WHERE slug='orange'),       40,  5.5, 3.7),
((SELECT id FROM products WHERE slug='strawberry'),   40,  3.6, 3.5),
((SELECT id FROM products WHERE slug='blueberry'),    53,  5.5, 3.1),
((SELECT id FROM products WHERE slug='grape'),        59, 11.3, 3.5),
((SELECT id FROM products WHERE slug='watermelon'),   72,  4.3, 5.5),
((SELECT id FROM products WHERE slug='avocado'),       15,  0.5, 6.3),
-- grains
((SELECT id FROM products WHERE slug='rice'),         73, 22.1, 7.0),
((SELECT id FROM products WHERE slug='pasta'),        50, 21.9, 6.0),
((SELECT id FROM products WHERE slug='oatmeal'),      55, 13.0, 6.2),
((SELECT id FROM products WHERE slug='buckwheat'),    51, 16.0, 6.0),
((SELECT id FROM products WHERE slug='wheat-flour'),  85, 71.5, 6.5),
((SELECT id FROM products WHERE slug='bread'),        75, 40.0, 5.5),
-- protein
((SELECT id FROM products WHERE slug='chicken-breast'), 0, 0.0, 6.0),
((SELECT id FROM products WHERE slug='beef'),           0, 0.0, 6.0),
((SELECT id FROM products WHERE slug='salmon'),         0, 0.0, 6.2),
((SELECT id FROM products WHERE slug='tuna'),           0, 0.0, 6.0),
((SELECT id FROM products WHERE slug='chicken-eggs'),  0, 0.0, 7.6),
-- dairy
((SELECT id FROM products WHERE slug='milk'),          27, 3.1, 6.7),
((SELECT id FROM products WHERE slug='butter'),         0, 0.0, 7.0),
-- oils
((SELECT id FROM products WHERE slug='olive-oil'),      0, 0.0, 7.0),
-- legumes
((SELECT id FROM products WHERE slug='chickpeas'),     28, 7.8, 7.0),
((SELECT id FROM products WHERE slug='lentils'),       32, 5.7, 6.8),
((SELECT id FROM products WHERE slug='beans'),         30, 7.0, 6.5),
-- sugar
((SELECT id FROM products WHERE slug='sugar'),        68, 68.0, 7.0),
((SELECT id FROM products WHERE slug='honey'),        55, 45.4, 3.9)
ON CONFLICT (product_id) DO NOTHING;

-- ── Step 9: food_culinary_properties ─────────────────────────────────────────
INSERT INTO food_culinary_properties (product_id, sweetness, acidity, bitterness, umami, aroma, texture)
VALUES
((SELECT id FROM products WHERE slug='salmon'),        1.0, 0.5, 0.5, 7.0, 7.0, 'tender'),
((SELECT id FROM products WHERE slug='tuna'),          1.0, 0.5, 0.5, 8.0, 6.0, 'firm'),
((SELECT id FROM products WHERE slug='mackerel'),      2.0, 0.5, 1.0, 7.5, 8.0, 'oily'),
((SELECT id FROM products WHERE slug='herring'),       1.5, 1.5, 0.5, 7.0, 8.0, 'soft'),
((SELECT id FROM products WHERE slug='cod'),           1.0, 0.5, 0.5, 5.0, 5.0, 'flaky'),
((SELECT id FROM products WHERE slug='shrimp'),        3.0, 0.5, 0.0, 7.0, 6.0, 'crunchy'),
((SELECT id FROM products WHERE slug='carp'),          1.5, 0.5, 1.0, 6.0, 5.0, 'firm'),
((SELECT id FROM products WHERE slug='trout'),         1.5, 0.5, 0.5, 6.5, 7.0, 'tender'),
((SELECT id FROM products WHERE slug='chicken-breast'),1.5, 0.0, 0.0, 5.0, 5.0, 'tender'),
((SELECT id FROM products WHERE slug='beef'),          1.0, 0.0, 1.0, 9.0, 8.0, 'chewy'),
((SELECT id FROM products WHERE slug='pork'),          2.0, 0.0, 0.5, 7.0, 7.0, 'tender'),
((SELECT id FROM products WHERE slug='tomato'),        3.5, 4.5, 1.0, 5.0, 6.0, 'juicy'),
((SELECT id FROM products WHERE slug='lemon'),         1.0, 9.0, 1.0, 0.0, 7.0, 'juicy'),
((SELECT id FROM products WHERE slug='garlic'),        2.0, 0.5, 2.0, 4.0, 9.0, 'firm'),
((SELECT id FROM products WHERE slug='onion'),         4.0, 1.0, 2.0, 5.0, 8.0, 'crunchy'),
((SELECT id FROM products WHERE slug='avocado'),       1.0, 0.5, 0.5, 2.0, 4.0, 'creamy'),
((SELECT id FROM products WHERE slug='banana'),        9.0, 0.5, 0.0, 0.0, 7.0, 'soft'),
((SELECT id FROM products WHERE slug='apple'),         7.0, 3.0, 1.0, 0.0, 6.0, 'crunchy'),
((SELECT id FROM products WHERE slug='honey'),         9.5, 0.5, 0.0, 0.0, 8.0, 'viscous'),
((SELECT id FROM products WHERE slug='sugar'),         9.5, 0.0, 0.0, 0.0, 0.0, 'granular'),
((SELECT id FROM products WHERE slug='butter'),        2.0, 0.5, 0.0, 3.0, 5.0, 'creamy'),
((SELECT id FROM products WHERE slug='olive-oil'),     1.0, 0.5, 2.0, 2.0, 6.0, 'liquid'),
((SELECT id FROM products WHERE slug='ginger'),        2.0, 1.5, 3.0, 1.0, 9.0, 'fibrous'),
((SELECT id FROM products WHERE slug='black-pepper'),  0.5, 0.0, 6.0, 1.0, 9.0, 'granular'),
((SELECT id FROM products WHERE slug='cinnamon'),      4.0, 0.0, 2.0, 0.0, 9.0, 'powdery'),
((SELECT id FROM products WHERE slug='chocolate'),     8.0, 0.5, 4.0, 2.0, 9.0, 'solid'),
((SELECT id FROM products WHERE slug='mozzarella-cheese'), 2.0, 1.0, 1.5, 8.0, 7.0, 'firm'),
((SELECT id FROM products WHERE slug='hard-cheese'),       2.5, 1.0, 1.5, 8.5, 7.0, 'firm'),
((SELECT id FROM products WHERE slug='broccoli'),      1.5, 0.5, 2.5, 3.0, 5.0, 'crisp'),
((SELECT id FROM products WHERE slug='spinach'),       1.0, 0.5, 2.0, 2.0, 4.0, 'tender'),
((SELECT id FROM products WHERE slug='potato'),        2.0, 0.5, 0.5, 2.0, 3.0, 'starchy'),
((SELECT id FROM products WHERE slug='carrot'),        5.0, 0.5, 0.5, 1.5, 5.0, 'crunchy'),
((SELECT id FROM products WHERE slug='rice'),          1.0, 0.0, 0.5, 1.0, 2.0, 'fluffy'),
((SELECT id FROM products WHERE slug='pasta'),         1.0, 0.0, 0.5, 1.0, 2.0, 'elastic'),
((SELECT id FROM products WHERE slug='buckwheat'),     1.5, 0.0, 2.0, 2.0, 4.0, 'nutty'),
((SELECT id FROM products WHERE slug='oatmeal'),       2.5, 0.0, 1.0, 1.0, 3.0, 'creamy'),
((SELECT id FROM products WHERE slug='basil'),         1.5, 0.5, 1.5, 1.0, 9.0, 'tender'),
((SELECT id FROM products WHERE slug='parsley'),       1.0, 0.5, 2.0, 1.0, 8.0, 'tender'),
((SELECT id FROM products WHERE slug='dill'),          1.0, 0.5, 1.5, 0.5, 9.0, 'feathery'),
((SELECT id FROM products WHERE slug='rosemary'),      1.0, 0.5, 3.0, 1.0, 9.0, 'woody'),
((SELECT id FROM products WHERE slug='milk'),          3.5, 0.0, 0.0, 1.0, 3.0, 'liquid'),
((SELECT id FROM products WHERE slug='chicken-eggs'),  2.0, 0.0, 0.0, 4.0, 4.0, 'creamy'),
((SELECT id FROM products WHERE slug='soy-sauce'),     1.0, 1.0, 1.0, 9.0, 8.0, 'liquid'),
((SELECT id FROM products WHERE slug='vinegar'),       0.0,10.0, 0.5, 0.0, 5.0, 'liquid'),
((SELECT id FROM products WHERE slug='mustard'),       2.0, 2.0, 4.0, 2.0, 7.0, 'paste'),
((SELECT id FROM products WHERE slug='ketchup'),       5.0, 3.5, 0.5, 3.0, 6.0, 'paste')
ON CONFLICT (product_id) DO NOTHING;

-- ── Step 10: food_pairing — classic combinations ──────────────────────────────
INSERT INTO food_pairing (ingredient_a, ingredient_b, flavor_score, nutrition_score, culinary_score, pair_score)
SELECT a.id, b.id, fs, ns, cs, (fs+ns+cs)/3.0
FROM (VALUES
    ('salmon',         'lemon',          9.0, 7.0, 9.5),
    ('salmon',         'avocado',        8.5, 9.0, 9.0),
    ('salmon',         'garlic',         8.0, 8.0, 9.0),
    ('salmon',         'dill',           9.5, 7.0, 9.5),
    ('tuna',           'avocado',        8.5, 9.5, 8.5),
    ('tuna',           'soy-sauce',      9.0, 6.0, 9.0),
    ('tuna',           'lemon',          9.0, 7.5, 9.0),
    ('mackerel',       'lemon',          9.5, 7.0, 9.5),
    ('mackerel',       'mustard',        8.0, 6.0, 8.5),
    ('herring',        'onion',          9.0, 6.0, 9.5),
    ('shrimp',         'garlic',         9.5, 7.5, 9.5),
    ('shrimp',         'lemon',          9.5, 7.5, 9.5),
    ('chicken-breast', 'garlic',         9.0, 8.0, 9.0),
    ('chicken-breast', 'lemon',          8.5, 7.5, 8.5),
    ('chicken-breast', 'rosemary',       9.0, 7.0, 9.0),
    ('beef',           'garlic',         9.0, 7.5, 9.0),
    ('beef',           'black-pepper',   9.5, 6.0, 9.5),
    ('beef',           'rosemary',       9.0, 7.0, 9.0),
    ('tomato',         'basil',          9.5, 8.5, 9.5),
    ('tomato',         'garlic',         9.0, 8.0, 9.0),
    ('avocado',        'lemon',          8.5, 9.0, 9.0),
    ('avocado',        'garlic',         8.5, 8.5, 8.5),
    ('garlic',         'olive-oil',      8.5, 8.5, 9.5),
    ('broccoli',       'garlic',         8.5, 9.0, 9.0),
    ('spinach',        'garlic',         8.5, 9.5, 9.0),
    ('spinach',        'lemon',          8.0, 9.0, 8.5),
    ('carrot',         'ginger',         8.5, 8.5, 9.0),
    ('honey',          'lemon',          9.0, 7.5, 9.5),
    ('banana',         'honey',          8.5, 6.0, 8.5),
    ('apple',          'cinnamon',       9.5, 7.0, 9.5),
    ('chocolate',      'raspberry',      9.5, 8.0, 9.5),
    ('chocolate',      'orange',         9.0, 8.0, 9.0),
    ('pasta',          'tomato',         9.0, 7.5, 9.5),
    ('pasta',          'garlic',         9.0, 7.5, 9.5),
    ('rice',           'soy-sauce',      8.5, 6.0, 9.0),
    ('potato',         'rosemary',       8.5, 6.0, 9.0),
    ('potato',         'garlic',         9.0, 7.5, 9.0),
    ('eggs',           'butter',         8.5, 5.0, 9.0),
    ('cheese',         'tomato',         9.0, 7.0, 9.5),
    ('milk',           'honey',          8.0, 7.0, 9.0)
) AS t(a_slug, b_slug, fs, ns, cs)
JOIN products a ON a.slug = t.a_slug
JOIN products b ON b.slug = t.b_slug
ON CONFLICT (ingredient_a, ingredient_b) DO NOTHING;
