-- Add priority field for publish queue ordering
-- 0 = low, 1 = normal (default), 2 = high

ALTER TABLE intent_pages ADD COLUMN IF NOT EXISTS priority INT NOT NULL DEFAULT 1;

-- Update scheduler index to include priority
DROP INDEX IF EXISTS idx_intent_pages_queued;
CREATE INDEX idx_intent_pages_queued
    ON intent_pages (priority DESC, queued_at ASC)
    WHERE status = 'queued';
