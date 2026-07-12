-- Single physical-church record per site: address/contacts entered once,
-- translatable copy (title/description/schedule/dedication/shrines/priest) kept per language in `translations`.

CREATE TABLE IF NOT EXISTS church_info (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    site_id UUID NOT NULL REFERENCES sites(id),
    address TEXT NOT NULL DEFAULT '',
    maps_url TEXT NOT NULL DEFAULT '',
    phone_or_site TEXT NOT NULL DEFAULT '',
    priest_phone TEXT NOT NULL DEFAULT '',
    image_url TEXT NOT NULL DEFAULT '',
    translations JSONB NOT NULL DEFAULT '{}'::jsonb,
    status TEXT NOT NULL DEFAULT 'draft',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT church_info_status_check CHECK (status IN ('draft', 'published', 'archived')),
    CONSTRAINT church_info_site_unique UNIQUE (site_id)
);

CREATE INDEX IF NOT EXISTS idx_church_info_site ON church_info(site_id);

DROP TRIGGER IF EXISTS church_info_set_updated_at ON church_info;
CREATE TRIGGER church_info_set_updated_at
    BEFORE UPDATE ON church_info
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();
