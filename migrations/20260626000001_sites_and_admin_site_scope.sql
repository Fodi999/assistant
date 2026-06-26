-- CRM site isolation: stable site UUIDs and per-resource ownership.
-- Existing string aliases remain only as migration/backward-compatibility data.

CREATE TABLE IF NOT EXISTS sites (
    id UUID PRIMARY KEY,
    key TEXT NOT NULL UNIQUE,
    display_name TEXT NOT NULL,
    aliases TEXT[] NOT NULL DEFAULT '{}',
    is_global BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

INSERT INTO sites (id, key, display_name, aliases)
VALUES
    ('00000000-0000-0000-0000-000000000101', 'church', 'Church Icons', ARRAY['church', 'icons']),
    ('00000000-0000-0000-0000-000000000102', 'construction', 'AlmaBuild', ARRAY['construction', 'almabuild']),
    ('00000000-0000-0000-0000-000000000103', 'kitchen', 'Culinary', ARRAY['kitchen', 'culinary'])
ON CONFLICT (id) DO UPDATE
SET key = EXCLUDED.key,
    display_name = EXCLUDED.display_name,
    aliases = EXCLUDED.aliases,
    updated_at = NOW();

ALTER TABLE catalog_ingredients ADD COLUMN IF NOT EXISTS site_id UUID;
ALTER TABLE catalog_ingredients ADD COLUMN IF NOT EXISTS is_global BOOLEAN NOT NULL DEFAULT FALSE;
UPDATE catalog_ingredients
SET site_id = '00000000-0000-0000-0000-000000000103'
WHERE site_id IS NULL;
ALTER TABLE catalog_ingredients ALTER COLUMN site_id SET NOT NULL;
CREATE INDEX IF NOT EXISTS idx_catalog_ingredients_site_active
    ON catalog_ingredients(site_id, is_active, name_en);

ALTER TABLE knowledge_articles ADD COLUMN IF NOT EXISTS site_id UUID;
ALTER TABLE knowledge_articles ADD COLUMN IF NOT EXISTS is_global BOOLEAN NOT NULL DEFAULT FALSE;
UPDATE knowledge_articles
SET site_id = '00000000-0000-0000-0000-000000000103'
WHERE site_id IS NULL;
ALTER TABLE knowledge_articles ALTER COLUMN site_id SET NOT NULL;
CREATE INDEX IF NOT EXISTS idx_knowledge_articles_site_updated
    ON knowledge_articles(site_id, updated_at DESC);

ALTER TABLE shop_products ADD COLUMN IF NOT EXISTS site_id UUID;
ALTER TABLE shop_products ADD COLUMN IF NOT EXISTS is_global BOOLEAN NOT NULL DEFAULT FALSE;
UPDATE shop_products
SET site_id = '00000000-0000-0000-0000-000000000103'
WHERE site_id IS NULL;
ALTER TABLE shop_products ALTER COLUMN site_id SET NOT NULL;
CREATE INDEX IF NOT EXISTS idx_shop_products_site_updated
    ON shop_products(site_id, updated_at DESC);

ALTER TABLE site_leads ADD COLUMN IF NOT EXISTS site_id UUID;
ALTER TABLE site_leads ADD COLUMN IF NOT EXISTS is_global BOOLEAN NOT NULL DEFAULT FALSE;
UPDATE site_leads
SET site_id = CASE
    WHEN site IN ('construction', 'almabuild') THEN '00000000-0000-0000-0000-000000000102'::uuid
    WHEN site IN ('church', 'icons') THEN '00000000-0000-0000-0000-000000000101'::uuid
    ELSE '00000000-0000-0000-0000-000000000103'::uuid
END
WHERE site_id IS NULL;
UPDATE site_leads
SET site = CASE
    WHEN site IN ('construction', 'almabuild') THEN 'construction'
    WHEN site IN ('church', 'icons') THEN 'church'
    ELSE 'kitchen'
END;
ALTER TABLE site_leads ALTER COLUMN site_id SET NOT NULL;
CREATE INDEX IF NOT EXISTS idx_site_leads_site_created
    ON site_leads(site_id, created_at DESC);

ALTER TABLE admin_affiliate_products ADD COLUMN IF NOT EXISTS site_id UUID;
ALTER TABLE admin_affiliate_products ADD COLUMN IF NOT EXISTS is_global BOOLEAN NOT NULL DEFAULT FALSE;
UPDATE admin_affiliate_products
SET site_id = CASE
    WHEN site = 'construction' THEN '00000000-0000-0000-0000-000000000102'::uuid
    ELSE '00000000-0000-0000-0000-000000000103'::uuid
END
WHERE site_id IS NULL;
ALTER TABLE admin_affiliate_products DROP CONSTRAINT IF EXISTS admin_affiliate_products_site_check;
UPDATE admin_affiliate_products
SET site = CASE
    WHEN site = 'construction' THEN 'construction'
    ELSE 'kitchen'
END;
ALTER TABLE admin_affiliate_products
    ADD CONSTRAINT admin_affiliate_products_site_check
    CHECK (site IN ('church', 'construction', 'kitchen'));
ALTER TABLE admin_affiliate_products ALTER COLUMN site_id SET NOT NULL;
CREATE INDEX IF NOT EXISTS idx_admin_affiliate_products_site_id_updated
    ON admin_affiliate_products(site_id, updated_at DESC);

ALTER TABLE admin_suppliers ADD COLUMN IF NOT EXISTS site_id UUID;
ALTER TABLE admin_suppliers ADD COLUMN IF NOT EXISTS is_global BOOLEAN NOT NULL DEFAULT FALSE;
UPDATE admin_suppliers
SET site_id = CASE
    WHEN supplier_type IN ('marketplace', 'affiliate_merchant') THEN '00000000-0000-0000-0000-000000000103'::uuid
    ELSE '00000000-0000-0000-0000-000000000102'::uuid
END
WHERE site_id IS NULL;
ALTER TABLE admin_suppliers ALTER COLUMN site_id SET NOT NULL;
CREATE INDEX IF NOT EXISTS idx_admin_suppliers_site_updated
    ON admin_suppliers(site_id, updated_at DESC);

ALTER TABLE seo_settings ADD COLUMN IF NOT EXISTS site_id UUID;
ALTER TABLE seo_settings ADD COLUMN IF NOT EXISTS is_global BOOLEAN NOT NULL DEFAULT TRUE;
UPDATE seo_settings
SET site_id = '00000000-0000-0000-0000-000000000103'
WHERE site_id IS NULL;
ALTER TABLE seo_settings ALTER COLUMN site_id SET NOT NULL;
CREATE INDEX IF NOT EXISTS idx_seo_settings_site_key
    ON seo_settings(site_id, key);

ALTER TABLE ai_usage_stats ADD COLUMN IF NOT EXISTS site_id UUID;
ALTER TABLE ai_usage_stats ADD COLUMN IF NOT EXISTS is_global BOOLEAN NOT NULL DEFAULT TRUE;
UPDATE ai_usage_stats
SET site_id = '00000000-0000-0000-0000-000000000103'
WHERE site_id IS NULL;
ALTER TABLE ai_usage_stats ALTER COLUMN site_id SET NOT NULL;
CREATE INDEX IF NOT EXISTS idx_ai_usage_stats_site_created
    ON ai_usage_stats(site_id, created_at DESC);

DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'catalog_ingredients_site_id_fkey') THEN
        ALTER TABLE catalog_ingredients ADD CONSTRAINT catalog_ingredients_site_id_fkey FOREIGN KEY (site_id) REFERENCES sites(id);
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'knowledge_articles_site_id_fkey') THEN
        ALTER TABLE knowledge_articles ADD CONSTRAINT knowledge_articles_site_id_fkey FOREIGN KEY (site_id) REFERENCES sites(id);
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'shop_products_site_id_fkey') THEN
        ALTER TABLE shop_products ADD CONSTRAINT shop_products_site_id_fkey FOREIGN KEY (site_id) REFERENCES sites(id);
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'site_leads_site_id_fkey') THEN
        ALTER TABLE site_leads ADD CONSTRAINT site_leads_site_id_fkey FOREIGN KEY (site_id) REFERENCES sites(id);
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'admin_affiliate_products_site_id_fkey') THEN
        ALTER TABLE admin_affiliate_products ADD CONSTRAINT admin_affiliate_products_site_id_fkey FOREIGN KEY (site_id) REFERENCES sites(id);
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'admin_suppliers_site_id_fkey') THEN
        ALTER TABLE admin_suppliers ADD CONSTRAINT admin_suppliers_site_id_fkey FOREIGN KEY (site_id) REFERENCES sites(id);
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'seo_settings_site_id_fkey') THEN
        ALTER TABLE seo_settings ADD CONSTRAINT seo_settings_site_id_fkey FOREIGN KEY (site_id) REFERENCES sites(id);
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ai_usage_stats_site_id_fkey') THEN
        ALTER TABLE ai_usage_stats ADD CONSTRAINT ai_usage_stats_site_id_fkey FOREIGN KEY (site_id) REFERENCES sites(id);
    END IF;
END $$;
