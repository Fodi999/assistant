-- Migration: Cleanup garbage categories from global catalog
-- Targets: 'Test', 'Alert Cat', 'Test Cat'

-- 1. Delete catalog ingredients that reference garbage categories
-- We already soft-deleted garbage ingredients, but here we can hard-delete 
-- those that specifically link to trash categories to satisfy foreign keys.
DELETE FROM catalog_ingredients 
WHERE category_id IN (
    SELECT id FROM catalog_categories 
    WHERE name_en ILIKE 'Test%' 
       OR name_en ILIKE 'Alert%'
       OR name_ru ILIKE 'Test%'
       OR name_ru ILIKE 'Alert%'
);

-- 2. Delete the garbage categories
DELETE FROM catalog_categories 
WHERE name_en ILIKE 'Test%' 
   OR name_en ILIKE 'Alert%'
   OR name_ru ILIKE 'Test%'
   OR name_ru ILIKE 'Alert%';

-- 3. Optional: Fix any typos in real categories if seen
UPDATE catalog_categories 
SET name_ru = 'Молочные продукты и яйца' 
WHERE name_en = 'Dairy & Eggs' AND name_ru = 'Молочные продукты и яйця';
