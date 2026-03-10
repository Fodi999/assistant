-- Slug aliases for 301 redirects (SEO best practice)
-- When product name_en changes, old slug is saved here so old URLs still work.
-- Date: 2026-03-10

-- ── 1. Create slug_aliases table ──────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS slug_aliases (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    ingredient_id   UUID NOT NULL REFERENCES catalog_ingredients(id) ON DELETE CASCADE,
    old_slug        TEXT NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Unique constraint: each old_slug can only point to one ingredient
CREATE UNIQUE INDEX IF NOT EXISTS idx_slug_aliases_old_slug
    ON slug_aliases (old_slug);

-- Fast lookup by ingredient
CREATE INDEX IF NOT EXISTS idx_slug_aliases_ingredient_id
    ON slug_aliases (ingredient_id);

-- ── 2. Update DB trigger to regenerate slug on name_en UPDATE ─────────────────
-- Drop old INSERT-only trigger
DROP TRIGGER IF EXISTS trg_catalog_ingredients_slug ON catalog_ingredients;

-- New trigger function: handles both INSERT and UPDATE
CREATE OR REPLACE FUNCTION generate_ingredient_slug()
RETURNS TRIGGER AS $$
DECLARE
    new_slug TEXT;
BEGIN
    -- On INSERT: generate slug if not provided
    IF TG_OP = 'INSERT' THEN
        IF NEW.slug IS NULL OR NEW.slug = '' THEN
            NEW.slug := LOWER(
                REGEXP_REPLACE(
                    REGEXP_REPLACE(TRIM(NEW.name_en), '[^a-zA-Z0-9\s-]', '', 'g'),
                    '\s+', '-', 'g'
                )
            );
        END IF;
    END IF;

    -- On UPDATE: if name_en changed, regenerate slug and save old as alias
    IF TG_OP = 'UPDATE' THEN
        IF NEW.name_en IS DISTINCT FROM OLD.name_en THEN
            new_slug := LOWER(
                REGEXP_REPLACE(
                    REGEXP_REPLACE(TRIM(NEW.name_en), '[^a-zA-Z0-9\s-]', '', 'g'),
                    '\s+', '-', 'g'
                )
            );

            -- Only change slug if it's actually different
            IF new_slug IS DISTINCT FROM OLD.slug THEN
                -- Save old slug as alias (ignore if already exists)
                INSERT INTO slug_aliases (ingredient_id, old_slug)
                VALUES (OLD.id, OLD.slug)
                ON CONFLICT (old_slug) DO NOTHING;

                -- Update to new slug
                NEW.slug := new_slug;
            END IF;
        END IF;
    END IF;

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Trigger fires on both INSERT and UPDATE
CREATE TRIGGER trg_catalog_ingredients_slug
BEFORE INSERT OR UPDATE ON catalog_ingredients
FOR EACH ROW
EXECUTE FUNCTION generate_ingredient_slug();

-- ── 3. Seed alias for the current pasteurized-milk → milk case ────────────────
-- After this migration, the trigger will handle future changes automatically.
-- But we need to fix the existing "pasteurized-milk" product manually:

-- First, save the old slug as alias
INSERT INTO slug_aliases (ingredient_id, old_slug)
SELECT id, slug
FROM catalog_ingredients
WHERE slug = 'pasteurized-milk' AND is_active = true
ON CONFLICT (old_slug) DO NOTHING;

-- Then update the slug to match current name_en
UPDATE catalog_ingredients
SET slug = LOWER(
    REGEXP_REPLACE(
        REGEXP_REPLACE(TRIM(name_en), '[^a-zA-Z0-9\s-]', '', 'g'),
        '\s+', '-', 'g'
    )
)
WHERE slug = 'pasteurized-milk' AND is_active = true;

COMMENT ON TABLE slug_aliases IS 'Old slugs that should 301-redirect to the current slug. Created automatically when name_en changes.';
