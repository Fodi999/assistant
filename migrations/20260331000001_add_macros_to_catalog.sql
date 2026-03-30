-- Add macronutrient columns to catalog_ingredients
-- protein / fat / carbs per 100g — required for Sous-Chef planner
-- Date: 2026-03-31

ALTER TABLE catalog_ingredients
ADD COLUMN IF NOT EXISTS protein_per_100g REAL DEFAULT 0,
ADD COLUMN IF NOT EXISTS fat_per_100g REAL DEFAULT 0,
ADD COLUMN IF NOT EXISTS carbs_per_100g REAL DEFAULT 0;

-- ── Backfill: real USDA / verified values for common ingredients ──────────
-- Only ingredients that appear in Sous-Chef templates are backfilled here.
-- Others keep 0 until admin fills them via dashboard.

-- Dairy & Eggs
UPDATE catalog_ingredients SET protein_per_100g=3.2,  fat_per_100g=1.0,  carbs_per_100g=5.0   WHERE slug='pasteurized-milk';
UPDATE catalog_ingredients SET protein_per_100g=2.1,  fat_per_100g=18.0, carbs_per_100g=3.7   WHERE slug='cream-18';
UPDATE catalog_ingredients SET protein_per_100g=0.9,  fat_per_100g=81.0, carbs_per_100g=0.1   WHERE slug='butter';
UPDATE catalog_ingredients SET protein_per_100g=12.6, fat_per_100g=10.6, carbs_per_100g=0.7   WHERE slug='chicken-eggs';
UPDATE catalog_ingredients SET protein_per_100g=16.5, fat_per_100g=1.0,  carbs_per_100g=3.3   WHERE slug='cottage-cheese';
UPDATE catalog_ingredients SET protein_per_100g=10.0, fat_per_100g=4.0,  carbs_per_100g=3.6   WHERE slug='plain-yogurt';

-- Meat & Poultry
UPDATE catalog_ingredients SET protein_per_100g=31.0, fat_per_100g=3.6,  carbs_per_100g=0.0  WHERE slug='chicken-breast';
UPDATE catalog_ingredients SET protein_per_100g=26.0, fat_per_100g=13.0, carbs_per_100g=0.0  WHERE slug='chicken-thighs';
UPDATE catalog_ingredients SET protein_per_100g=26.0, fat_per_100g=15.0, carbs_per_100g=0.0  WHERE slug='beef';
UPDATE catalog_ingredients SET protein_per_100g=27.0, fat_per_100g=14.0, carbs_per_100g=0.0  WHERE slug='pork';
UPDATE catalog_ingredients SET protein_per_100g=17.0, fat_per_100g=20.0, carbs_per_100g=0.0  WHERE slug='ground-meat';
UPDATE catalog_ingredients SET protein_per_100g=29.0, fat_per_100g=3.5,  carbs_per_100g=0.0  WHERE slug='turkey-breast' OR slug='filet-z-indyka';

-- Fish & Seafood
UPDATE catalog_ingredients SET protein_per_100g=20.0, fat_per_100g=13.0, carbs_per_100g=0.0  WHERE slug='salmon';
UPDATE catalog_ingredients SET protein_per_100g=17.5, fat_per_100g=0.7,  carbs_per_100g=0.0  WHERE slug='cod';
UPDATE catalog_ingredients SET protein_per_100g=24.0, fat_per_100g=0.5,  carbs_per_100g=0.0  WHERE slug='shrimp' OR slug='shrimps';
UPDATE catalog_ingredients SET protein_per_100g=26.0, fat_per_100g=1.0,  carbs_per_100g=0.0  WHERE slug='canned-tuna';
UPDATE catalog_ingredients SET protein_per_100g=17.6, fat_per_100g=11.5, carbs_per_100g=0.0  WHERE slug='mackerel';
UPDATE catalog_ingredients SET protein_per_100g=17.5, fat_per_100g=9.0,  carbs_per_100g=0.0  WHERE slug='herring';
UPDATE catalog_ingredients SET protein_per_100g=20.0, fat_per_100g=6.0,  carbs_per_100g=0.0  WHERE slug='trout';
UPDATE catalog_ingredients SET protein_per_100g=18.0, fat_per_100g=5.0,  carbs_per_100g=0.0  WHERE slug='carp';
UPDATE catalog_ingredients SET protein_per_100g=19.0, fat_per_100g=3.0,  carbs_per_100g=0.0  WHERE slug='sea-bass';

