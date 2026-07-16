-- Turns icon_order_options (already the add-on options table) into a full
-- product catalog: optional link back to an icon translation group, richer
-- localized content, a photo gallery, stock status and SEO fields.
-- The physical table is intentionally NOT renamed to avoid breaking existing
-- FKs (icon_order_items.option_id) and in-flight orders.

ALTER TABLE icon_order_options
    ADD COLUMN IF NOT EXISTS linked_icon_translation_group_id UUID,
    ADD COLUMN IF NOT EXISTS full_description_uk TEXT NOT NULL DEFAULT '',
    ADD COLUMN IF NOT EXISTS full_description_ru TEXT NOT NULL DEFAULT '',
    ADD COLUMN IF NOT EXISTS full_description_en TEXT NOT NULL DEFAULT '',
    ADD COLUMN IF NOT EXISTS gallery_urls TEXT[] NOT NULL DEFAULT '{}',
    ADD COLUMN IF NOT EXISTS production_time TEXT NOT NULL DEFAULT '',
    ADD COLUMN IF NOT EXISTS consecration_available BOOLEAN NOT NULL DEFAULT false,
    ADD COLUMN IF NOT EXISTS stock_status TEXT NOT NULL DEFAULT 'available',
    ADD COLUMN IF NOT EXISTS featured BOOLEAN NOT NULL DEFAULT false,
    ADD COLUMN IF NOT EXISTS seo_title_uk TEXT NOT NULL DEFAULT '',
    ADD COLUMN IF NOT EXISTS seo_title_ru TEXT NOT NULL DEFAULT '',
    ADD COLUMN IF NOT EXISTS seo_title_en TEXT NOT NULL DEFAULT '',
    ADD COLUMN IF NOT EXISTS seo_description_uk TEXT NOT NULL DEFAULT '',
    ADD COLUMN IF NOT EXISTS seo_description_ru TEXT NOT NULL DEFAULT '',
    ADD COLUMN IF NOT EXISTS seo_description_en TEXT NOT NULL DEFAULT '';

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint WHERE conname = 'icon_order_options_stock_status_check'
    ) THEN
        ALTER TABLE icon_order_options
            ADD CONSTRAINT icon_order_options_stock_status_check
            CHECK (stock_status IN ('available', 'made_to_order', 'unavailable'));
    END IF;
END $$;

CREATE INDEX IF NOT EXISTS idx_icon_order_options_linked_icon_group
    ON icon_order_options(linked_icon_translation_group_id);

CREATE INDEX IF NOT EXISTS idx_icon_order_options_featured
    ON icon_order_options(site_id, featured) WHERE featured = true;

-- Generalize icon_orders to snapshot an arbitrary "primary product" instead
-- of always being about a directly-ordered icon. The existing icon_id /
-- icon_title_snapshot / icon_slug_snapshot columns are untouched and keep
-- working for orders created through the older icon-only endpoint.
ALTER TABLE icon_orders
    ADD COLUMN IF NOT EXISTS primary_product_id UUID REFERENCES icon_order_options(id) ON DELETE SET NULL,
    ADD COLUMN IF NOT EXISTS primary_product_name_snapshot TEXT NOT NULL DEFAULT '',
    ADD COLUMN IF NOT EXISTS primary_product_slug_snapshot TEXT NOT NULL DEFAULT '',
    ADD COLUMN IF NOT EXISTS primary_product_price_cents_snapshot BIGINT NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS primary_product_photo_snapshot TEXT NOT NULL DEFAULT '';
