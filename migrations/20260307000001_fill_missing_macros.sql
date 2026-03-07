-- Migration: fill missing protein/fat/carbs for all ingredients
-- Sources: USDA FoodData Central, standard nutritional databases
-- All values per 100g

-- ── Fruits ───────────────────────────────────────────────────────────────────
UPDATE catalog_ingredients SET protein_per_100g = 0.3, fat_per_100g = 0.2, carbs_per_100g = 14, calories_per_100g = 52 WHERE slug = 'apple';
UPDATE catalog_ingredients SET protein_per_100g = 3.3, fat_per_100g = 0.2, carbs_per_100g = 11, calories_per_100g = 47 WHERE slug = 'artichoke';
UPDATE catalog_ingredients SET protein_per_100g = 1.1, fat_per_100g = 0.3, carbs_per_100g = 23, calories_per_100g = 89 WHERE slug = 'banana';
UPDATE catalog_ingredients SET protein_per_100g = 0.7, fat_per_100g = 0.3, carbs_per_100g = 14, calories_per_100g = 57 WHERE slug = 'blueberry';
UPDATE catalog_ingredients SET protein_per_100g = 0.7, fat_per_100g = 0.2, carbs_per_100g = 18, calories_per_100g = 69 WHERE slug = 'grape';
UPDATE catalog_ingredients SET protein_per_100g = 0.9, fat_per_100g = 0.1, carbs_per_100g = 12, calories_per_100g = 47 WHERE slug = 'orange';
UPDATE catalog_ingredients SET protein_per_100g = 0.5, fat_per_100g = 0.1, carbs_per_100g = 13, calories_per_100g = 50 WHERE slug = 'pineapple';
UPDATE catalog_ingredients SET protein_per_100g = 1.2, fat_per_100g = 0.7, carbs_per_100g = 12, calories_per_100g = 52 WHERE slug = 'raspberry';
UPDATE catalog_ingredients SET protein_per_100g = 0.7, fat_per_100g = 0.3, carbs_per_100g = 8, calories_per_100g = 32 WHERE slug = 'strawberry';
UPDATE catalog_ingredients SET protein_per_100g = 0.6, fat_per_100g = 0.2, carbs_per_100g = 8, calories_per_100g = 30 WHERE slug = 'watermelon';

-- ── Vegetables & Legumes ─────────────────────────────────────────────────────
UPDATE catalog_ingredients SET protein_per_100g = 7.5, fat_per_100g = 0.5, carbs_per_100g = 22, calories_per_100g = 127 WHERE slug = 'beans';
UPDATE catalog_ingredients SET protein_per_100g = 1.3, fat_per_100g = 0.1, carbs_per_100g = 6, calories_per_100g = 25 WHERE slug = 'cabbage';
UPDATE catalog_ingredients SET protein_per_100g = 1.9, fat_per_100g = 0.3, carbs_per_100g = 5, calories_per_100g = 25 WHERE slug = 'cauliflower';
UPDATE catalog_ingredients SET protein_per_100g = 8.9, fat_per_100g = 2.6, carbs_per_100g = 27, calories_per_100g = 164 WHERE slug = 'chickpeas';
UPDATE catalog_ingredients SET protein_per_100g = 3.3, fat_per_100g = 1.4, carbs_per_100g = 19, calories_per_100g = 86 WHERE slug = 'corn';
UPDATE catalog_ingredients SET protein_per_100g = 5.4, fat_per_100g = 0.4, carbs_per_100g = 14, calories_per_100g = 81 WHERE slug = 'green-peas';
UPDATE catalog_ingredients SET protein_per_100g = 9.0, fat_per_100g = 0.4, carbs_per_100g = 20, calories_per_100g = 116 WHERE slug = 'lentils';
UPDATE catalog_ingredients SET protein_per_100g = 0.3, fat_per_100g = 0.2, carbs_per_100g = 2.3, calories_per_100g = 11 WHERE slug = 'pickles';
UPDATE catalog_ingredients SET protein_per_100g = 2.5, fat_per_100g = 0.3, carbs_per_100g = 12, calories_per_100g = 65 WHERE slug = 'frozen-vegetables';

