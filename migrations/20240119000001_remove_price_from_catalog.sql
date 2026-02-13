-- Remove price from master catalog (it's tenant-specific, not global)
-- Migration: 20240119000001

-- 🎯 Step 1: Mark duplicate "Onions" as inactive (keep only first one)
WITH onions_duplicates AS (
    SELECT 
        id,
        ROW_NUMBER() OVER (
            PARTITION BY LOWER(name_en) 
            ORDER BY created_at NULLS LAST, id
        ) as rn
    FROM catalog_ingredients
    WHERE LOWER(name_en) = 'onions' 
      AND COALESCE(is_active, true) = true
)
UPDATE catalog_ingredients
SET is_active = false
WHERE id IN (
    SELECT id FROM onions_duplicates WHERE rn > 1
);

-- 🎯 Step 2: Remove price from catalog_ingredients
-- Price is tenant-specific, not master data
ALTER TABLE catalog_ingredients DROP COLUMN IF EXISTS price;

-- 🎯 Step 3: Also remove description from master catalog (optional - can keep for reference)
-- COMMENT: Description can stay in master catalog as reference info

COMMENT ON TABLE catalog_ingredients IS 'Master catalog of ingredients - managed by admin, shared across all tenants. Prices are tenant-specific.';
