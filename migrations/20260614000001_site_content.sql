CREATE TABLE IF NOT EXISTS site_content (
    site TEXT PRIMARY KEY,
    content JSONB NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS site_leads (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    site TEXT NOT NULL,
    name TEXT NOT NULL DEFAULT '',
    phone TEXT NOT NULL,
    object_type TEXT NOT NULL DEFAULT '',
    area TEXT NOT NULL DEFAULT '',
    comment TEXT NOT NULL DEFAULT '',
    items TEXT[] NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_site_leads_site_created_at
    ON site_leads (site, created_at DESC);
