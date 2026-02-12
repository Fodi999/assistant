-- Add price column to catalog_ingredients for admin product management
ALTER TABLE catalog_ingredients
ADD COLUMN IF NOT EXISTS price DOUBLE PRECISION DEFAULT 0.0;

-- Add comment
COMMENT ON COLUMN catalog_ingredients.price IS 'Price per unit in currency (e.g., price per kg, per liter, per piece)';
