-- Stage 14: publication slugs and import safety for church content.

ALTER TABLE church_prayers ADD COLUMN IF NOT EXISTS slug TEXT;

UPDATE church_prayers
SET slug = 'prayer-' || id::text
WHERE slug IS NULL OR btrim(slug) = '';

ALTER TABLE church_prayers ALTER COLUMN slug SET NOT NULL;

CREATE UNIQUE INDEX IF NOT EXISTS idx_church_prayers_site_slug_language
    ON church_prayers(site_id, slug, language);

CREATE INDEX IF NOT EXISTS idx_church_prayers_public_slug
    ON church_prayers(site_id, slug, status);
