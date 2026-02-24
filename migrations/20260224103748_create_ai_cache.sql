-- AI Cache for LLM results
CREATE TABLE ai_cache (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    key TEXT NOT NULL UNIQUE, -- SHA256 or unique string (e.g., "classify:milk")
    value JSONB NOT NULL,    -- The result (e.g., UnifiedProductResponse)
    provider TEXT NOT NULL,  -- "groq", "openai", etc.
    model TEXT NOT NULL,     -- "llama-3.1-8b-instant"
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_ai_cache_key ON ai_cache(key);
