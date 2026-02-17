-- Migration: Cleanup garbage categories and ingredients correctly (Safe Version)
-- Targets: 'Test', 'Alert Cat', 'Test Cat'

-- 1. We CANNOT delete catalog_ingredients if they are referenced by inventory_batches.
-- Instead, move them to a safe category and mark them as inactive (Soft Delete).
UPDATE catalog_ingredients 
SET 
  is_active = false,
  category_id = (SELECT id FROM catalog_categories WHERE name_en = 'Dairy & Eggs' LIMIT 1)
WHERE category_id IN (
    SELECT id FROM catalog_categories 
    WHERE name_en ILIKE 'Test%' 
       OR name_en ILIKE 'Alert%'
       OR name_ru ILIKE 'Test%'
       OR name_ru ILIKE 'Alert%'
);

-- 2. Now no ingredients (active or inactive) point to the garbage categories.
-- We can safely delete the garbage categories.
DELETE FROM catalog_categories 
WHERE name_en ILIKE 'Test%' 
   OR name_en ILIKE 'Alert%'
   OR name_ru ILIKE 'Test%'
   OR name_ru ILIKE 'Alert%';

-- 3. Fix minor typo in real category
UPDATE catalog_categories 
SET name_ru = 'Молочные продукты и яйца' 
WHERE name_en = 'Dairy & Eggs' AND name_ru = 'Молочные продукты и яйця';
