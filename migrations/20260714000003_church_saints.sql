-- Dedicated saints entity for the church content system, mirroring
-- church_icons/church_prayers: per-language rows linked by translation group.

CREATE TABLE IF NOT EXISTS church_saints (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    site_id UUID NOT NULL REFERENCES sites(id),
    icon_id UUID REFERENCES church_icons(id) ON DELETE SET NULL,
    calendar_day_id UUID REFERENCES church_calendar_days(id) ON DELETE SET NULL,
    slug TEXT NOT NULL,
    name TEXT NOT NULL,
    short_description TEXT NOT NULL DEFAULT '',
    biography TEXT NOT NULL DEFAULT '',
    feast_day TEXT NOT NULL DEFAULT '',
    image_url TEXT NOT NULL DEFAULT '',
    language TEXT NOT NULL DEFAULT 'uk',
    translation_group_id UUID NOT NULL DEFAULT gen_random_uuid(),
    status TEXT NOT NULL DEFAULT 'draft',
    is_global BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT church_saints_language_check CHECK (language IN ('uk', 'ru', 'en')),
    CONSTRAINT church_saints_status_check CHECK (status IN ('draft', 'published', 'archived')),
    CONSTRAINT church_saints_site_slug_language_unique UNIQUE (site_id, slug, language)
);

CREATE INDEX IF NOT EXISTS idx_church_saints_site_status
    ON church_saints(site_id, status, updated_at DESC);
CREATE INDEX IF NOT EXISTS idx_church_saints_translation_group
    ON church_saints(translation_group_id, language);
CREATE INDEX IF NOT EXISTS idx_church_saints_site_calendar_day
    ON church_saints(site_id, calendar_day_id, updated_at DESC);
