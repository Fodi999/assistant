-- Add SEO fields to catalog_ingredients for per-product SEO management
ALTER TABLE catalog_ingredients
    ADD COLUMN IF NOT EXISTS seo_title       TEXT,
    ADD COLUMN IF NOT EXISTS seo_description TEXT,
    ADD COLUMN IF NOT EXISTS seo_h1          TEXT,
    ADD COLUMN IF NOT EXISTS canonical_url   TEXT,
    ADD COLUMN IF NOT EXISTS og_title        TEXT,
    ADD COLUMN IF NOT EXISTS og_description  TEXT,
    ADD COLUMN IF NOT EXISTS og_image        TEXT;

COMMENT ON COLUMN catalog_ingredients.seo_title       IS 'Custom SEO <title> tag for search engines';
COMMENT ON COLUMN catalog_ingredients.seo_description  IS 'Custom SEO <meta description> for search engines';
COMMENT ON COLUMN catalog_ingredients.seo_h1           IS 'Custom H1 heading for the ingredient page';
COMMENT ON COLUMN catalog_ingredients.canonical_url    IS 'Canonical URL override (usually auto-generated from slug)';
COMMENT ON COLUMN catalog_ingredients.og_title         IS 'Open Graph title for social sharing';
COMMENT ON COLUMN catalog_ingredients.og_description   IS 'Open Graph description for social sharing';
COMMENT ON COLUMN catalog_ingredients.og_image         IS 'Open Graph image URL for social sharing';
