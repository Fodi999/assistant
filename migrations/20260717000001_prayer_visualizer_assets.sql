-- Precomputed particle maps for the prayer-mode WebGL visualizer.
--
-- Previously the browser downloaded the raw source photo and rebuilt the
-- entire particle field (luminance/edge sampling over tens of thousands of
-- candidate pixels) on every single page load. This table lets the backend
-- do that work exactly once per source image and hand the browser a
-- ready-to-use binary buffer instead.
CREATE TABLE IF NOT EXISTS church_prayer_visualizer_assets (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    prayer_id UUID NOT NULL REFERENCES church_prayers(id) ON DELETE CASCADE,
    source_image_url TEXT NOT NULL DEFAULT '',
    desktop_map_url TEXT NOT NULL DEFAULT '',
    mobile_map_url TEXT NOT NULL DEFAULT '',
    low_power_map_url TEXT NOT NULL DEFAULT '',
    fallback_image_url TEXT NOT NULL DEFAULT '',
    thumbnail_url TEXT NOT NULL DEFAULT '',
    desktop_particle_count INTEGER NOT NULL DEFAULT 0,
    mobile_particle_count INTEGER NOT NULL DEFAULT 0,
    low_power_particle_count INTEGER NOT NULL DEFAULT 0,
    processing_status TEXT NOT NULL DEFAULT 'pending'
        CHECK (processing_status IN ('pending', 'processing', 'ready', 'failed')),
    processing_error TEXT NOT NULL DEFAULT '',
    processing_version INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT church_prayer_visualizer_assets_prayer_unique UNIQUE (prayer_id)
);

CREATE INDEX IF NOT EXISTS idx_church_prayer_visualizer_assets_status
    ON church_prayer_visualizer_assets(processing_status);

CREATE TRIGGER church_prayer_visualizer_assets_set_updated_at
    BEFORE UPDATE ON church_prayer_visualizer_assets
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();
