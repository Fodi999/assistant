-- Fix: add full UNIQUE constraint on catalog_ingredients.slug
-- The existing partial index (WHERE slug IS NOT NULL AND is_active = true)
-- cannot be used by ON CONFLICT (slug) — PostgreSQL requires an unconditional
-- unique constraint for that syntax.

-- Drop the partial index first to avoid duplicate enforcement
DROP INDEX IF EXISTS idx_catalog_ingredients_slug;

-- Add a proper UNIQUE constraint (NULL values are always distinct in PostgreSQL,
-- so multiple rows with slug IS NULL are still allowed)
ALTER TABLE catalog_ingredients
ADD CONSTRAINT catalog_ingredients_slug_unique UNIQUE (slug);