-- Vegetables
UPDATE catalog_ingredients SET protein_per_100g=0.9,  fat_per_100g=0.2,  carbs_per_100g=3.9  WHERE slug='tomato';
UPDATE catalog_ingredients SET protein_per_100g=2.8,  fat_per_100g=0.4,  carbs_per_100g=7.0  WHERE slug='broccoli';
UPDATE catalog_ingredients SET protein_per_100g=2.9,  fat_per_100g=0.4,  carbs_per_100g=3.6  WHERE slug='spinach';
UPDATE catalog_ingredients SET protein_per_100g=2.0,  fat_per_100g=0.1,  carbs_per_100g=17.0 WHERE slug='potato' OR slug='potatoes';
UPDATE catalog_ingredients SET protein_per_100g=1.6,  fat_per_100g=0.1,  carbs_per_100g=20.0 WHERE slug='sweet-potato';
UPDATE catalog_ingredients SET protein_per_100g=0.7,  fat_per_100g=0.2,  carbs_per_100g=2.6  WHERE slug='cucumber';
UPDATE catalog_ingredients SET protein_per_100g=0.9,  fat_per_100g=0.3,  carbs_per_100g=4.6  WHERE slug='bell-pepper';
UPDATE catalog_ingredients SET protein_per_100g=1.1,  fat_per_100g=0.1,  carbs_per_100g=9.6  WHERE slug='onion';
UPDATE catalog_ingredients SET protein_per_100g=6.4,  fat_per_100g=0.5,  carbs_per_100g=33.0 WHERE slug='garlic';
UPDATE catalog_ingredients SET protein_per_100g=0.9,  fat_per_100g=0.2,  carbs_per_100g=3.6  WHERE slug='cherry-tomatoes';

-- Grains & Carbs
UPDATE catalog_ingredients SET protein_per_100g=13.0, fat_per_100g=6.5,  carbs_per_100g=56.0 WHERE slug='oatmeal' OR slug='oat-flakes';
UPDATE catalog_ingredients SET protein_per_100g=7.0,  fat_per_100g=2.7,  carbs_per_100g=77.0 WHERE slug='brown-rice';
UPDATE catalog_ingredients SET protein_per_100g=7.0,  fat_per_100g=0.9,  carbs_per_100g=74.0 WHERE slug='white-rice' OR slug='rice';
UPDATE catalog_ingredients SET protein_per_100g=13.0, fat_per_100g=2.5,  carbs_per_100g=71.0 WHERE slug='pasta' OR slug='spaghetti';
UPDATE catalog_ingredients SET protein_per_100g=14.0, fat_per_100g=6.0,  carbs_per_100g=64.0 WHERE slug='quinoa';
UPDATE catalog_ingredients SET protein_per_100g=13.0, fat_per_100g=3.4,  carbs_per_100g=68.0 WHERE slug='buckwheat';
UPDATE catalog_ingredients SET protein_per_100g=7.0,  fat_per_100g=1.0,  carbs_per_100g=49.0 WHERE slug='rye-bread';

-- Fruits
UPDATE catalog_ingredients SET protein_per_100g=1.1,  fat_per_100g=0.3,  carbs_per_100g=23.0 WHERE slug='banana';
UPDATE catalog_ingredients SET protein_per_100g=2.0,  fat_per_100g=15.0, carbs_per_100g=9.0  WHERE slug='avocado';

-- Nuts & Seeds
UPDATE catalog_ingredients SET protein_per_100g=15.0, fat_per_100g=65.0, carbs_per_100g=14.0 WHERE slug='walnuts';
UPDATE catalog_ingredients SET protein_per_100g=25.0, fat_per_100g=50.0, carbs_per_100g=16.0 WHERE slug='peanut-butter';

-- Fats & Oils
UPDATE catalog_ingredients SET protein_per_100g=0.0,  fat_per_100g=100.0,carbs_per_100g=0.0  WHERE slug='olive-oil';
UPDATE catalog_ingredients SET protein_per_100g=0.0,  fat_per_100g=100.0,carbs_per_100g=0.0  WHERE slug='sunflower-oil';

-- Other common
UPDATE catalog_ingredients SET protein_per_100g=0.3,  fat_per_100g=0.0,  carbs_per_100g=82.0 WHERE slug='honey';
UPDATE catalog_ingredients SET protein_per_100g=2.4,  fat_per_100g=15.0, carbs_per_100g=4.0  WHERE slug='sour-cream-15' OR slug='sour-cream';
