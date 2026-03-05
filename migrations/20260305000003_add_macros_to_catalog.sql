-- Add protein, fat, carbs columns to catalog_ingredients
-- so that nutrition data lives in one place (DB) with images & translations

ALTER TABLE catalog_ingredients
ADD COLUMN IF NOT EXISTS protein_per_100g NUMERIC,
ADD COLUMN IF NOT EXISTS fat_per_100g     NUMERIC,
ADD COLUMN IF NOT EXISTS carbs_per_100g   NUMERIC;

-- ── Populate from NUTRITION_TABLE data ──
-- Dairy & Eggs
UPDATE catalog_ingredients SET protein_per_100g = 3.4,  fat_per_100g = 1.0,   carbs_per_100g = 5.0   WHERE LOWER(name_en) LIKE '%milk%'           AND protein_per_100g IS NULL;
UPDATE catalog_ingredients SET protein_per_100g = 2.1,  fat_per_100g = 37.0,  carbs_per_100g = 2.8   WHERE LOWER(name_en) LIKE '%cream%'          AND protein_per_100g IS NULL AND LOWER(name_en) NOT LIKE '%sour%' AND LOWER(name_en) NOT LIKE '%ice%';
UPDATE catalog_ingredients SET protein_per_100g = 0.9,  fat_per_100g = 81.0,  carbs_per_100g = 0.1   WHERE LOWER(name_en) LIKE '%butter%'         AND protein_per_100g IS NULL;
UPDATE catalog_ingredients SET protein_per_100g = 13.0, fat_per_100g = 11.0,  carbs_per_100g = 1.1   WHERE LOWER(name_en) LIKE '%egg%'            AND protein_per_100g IS NULL;
UPDATE catalog_ingredients SET protein_per_100g = 25.0, fat_per_100g = 33.0,  carbs_per_100g = 1.3   WHERE LOWER(name_en) LIKE '%cheese%'         AND protein_per_100g IS NULL AND LOWER(name_en) NOT LIKE '%mozzarella%' AND LOWER(name_en) NOT LIKE '%parmesan%';
UPDATE catalog_ingredients SET protein_per_100g = 10.0, fat_per_100g = 0.7,   carbs_per_100g = 3.6   WHERE LOWER(name_en) LIKE '%yogurt%'         AND protein_per_100g IS NULL;
UPDATE catalog_ingredients SET protein_per_100g = 2.4,  fat_per_100g = 19.0,  carbs_per_100g = 3.4   WHERE LOWER(name_en) LIKE '%sour cream%'     AND protein_per_100g IS NULL;
UPDATE catalog_ingredients SET protein_per_100g = 38.5, fat_per_100g = 29.0,  carbs_per_100g = 4.1   WHERE LOWER(name_en) LIKE '%parmesan%'       AND protein_per_100g IS NULL;

-- Meat & Fish
UPDATE catalog_ingredients SET protein_per_100g = 20.0, fat_per_100g = 13.0,  carbs_per_100g = 0.0   WHERE LOWER(name_en) LIKE '%salmon%'         AND protein_per_100g IS NULL;
UPDATE catalog_ingredients SET protein_per_100g = 31.0, fat_per_100g = 3.6,   carbs_per_100g = 0.0   WHERE LOWER(name_en) LIKE '%chicken%breast%' AND protein_per_100g IS NULL;
UPDATE catalog_ingredients SET protein_per_100g = 26.0, fat_per_100g = 15.0,  carbs_per_100g = 0.0   WHERE LOWER(name_en) LIKE '%beef%'           AND protein_per_100g IS NULL;

-- Vegetables
UPDATE catalog_ingredients SET protein_per_100g = 2.0,  fat_per_100g = 0.1,   carbs_per_100g = 17.0  WHERE LOWER(name_en) LIKE '%potato%'         AND protein_per_100g IS NULL;
UPDATE catalog_ingredients SET protein_per_100g = 0.9,  fat_per_100g = 0.2,   carbs_per_100g = 3.9   WHERE LOWER(name_en) LIKE '%tomato%'         AND protein_per_100g IS NULL AND LOWER(name_en) NOT LIKE '%paste%' AND LOWER(name_en) NOT LIKE '%sauce%';
UPDATE catalog_ingredients SET protein_per_100g = 1.1,  fat_per_100g = 0.1,   carbs_per_100g = 9.3   WHERE LOWER(name_en) LIKE '%onion%'          AND protein_per_100g IS NULL;
UPDATE catalog_ingredients SET protein_per_100g = 6.4,  fat_per_100g = 0.5,   carbs_per_100g = 33.0  WHERE LOWER(name_en) LIKE '%garlic%'         AND protein_per_100g IS NULL;
UPDATE catalog_ingredients SET protein_per_100g = 0.9,  fat_per_100g = 0.2,   carbs_per_100g = 10.0  WHERE LOWER(name_en) LIKE '%carrot%'         AND protein_per_100g IS NULL;
UPDATE catalog_ingredients SET protein_per_100g = 2.8,  fat_per_100g = 0.4,   carbs_per_100g = 7.0   WHERE LOWER(name_en) LIKE '%broccoli%'       AND protein_per_100g IS NULL;
UPDATE catalog_ingredients SET protein_per_100g = 2.9,  fat_per_100g = 0.4,   carbs_per_100g = 3.6   WHERE LOWER(name_en) LIKE '%spinach%'        AND protein_per_100g IS NULL;
UPDATE catalog_ingredients SET protein_per_100g = 1.1,  fat_per_100g = 0.3,   carbs_per_100g = 9.3   WHERE LOWER(name_en) LIKE '%lemon%'          AND protein_per_100g IS NULL;
UPDATE catalog_ingredients SET protein_per_100g = 2.0,  fat_per_100g = 15.0,  carbs_per_100g = 9.0   WHERE LOWER(name_en) LIKE '%avocado%'        AND protein_per_100g IS NULL;
UPDATE catalog_ingredients SET protein_per_100g = 1.8,  fat_per_100g = 0.8,   carbs_per_100g = 17.8  WHERE LOWER(name_en) LIKE '%ginger%'         AND protein_per_100g IS NULL;

