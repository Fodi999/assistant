-- Add is_published flag to catalog_ingredients
-- Products start as drafts (is_published = false)
-- Admin must explicitly publish after filling all required fields
-- Public API (blog) only shows published products

ALTER TABLE catalog_ingredients
ADD COLUMN IF NOT EXISTS is_published BOOLEAN NOT NULL DEFAULT false;

-- Index for public queries: only published + active products
CREATE INDEX IF NOT EXISTS idx_catalog_ingredients_published
ON catalog_ingredients (is_published)
WHERE is_active = true AND is_published = true;

-- Add published_at timestamp to track when product was published
ALTER TABLE catalog_ingredients
ADD COLUMN IF NOT EXISTS published_at TIMESTAMPTZ;
