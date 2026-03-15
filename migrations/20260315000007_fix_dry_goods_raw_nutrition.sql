-- Fix DryGoods products that had COOKED values instead of RAW
-- All nutrition in catalog_ingredients must be per 100g RAW product
-- so that the state transform algorithm can correctly compute cooked values.

-- Rice: USDA raw long-grain white rice
UPDATE catalog_ingredients SET
    calories_per_100g = 360,
    protein_per_100g = 7.1,
    fat_per_100g = 0.66,
    carbs_per_100g = 80.0,
    fiber_per_100g = 1.3
WHERE slug = 'rice';

-- Beans (dry kidney beans, USDA raw): 333 kcal
UPDATE catalog_ingredients SET
    calories_per_100g = 333,
    protein_per_100g = 23.6,
    fat_per_100g = 0.8,
    carbs_per_100g = 60.0,
    fiber_per_100g = 15.2
WHERE slug = 'beans';

-- Chickpeas (dry, USDA raw): 364 kcal
UPDATE catalog_ingredients SET
    calories_per_100g = 364,
    protein_per_100g = 19.3,
    fat_per_100g = 6.0,
    carbs_per_100g = 61.0,
    fiber_per_100g = 17.4
WHERE slug = 'chickpeas';

-- Lentils (dry, USDA raw): 353 kcal
UPDATE catalog_ingredients SET
    calories_per_100g = 353,
    protein_per_100g = 25.8,
    fat_per_100g = 1.1,
    carbs_per_100g = 63.0,
    fiber_per_100g = 10.7
WHERE slug = 'lentils';
