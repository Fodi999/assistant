-- Dedicated category catalog for icon order add-on options (icon_order_options),
-- replacing the free-text `category` column with a proper reference table.

CREATE TABLE IF NOT EXISTS icon_product_categories (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    site_id UUID NOT NULL REFERENCES sites(id),
    slug TEXT NOT NULL,
    name_uk TEXT NOT NULL DEFAULT '',
    name_ru TEXT NOT NULL DEFAULT '',
    name_en TEXT NOT NULL DEFAULT '',
    description_uk TEXT NOT NULL DEFAULT '',
    description_ru TEXT NOT NULL DEFAULT '',
    description_en TEXT NOT NULL DEFAULT '',
    image_url TEXT NOT NULL DEFAULT '',
    is_active BOOLEAN NOT NULL DEFAULT true,
    sort_order INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT icon_product_categories_site_slug_unique UNIQUE (site_id, slug)
);

CREATE INDEX IF NOT EXISTS idx_icon_product_categories_site_active
    ON icon_product_categories(site_id, is_active, sort_order);

DROP TRIGGER IF EXISTS icon_product_categories_set_updated_at ON icon_product_categories;
CREATE TRIGGER icon_product_categories_set_updated_at
    BEFORE UPDATE ON icon_product_categories
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();

-- Add the new FK column (nullable: options can be uncategorized).
ALTER TABLE icon_order_options
    ADD COLUMN IF NOT EXISTS category_id UUID REFERENCES icon_product_categories(id) ON DELETE SET NULL;

-- Backfill: for every distinct non-empty legacy `category` string still present,
-- create a matching category row (per site) and point existing options at it,
-- so no existing data is silently dropped.
DO $$
DECLARE
    rec RECORD;
    new_category_id UUID;
BEGIN
    IF EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_name = 'icon_order_options' AND column_name = 'category'
    ) THEN
        FOR rec IN
            SELECT DISTINCT site_id, category
            FROM icon_order_options
            WHERE category IS NOT NULL AND btrim(category) <> ''
        LOOP
            INSERT INTO icon_product_categories (site_id, slug, name_uk, name_ru, name_en, sort_order)
            VALUES (
                rec.site_id,
                btrim(regexp_replace(lower(btrim(rec.category)), '[^[:alnum:]]+', '-', 'g'), '-'),
                rec.category,
                rec.category,
                rec.category,
                0
            )
            ON CONFLICT (site_id, slug) DO UPDATE SET slug = EXCLUDED.slug
            RETURNING id INTO new_category_id;

            UPDATE icon_order_options
            SET category_id = new_category_id
            WHERE site_id = rec.site_id AND category = rec.category;
        END LOOP;

        ALTER TABLE icon_order_options DROP COLUMN category;
    END IF;
END $$;
