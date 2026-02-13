-- Add soft delete and uniqueness to catalog_ingredients
-- Migration: 20240118000001

-- 1. Add is_active column for soft delete
ALTER TABLE catalog_ingredients 
ADD COLUMN IF NOT EXISTS is_active BOOLEAN DEFAULT true;

-- 2. Update existing duplicate entries FIRST (before creating unique index)
-- Keep only the first one, mark others as inactive
WITH duplicates AS (
    SELECT 
        id,
        LOWER(name_en) as name_lower,
        ROW_NUMBER() OVER (PARTITION BY LOWER(name_en) ORDER BY created_at NULLS LAST, id) as rn
    FROM catalog_ingredients
    WHERE is_active = true OR is_active IS NULL
)
UPDATE catalog_ingredients
SET is_active = false
WHERE id IN (
    SELECT id 
    FROM duplicates 
    WHERE rn > 1
);

-- 3. NOW create unique index after deduplication
CREATE UNIQUE INDEX IF NOT EXISTS idx_catalog_ingredients_name_en_unique 
ON catalog_ingredients (LOWER(name_en)) 
WHERE is_active = true;

-- 4. Add comments
COMMENT ON COLUMN catalog_ingredients.is_active IS 'Soft delete flag - false means deleted';
COMMENT ON INDEX idx_catalog_ingredients_name_en_unique IS 'Ensures unique product names (case-insensitive) among active products';
