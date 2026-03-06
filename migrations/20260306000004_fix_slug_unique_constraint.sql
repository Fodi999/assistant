-- Fix: add full UNIQUE constraint on catalog_ingredients.slug
-- The existing partial index (WHERE slug IS NOT NULL AND is_active = true)
-- cannot be used by ON CONFLICT (slug) — PostgreSQL requires an unconditional
-- unique constraint for that syntax.

-- Drop the partial index first to avoid duplicate enforcement
DROP INDEX IF EXISTS idx_catalog_ingredients_slug;

-- ── Deduplicate slugs before adding UNIQUE constraint ────────────────────────
-- For each duplicated slug: keep the best row (is_active DESC, id ASC),
-- soft-delete all other duplicates by setting slug = NULL and is_active = false.
UPDATE catalog_ingredients
SET slug = NULL, is_active = false
WHERE id IN (
    SELECT id FROM (
        SELECT id,
               ROW_NUMBER() OVER (
                   PARTITION BY slug
                   ORDER BY is_active DESC, id ASC
               ) AS rn
        FROM catalog_ingredients
        WHERE slug IS NOT NULL
    ) ranked
    WHERE rn > 1
);

-- Add a proper UNIQUE constraint (NULL values are always distinct in PostgreSQL,
-- so multiple rows with slug IS NULL are still allowed)
ALTER TABLE catalog_ingredients
ADD CONSTRAINT catalog_ingredients_slug_unique UNIQUE (slug);
