-- Add description column to catalog_ingredients (was missing from previous migration)
ALTER TABLE catalog_ingredients
ADD COLUMN IF NOT EXISTS description TEXT;

COMMENT ON COLUMN catalog_ingredients.description IS 'Additional product description for admin management';
