-- Per-site Google Analytics connection configuration.
-- Tokens are stored per site so one site's reconnect cannot affect another site.

CREATE TABLE IF NOT EXISTS site_analytics_connections (
    site_id UUID PRIMARY KEY REFERENCES sites(id) ON DELETE CASCADE,
    google_property_id TEXT,
    refresh_token TEXT,
    connection_id TEXT,
    connected_at TIMESTAMPTZ,
    status TEXT NOT NULL DEFAULT 'not_connected',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT site_analytics_connections_status_check
        CHECK (status IN ('not_connected', 'connected', 'expired', 'error'))
);

CREATE INDEX IF NOT EXISTS idx_site_analytics_connections_status
    ON site_analytics_connections(status, updated_at DESC);

INSERT INTO site_analytics_connections (site_id, status)
SELECT id, 'not_connected'
FROM sites
ON CONFLICT (site_id) DO NOTHING;