-- ── Meat & Processed ─────────────────────────────────────────────────────────
UPDATE catalog_ingredients SET protein_per_100g = 37, fat_per_100g = 41.8, carbs_per_100g = 1.4, calories_per_100g = 541 WHERE slug = 'bacon';
UPDATE catalog_ingredients SET protein_per_100g = 17.4, fat_per_100g = 29.2, carbs_per_100g = 0.6 WHERE slug = 'ground-meat';
UPDATE catalog_ingredients SET protein_per_100g = 21.6, fat_per_100g = 5.5, carbs_per_100g = 1.5, calories_per_100g = 145 WHERE slug = 'ham';
UPDATE catalog_ingredients SET protein_per_100g = 12, fat_per_100g = 25, carbs_per_100g = 2.5, calories_per_100g = 301 WHERE slug = 'sausage';
UPDATE catalog_ingredients SET protein_per_100g = 12, fat_per_100g = 25, carbs_per_100g = 2.5, calories_per_100g = 301 WHERE slug = 'sausages';

-- ── Fish ─────────────────────────────────────────────────────────────────────
UPDATE catalog_ingredients SET protein_per_100g = 17.7, fat_per_100g = 9.0, carbs_per_100g = 0, calories_per_100g = 158 WHERE slug = 'herring';

-- ── Dairy ────────────────────────────────────────────────────────────────────
UPDATE catalog_ingredients SET protein_per_100g = 22.2, fat_per_100g = 22.4, carbs_per_100g = 2.2, calories_per_100g = 280 WHERE slug = 'mozzarella-cheese';

-- ── Grains & Baked ───────────────────────────────────────────────────────────
UPDATE catalog_ingredients SET protein_per_100g = 9.0, fat_per_100g = 3.2, carbs_per_100g = 49, calories_per_100g = 265 WHERE slug = 'bread';
UPDATE catalog_ingredients SET protein_per_100g = 13.3, fat_per_100g = 3.4, carbs_per_100g = 72, calories_per_100g = 343 WHERE slug = 'buckwheat';

-- ── Nuts & Seeds ─────────────────────────────────────────────────────────────
UPDATE catalog_ingredients SET protein_per_100g = 15, fat_per_100g = 60.8, carbs_per_100g = 17, calories_per_100g = 628 WHERE slug = 'hazelnuts';
UPDATE catalog_ingredients SET protein_per_100g = 17.7, fat_per_100g = 49.7, carbs_per_100g = 23.4, calories_per_100g = 573 WHERE slug = 'sesame-seeds';
UPDATE catalog_ingredients SET protein_per_100g = 20.8, fat_per_100g = 51.5, carbs_per_100g = 20, calories_per_100g = 584 WHERE slug = 'sunflower-seeds';

-- ── Sweets & Condiments ──────────────────────────────────────────────────────
UPDATE catalog_ingredients SET protein_per_100g = 5.0, fat_per_100g = 31, carbs_per_100g = 60, calories_per_100g = 546 WHERE slug = 'chocolate';
UPDATE catalog_ingredients SET protein_per_100g = 1.0, fat_per_100g = 11, carbs_per_100g = 4, calories_per_100g = 115 WHERE slug = 'olives';

-- ── Herbs & Spices ───────────────────────────────────────────────────────────
UPDATE catalog_ingredients SET protein_per_100g = 9.0, fat_per_100g = 4.3, carbs_per_100g = 69, calories_per_100g = 265 WHERE slug = 'oregano';
UPDATE catalog_ingredients SET protein_per_100g = 3.3, fat_per_100g = 5.9, carbs_per_100g = 13, calories_per_100g = 131 WHERE slug = 'rosemary';
UPDATE catalog_ingredients SET protein_per_100g = 5.6, fat_per_100g = 1.7, carbs_per_100g = 24, calories_per_100g = 101 WHERE slug = 'thyme';

-- ── Beverages ────────────────────────────────────────────────────────────────
UPDATE catalog_ingredients SET protein_per_100g = 0.5, fat_per_100g = 0, carbs_per_100g = 3.6, calories_per_100g = 43 WHERE slug = 'beer';
UPDATE catalog_ingredients SET protein_per_100g = 0, fat_per_100g = 0, carbs_per_100g = 0, calories_per_100g = 0 WHERE slug = 'mineral-water';
UPDATE catalog_ingredients SET protein_per_100g = 0.7, fat_per_100g = 0.2, carbs_per_100g = 10.4, calories_per_100g = 45 WHERE slug = 'orange-juice';
UPDATE catalog_ingredients SET protein_per_100g = 0.1, fat_per_100g = 0, carbs_per_100g = 2.6, calories_per_100g = 85 WHERE slug = 'red-wine';
UPDATE catalog_ingredients SET protein_per_100g = 0.1, fat_per_100g = 0, carbs_per_100g = 2.6, calories_per_100g = 82 WHERE slug = 'white-wine';
