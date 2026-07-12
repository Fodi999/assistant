-- Gospel reading, mirroring church_prayers/church_articles: one row per language,
-- attached to a calendar day and/or icon, with its own public slug.

CREATE TABLE IF NOT EXISTS church_gospel_readings (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    site_id UUID NOT NULL REFERENCES sites(id),
    icon_id UUID REFERENCES church_icons(id) ON DELETE SET NULL,
    calendar_day_id UUID REFERENCES church_calendar_days(id) ON DELETE SET NULL,
    slug TEXT NOT NULL,
    title TEXT NOT NULL,
    reference TEXT NOT NULL DEFAULT '',
    text TEXT NOT NULL DEFAULT '',
    explanation TEXT NOT NULL DEFAULT '',
    language TEXT NOT NULL DEFAULT 'uk',
    status TEXT NOT NULL DEFAULT 'draft',
    is_global BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT church_gospel_readings_language_check CHECK (language IN ('uk', 'ru', 'en')),
    CONSTRAINT church_gospel_readings_status_check CHECK (status IN ('draft', 'published', 'archived')),
    CONSTRAINT church_gospel_readings_site_slug_language_unique UNIQUE (site_id, slug, language)
);

CREATE INDEX IF NOT EXISTS idx_church_gospel_readings_site_calendar_day
    ON church_gospel_readings(site_id, calendar_day_id, updated_at DESC);
CREATE INDEX IF NOT EXISTS idx_church_gospel_readings_site_icon
    ON church_gospel_readings(site_id, icon_id, updated_at DESC);
CREATE INDEX IF NOT EXISTS idx_church_gospel_readings_public_slug
    ON church_gospel_readings(site_id, slug, status);

DROP TRIGGER IF EXISTS church_gospel_readings_set_updated_at ON church_gospel_readings;
CREATE TRIGGER church_gospel_readings_set_updated_at
    BEFORE UPDATE ON church_gospel_readings
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();
