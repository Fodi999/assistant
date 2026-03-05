-- Add density column for volume↔mass conversion
ALTER TABLE catalog_ingredients
ADD COLUMN IF NOT EXISTS density_g_per_ml NUMERIC;

-- Populate common ingredients
UPDATE catalog_ingredients SET density_g_per_ml = 1.0   WHERE LOWER(name_en) = 'water';
UPDATE catalog_ingredients SET density_g_per_ml = 1.03  WHERE LOWER(name_en) = 'milk';
UPDATE catalog_ingredients SET density_g_per_ml = 0.91  WHERE LOWER(name_en) = 'olive oil';
UPDATE catalog_ingredients SET density_g_per_ml = 0.53  WHERE LOWER(name_en) = 'flour';
UPDATE catalog_ingredients SET density_g_per_ml = 0.85  WHERE LOWER(name_en) = 'sugar';
UPDATE catalog_ingredients SET density_g_per_ml = 0.92  WHERE LOWER(name_en) = 'butter';
UPDATE catalog_ingredients SET density_g_per_ml = 0.88  WHERE LOWER(name_en) = 'honey';
UPDATE catalog_ingredients SET density_g_per_ml = 1.03  WHERE LOWER(name_en) = 'cream';
UPDATE catalog_ingredients SET density_g_per_ml = 1.33  WHERE LOWER(name_en) = 'tomato paste';
UPDATE catalog_ingredients SET density_g_per_ml = 1.04  WHERE LOWER(name_en) = 'soy sauce';
UPDATE catalog_ingredients SET density_g_per_ml = 0.77  WHERE LOWER(name_en) = 'rice';
UPDATE catalog_ingredients SET density_g_per_ml = 0.95  WHERE LOWER(name_en) = 'egg';
UPDATE catalog_ingredients SET density_g_per_ml = 0.56  WHERE LOWER(name_en) = 'oats';
UPDATE catalog_ingredients SET density_g_per_ml = 0.72  WHERE LOWER(name_en) = 'cocoa powder';
UPDATE catalog_ingredients SET density_g_per_ml = 1.26  WHERE LOWER(name_en) = 'salt';
