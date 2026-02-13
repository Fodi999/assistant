-- Add soft delete and uniqueness to catalog_ingredients
-- Migration: 20240118000001

-- 1. Add is_active column for soft delete
ALTER TABLE catalog_ingredients 
ADD COLUMN IF NOT EXISTS is_active BOOLEAN DEFAULT true;

-- 2. Add unique constraint on lowercase name_en
-- First, create a unique index on LOWER(name_en) where is_active = true
CREATE UNIQUE INDEX IF NOT EXISTS idx_catalog_ingredients_name_en_unique 
ON catalog_ingredients (LOWER(name_en)) 
WHERE is_active = true;

-- 3. Update existing duplicate entries (keep only the first one, mark others as inactive)
WITH duplicates AS (
    SELECT 
        id,
        LOWER(name_en) as name_lower,
        ROW_NUMBER() OVER (PARTITION BY LOWER(name_en) ORDER BY created_at) as rn
    FROM catalog_ingredients
    WHERE is_active = true
)
UPDATE catalog_ingredients
SET is_active = false
WHERE id IN (
    SELECT id 
    FROM duplicates 
    WHERE rn > 1
);

-- 4. Add comment explaining the constraint
COMMENT ON COLUMN catalog_ingredients.is_active IS 'Soft delete flag - false means deleted';
COMMENT ON INDEX idx_catalog_ingredients_name_en_unique IS 'Ensures unique product names (case-insensitive) among active products';
