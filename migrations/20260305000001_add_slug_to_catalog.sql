-- Add slug column to catalog_ingredients for SEO-friendly public URLs
-- Date: 2026-03-05

ALTER TABLE catalog_ingredients
ADD COLUMN IF NOT EXISTS slug TEXT;

-- Create unique index (only for non-null, active slugs)
CREATE UNIQUE INDEX IF NOT EXISTS idx_catalog_ingredients_slug
ON catalog_ingredients (slug)
WHERE slug IS NOT NULL AND is_active = true;

-- Auto-generate slugs for existing ingredients from name_en
-- "Pasteurized milk" -> "pasteurized-milk"
-- "Soy sauce"        -> "soy-sauce"
UPDATE catalog_ingredients
SET slug = LOWER(
    REGEXP_REPLACE(
        REGEXP_REPLACE(TRIM(name_en), '[^a-zA-Z0-9\s-]', '', 'g'),
        '\s+', '-', 'g'
    )
)
WHERE slug IS NULL AND is_active = true;

-- For inactive entries, also generate slug (may be null if name_en is weird)
UPDATE catalog_ingredients
SET slug = LOWER(
    REGEXP_REPLACE(
        REGEXP_REPLACE(TRIM(name_en), '[^a-zA-Z0-9\s-]', '', 'g'),
        '\s+', '-', 'g'
    )
)
WHERE slug IS NULL AND (is_active = false OR is_active IS NULL);

-- Trigger: auto-generate slug on INSERT if not provided
CREATE OR REPLACE FUNCTION generate_ingredient_slug()
RETURNS TRIGGER AS $$
BEGIN
    IF NEW.slug IS NULL OR NEW.slug = '' THEN
        NEW.slug := LOWER(
            REGEXP_REPLACE(
                REGEXP_REPLACE(TRIM(NEW.name_en), '[^a-zA-Z0-9\s-]', '', 'g'),
                '\s+', '-', 'g'
            )
        );
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_catalog_ingredients_slug
BEFORE INSERT ON catalog_ingredients
FOR EACH ROW
EXECUTE FUNCTION generate_ingredient_slug();

COMMENT ON COLUMN catalog_ingredients.slug IS 'SEO-friendly URL slug, e.g. salmon, soy-sauce';
