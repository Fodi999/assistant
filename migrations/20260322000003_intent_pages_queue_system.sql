-- Add queue system to intent pages
-- New statuses: draft → queued → published → archived

-- 1. Drop old CHECK constraint and add new one with 'queued' status
ALTER TABLE intent_pages DROP CONSTRAINT IF EXISTS intent_pages_status_check;
ALTER TABLE intent_pages ADD CONSTRAINT intent_pages_status_check
    CHECK (status IN ('draft', 'queued', 'published', 'archived'));

-- 2. Add queued_at column for queue ordering
ALTER TABLE intent_pages ADD COLUMN IF NOT EXISTS queued_at TIMESTAMPTZ;

-- 3. Create settings table for publish limits
CREATE TABLE IF NOT EXISTS seo_settings (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 4. Default: 24 pages per day
INSERT INTO seo_settings (key, value)
VALUES ('publish_limit_per_day', '24')
ON CONFLICT (key) DO NOTHING;

-- 5. Index for scheduler: pick queued pages ordered by queue time
CREATE INDEX IF NOT EXISTS idx_intent_pages_queued
    ON intent_pages (queued_at ASC)
    WHERE status = 'queued';

-- 6. Index for archived pages
CREATE INDEX IF NOT EXISTS idx_intent_pages_archived
    ON intent_pages (updated_at DESC)
    WHERE status = 'archived';
