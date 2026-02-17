-- Clean up test/garbage products from the global catalog
-- Leaves only real products (those that either have an image OR are from the standard initial list)

-- 1. Identify and delete products with "Expiring Fish" or hex suffixes in name
DELETE FROM catalog_ingredients 
WHERE name_en LIKE 'Expiring Fish%'
   OR name_en ~ '[a-f0-9]{8}$';

-- 2. Delete products that don't have an image AND are not part of the core set (optional but safer to do by name)
-- For now, let's just target the specific ones mentioned by the user.
DELETE FROM catalog_ingredients
WHERE name_en LIKE 'Test %'
   OR name_en LIKE 'Dummy %';

-- 3. Ensure we keep the "Frozen vegetables" and other real ones.
-- (This is just a comment, DELETE already excluded them)
