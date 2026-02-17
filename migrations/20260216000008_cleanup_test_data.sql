-- Migration: Cleanup test data from global catalog (Soft Delete)
-- Instead of hard-deleting, mark as inactive to preserve data integrity
-- but hide from the application.

-- 1. Mark "Expiring Fish" and products with hex suffixes as inactive
UPDATE catalog_ingredients 
SET is_active = false
WHERE name_en LIKE 'Expiring Fish%'
   OR name_en ~ '[a-f0-9]{8}$';

-- 2. Mark any other test products as inactive
UPDATE catalog_ingredients
SET is_active = false
WHERE name_en LIKE 'Test %'
   OR name_en LIKE 'Dummy %';
