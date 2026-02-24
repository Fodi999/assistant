-- AI Usage Stats
CREATE TABLE ai_usage_stats (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    endpoint TEXT NOT NULL,
    prompt_tokens INTEGER NOT NULL DEFAULT 0,
    completion_tokens INTEGER NOT NULL DEFAULT 0,
    total_tokens INTEGER NOT NULL DEFAULT 0,
    duration_ms INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_ai_usage_stats_endpoint ON ai_usage_stats(endpoint);

-- Add expires_at to ai_cache
ALTER TABLE ai_cache ADD COLUMN expires_at TIMESTAMPTZ;
UPDATE ai_cache SET expires_at = NOW() + INTERVAL '90 days' WHERE expires_at IS NULL;
ALTER TABLE ai_cache ALTER COLUMN expires_at SET NOT NULL;