-- Grains & Baking
UPDATE catalog_ingredients SET protein_per_100g = 2.7,  fat_per_100g = 0.3,   carbs_per_100g = 28.0  WHERE LOWER(name_en) LIKE '%rice%'           AND protein_per_100g IS NULL;
UPDATE catalog_ingredients SET protein_per_100g = 13.0, fat_per_100g = 1.5,   carbs_per_100g = 74.0  WHERE LOWER(name_en) LIKE '%pasta%'          AND protein_per_100g IS NULL;
UPDATE catalog_ingredients SET protein_per_100g = 10.0, fat_per_100g = 1.0,   carbs_per_100g = 76.0  WHERE LOWER(name_en) LIKE '%flour%'          AND protein_per_100g IS NULL;
UPDATE catalog_ingredients SET protein_per_100g = 16.9, fat_per_100g = 6.9,   carbs_per_100g = 66.3  WHERE LOWER(name_en) LIKE '%oat%'            AND protein_per_100g IS NULL;
UPDATE catalog_ingredients SET protein_per_100g = 13.4, fat_per_100g = 5.3,   carbs_per_100g = 71.0  WHERE LOWER(name_en) LIKE '%breadcrumb%'     AND protein_per_100g IS NULL;
UPDATE catalog_ingredients SET protein_per_100g = 0.3,  fat_per_100g = 0.1,   carbs_per_100g = 91.3  WHERE LOWER(name_en) LIKE '%corn starch%'    AND protein_per_100g IS NULL;

-- Sweeteners
UPDATE catalog_ingredients SET protein_per_100g = 0.0,  fat_per_100g = 0.0,   carbs_per_100g = 100.0 WHERE LOWER(name_en) = 'sugar'               AND protein_per_100g IS NULL;
UPDATE catalog_ingredients SET protein_per_100g = 0.1,  fat_per_100g = 0.0,   carbs_per_100g = 98.0  WHERE LOWER(name_en) LIKE '%brown sugar%'    AND protein_per_100g IS NULL;
UPDATE catalog_ingredients SET protein_per_100g = 0.0,  fat_per_100g = 0.0,   carbs_per_100g = 100.0 WHERE LOWER(name_en) LIKE '%powdered sugar%' AND protein_per_100g IS NULL;
UPDATE catalog_ingredients SET protein_per_100g = 0.3,  fat_per_100g = 0.0,   carbs_per_100g = 82.0  WHERE LOWER(name_en) LIKE '%honey%'          AND protein_per_100g IS NULL;
UPDATE catalog_ingredients SET protein_per_100g = 0.0,  fat_per_100g = 0.1,   carbs_per_100g = 67.0  WHERE LOWER(name_en) LIKE '%maple syrup%'    AND protein_per_100g IS NULL;

-- Oils & Fats
UPDATE catalog_ingredients SET protein_per_100g = 0.0,  fat_per_100g = 100.0, carbs_per_100g = 0.0   WHERE LOWER(name_en) LIKE '%olive oil%'      AND protein_per_100g IS NULL;
UPDATE catalog_ingredients SET protein_per_100g = 0.0,  fat_per_100g = 100.0, carbs_per_100g = 0.0   WHERE LOWER(name_en) LIKE '%coconut oil%'    AND protein_per_100g IS NULL;
UPDATE catalog_ingredients SET protein_per_100g = 0.0,  fat_per_100g = 100.0, carbs_per_100g = 0.0   WHERE LOWER(name_en) LIKE '%sesame oil%'     AND protein_per_100g IS NULL;
UPDATE catalog_ingredients SET protein_per_100g = 0.0,  fat_per_100g = 100.0, carbs_per_100g = 0.0   WHERE LOWER(name_en) LIKE '%lard%'           AND protein_per_100g IS NULL;
UPDATE catalog_ingredients SET protein_per_100g = 0.0,  fat_per_100g = 100.0, carbs_per_100g = 0.0   WHERE LOWER(name_en) LIKE '%sunflower oil%'  AND protein_per_100g IS NULL;
UPDATE catalog_ingredients SET protein_per_100g = 0.0,  fat_per_100g = 100.0, carbs_per_100g = 0.0   WHERE LOWER(name_en) LIKE '%vegetable oil%'  AND protein_per_100g IS NULL;

