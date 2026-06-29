-- Stage 12: normalized church calendar content model.
-- Calendar day is the main node; icons, prayers and articles are attached to it.

CREATE TABLE IF NOT EXISTS church_calendar_days (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    site_id UUID NOT NULL REFERENCES sites(id),
    date_old_style DATE,
    date_new_style DATE,
    calendar_type TEXT NOT NULL DEFAULT 'both',
    title TEXT NOT NULL,
    day_type TEXT NOT NULL DEFAULT 'saint',
    description TEXT NOT NULL DEFAULT '',
    rank INTEGER NOT NULL DEFAULT 0,
    status TEXT NOT NULL DEFAULT 'draft',
    is_global BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT church_calendar_days_calendar_type_check
        CHECK (calendar_type IN ('old_style', 'new_style', 'both')),
    CONSTRAINT church_calendar_days_day_type_check
        CHECK (day_type IN ('saint', 'feast', 'fasting', 'memorial', 'gospel', 'quiet')),
    CONSTRAINT church_calendar_days_status_check
        CHECK (status IN ('draft', 'published', 'archived')),
    CONSTRAINT church_calendar_days_has_date_check
        CHECK (date_old_style IS NOT NULL OR date_new_style IS NOT NULL)
);

CREATE INDEX IF NOT EXISTS idx_church_calendar_days_site_new_date
    ON church_calendar_days(site_id, date_new_style, rank DESC);
CREATE INDEX IF NOT EXISTS idx_church_calendar_days_site_old_date
    ON church_calendar_days(site_id, date_old_style, rank DESC);
CREATE INDEX IF NOT EXISTS idx_church_calendar_days_global_new_date
    ON church_calendar_days(is_global, date_new_style)
    WHERE is_global = true;

CREATE TABLE IF NOT EXISTS church_icons (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    site_id UUID NOT NULL REFERENCES sites(id),
    calendar_day_id UUID REFERENCES church_calendar_days(id) ON DELETE SET NULL,
    title TEXT NOT NULL,
    slug TEXT NOT NULL,
    image_url TEXT NOT NULL DEFAULT '',
    saint_name TEXT NOT NULL DEFAULT '',
    feast_name TEXT NOT NULL DEFAULT '',
    description TEXT NOT NULL DEFAULT '',
    language TEXT NOT NULL DEFAULT 'uk',
    status TEXT NOT NULL DEFAULT 'draft',
    is_global BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT church_icons_language_check CHECK (language IN ('uk', 'ru', 'en')),
    CONSTRAINT church_icons_status_check CHECK (status IN ('draft', 'published', 'archived')),
    CONSTRAINT church_icons_site_slug_language_unique UNIQUE (site_id, slug, language)
);

CREATE INDEX IF NOT EXISTS idx_church_icons_site_calendar_day
    ON church_icons(site_id, calendar_day_id, updated_at DESC);
CREATE INDEX IF NOT EXISTS idx_church_icons_site_status
    ON church_icons(site_id, status, updated_at DESC);

CREATE TABLE IF NOT EXISTS church_prayers (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    site_id UUID NOT NULL REFERENCES sites(id),
    icon_id UUID REFERENCES church_icons(id) ON DELETE SET NULL,
    calendar_day_id UUID REFERENCES church_calendar_days(id) ON DELETE SET NULL,
    title TEXT NOT NULL,
    text TEXT NOT NULL DEFAULT '',
    language TEXT NOT NULL DEFAULT 'uk',
    prayer_type TEXT NOT NULL DEFAULT 'prayer',
    status TEXT NOT NULL DEFAULT 'draft',
    is_global BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT church_prayers_language_check CHECK (language IN ('uk', 'ru', 'en')),
    CONSTRAINT church_prayers_type_check CHECK (prayer_type IN ('prayer', 'akathist', 'troparion', 'kontakion')),
    CONSTRAINT church_prayers_status_check CHECK (status IN ('draft', 'published', 'archived'))
);

CREATE INDEX IF NOT EXISTS idx_church_prayers_site_calendar_day
    ON church_prayers(site_id, calendar_day_id, prayer_type, updated_at DESC);
CREATE INDEX IF NOT EXISTS idx_church_prayers_site_icon
    ON church_prayers(site_id, icon_id, prayer_type, updated_at DESC);

CREATE TABLE IF NOT EXISTS church_articles (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    site_id UUID NOT NULL REFERENCES sites(id),
    icon_id UUID REFERENCES church_icons(id) ON DELETE SET NULL,
    calendar_day_id UUID REFERENCES church_calendar_days(id) ON DELETE SET NULL,
    title TEXT NOT NULL,
    slug TEXT NOT NULL,
    content TEXT NOT NULL DEFAULT '',
    language TEXT NOT NULL DEFAULT 'uk',
    seo_title TEXT NOT NULL DEFAULT '',
    seo_description TEXT NOT NULL DEFAULT '',
    status TEXT NOT NULL DEFAULT 'draft',
    is_global BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT church_articles_language_check CHECK (language IN ('uk', 'ru', 'en')),
    CONSTRAINT church_articles_status_check CHECK (status IN ('draft', 'published', 'archived')),
    CONSTRAINT church_articles_site_slug_language_unique UNIQUE (site_id, slug, language)
);

CREATE INDEX IF NOT EXISTS idx_church_articles_site_calendar_day
    ON church_articles(site_id, calendar_day_id, updated_at DESC);
CREATE INDEX IF NOT EXISTS idx_church_articles_site_icon
    ON church_articles(site_id, icon_id, updated_at DESC);

DROP TRIGGER IF EXISTS church_calendar_days_set_updated_at ON church_calendar_days;
CREATE TRIGGER church_calendar_days_set_updated_at
    BEFORE UPDATE ON church_calendar_days
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();

DROP TRIGGER IF EXISTS church_icons_set_updated_at ON church_icons;
CREATE TRIGGER church_icons_set_updated_at
    BEFORE UPDATE ON church_icons
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();

DROP TRIGGER IF EXISTS church_prayers_set_updated_at ON church_prayers;
CREATE TRIGGER church_prayers_set_updated_at
    BEFORE UPDATE ON church_prayers
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();

DROP TRIGGER IF EXISTS church_articles_set_updated_at ON church_articles;
CREATE TRIGGER church_articles_set_updated_at
    BEFORE UPDATE ON church_articles
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();
