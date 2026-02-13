-- Fix tenant_ingredients unique constraint to allow re-adding after soft-delete
-- Migration: 20240120000001

-- Drop the old UNIQUE constraint
ALTER TABLE tenant_ingredients DROP CONSTRAINT IF EXISTS tenant_ingredient_unique;

-- Create partial UNIQUE index that only applies to active records
CREATE UNIQUE INDEX tenant_ingredient_unique 
ON tenant_ingredients(tenant_id, catalog_ingredient_id) 
WHERE is_active = true;

-- Comment
COMMENT ON INDEX tenant_ingredient_unique IS 'Ensures tenant can only have one active instance of each catalog ingredient. Allows re-adding after soft-delete.';