-- Nuts
UPDATE catalog_ingredients SET protein_per_100g = 15.0, fat_per_100g = 65.0,  carbs_per_100g = 14.0  WHERE LOWER(name_en) LIKE '%walnut%'         AND protein_per_100g IS NULL;
UPDATE catalog_ingredients SET protein_per_100g = 21.0, fat_per_100g = 50.0,  carbs_per_100g = 22.0  WHERE LOWER(name_en) LIKE '%almond%'         AND protein_per_100g IS NULL;
UPDATE catalog_ingredients SET protein_per_100g = 25.0, fat_per_100g = 50.0,  carbs_per_100g = 20.0  WHERE LOWER(name_en) LIKE '%peanut butter%'  AND protein_per_100g IS NULL;

-- Condiments & Sauces
UPDATE catalog_ingredients SET protein_per_100g = 0.0,  fat_per_100g = 0.0,   carbs_per_100g = 0.0   WHERE LOWER(name_en) LIKE '%salt%'           AND protein_per_100g IS NULL AND LOWER(name_en) NOT LIKE '%sauce%';
UPDATE catalog_ingredients SET protein_per_100g = 0.0,  fat_per_100g = 0.0,   carbs_per_100g = 0.0   WHERE LOWER(name_en) = 'water'               AND protein_per_100g IS NULL;
UPDATE catalog_ingredients SET protein_per_100g = 8.1,  fat_per_100g = 0.6,   carbs_per_100g = 4.9   WHERE LOWER(name_en) LIKE '%soy sauce%'      AND protein_per_100g IS NULL;
UPDATE catalog_ingredients SET protein_per_100g = 0.0,  fat_per_100g = 0.0,   carbs_per_100g = 0.9   WHERE LOWER(name_en) LIKE '%vinegar%'        AND protein_per_100g IS NULL;
UPDATE catalog_ingredients SET protein_per_100g = 1.7,  fat_per_100g = 0.1,   carbs_per_100g = 27.4  WHERE LOWER(name_en) LIKE '%ketchup%'        AND protein_per_100g IS NULL;
UPDATE catalog_ingredients SET protein_per_100g = 1.0,  fat_per_100g = 75.0,  carbs_per_100g = 0.6   WHERE LOWER(name_en) LIKE '%mayonnaise%'     AND protein_per_100g IS NULL;
UPDATE catalog_ingredients SET protein_per_100g = 4.4,  fat_per_100g = 4.0,   carbs_per_100g = 5.8   WHERE LOWER(name_en) LIKE '%mustard%'        AND protein_per_100g IS NULL;

-- Spices
UPDATE catalog_ingredients SET protein_per_100g = 4.0,  fat_per_100g = 1.2,   carbs_per_100g = 80.6  WHERE LOWER(name_en) LIKE '%cinnamon%'       AND protein_per_100g IS NULL;
UPDATE catalog_ingredients SET protein_per_100g = 10.4, fat_per_100g = 3.3,   carbs_per_100g = 63.9  WHERE LOWER(name_en) LIKE '%black pepper%'   AND protein_per_100g IS NULL;
UPDATE catalog_ingredients SET protein_per_100g = 14.1, fat_per_100g = 13.0,  carbs_per_100g = 53.9  WHERE LOWER(name_en) LIKE '%paprika%'        AND protein_per_100g IS NULL;
UPDATE catalog_ingredients SET protein_per_100g = 17.8, fat_per_100g = 22.3,  carbs_per_100g = 44.2  WHERE LOWER(name_en) LIKE '%cumin%'          AND protein_per_100g IS NULL;
UPDATE catalog_ingredients SET protein_per_100g = 7.8,  fat_per_100g = 9.9,   carbs_per_100g = 64.9  WHERE LOWER(name_en) LIKE '%turmeric%'       AND protein_per_100g IS NULL;

-- Baking
UPDATE catalog_ingredients SET protein_per_100g = 19.6, fat_per_100g = 13.7,  carbs_per_100g = 57.9  WHERE LOWER(name_en) LIKE '%cocoa%'          AND protein_per_100g IS NULL;
UPDATE catalog_ingredients SET protein_per_100g = 0.0,  fat_per_100g = 0.0,   carbs_per_100g = 28.0  WHERE LOWER(name_en) LIKE '%baking powder%'  AND protein_per_100g IS NULL;
UPDATE catalog_ingredients SET protein_per_100g = 0.0,  fat_per_100g = 0.0,   carbs_per_100g = 0.0   WHERE LOWER(name_en) LIKE '%baking soda%'    AND protein_per_100g IS NULL;
UPDATE catalog_ingredients SET protein_per_100g = 0.1,  fat_per_100g = 0.1,   carbs_per_100g = 12.7  WHERE LOWER(name_en) LIKE '%vanilla%'        AND protein_per_100g IS NULL;
