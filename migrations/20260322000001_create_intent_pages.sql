-- Intent Pages — AI-generated SEO content for programmatic pages
--
-- Each row = one SEO page (e.g. "Is salmon healthy?", "Salmon vs Tuna")
-- Pipeline: generate → draft → review → publish → frontend renders

CREATE TABLE IF NOT EXISTS intent_pages (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    -- Intent classification
    intent_type     TEXT NOT NULL CHECK (intent_type IN ('question', 'comparison', 'goal', 'combo')),
    entity_a        TEXT NOT NULL,                -- main ingredient slug (e.g. "salmon")
    entity_b        TEXT,                         -- secondary ingredient (for comparison/combo)
    locale          TEXT NOT NULL CHECK (locale IN ('en', 'pl', 'ru', 'uk')),

    -- AI-generated content
    title           TEXT NOT NULL,                -- SEO title (max 60 chars)
    description     TEXT NOT NULL,                -- meta description (max 140 chars)
    answer          TEXT NOT NULL,                -- main answer (2-3 sentences)
    faq             JSONB NOT NULL DEFAULT '[]',  -- [{q, a}, {q, a}]

    -- URL slug (auto-generated, unique per locale)
    slug            TEXT NOT NULL,                -- e.g. "is-salmon-healthy"

    -- Publication status
    status          TEXT NOT NULL DEFAULT 'draft' CHECK (status IN ('draft', 'published', 'archived')),
    published_at    TIMESTAMPTZ,

    -- Metadata
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()

    -- Prevent duplicate content: one page per intent+entities+locale
    -- NOTE: we use a unique index (below) instead of table-level UNIQUE
    -- because entity_b can be NULL and PostgreSQL UNIQUE treats NULLs as distinct
);

-- Unique index that handles NULL entity_b correctly
CREATE UNIQUE INDEX uq_intent_pages_content
    ON intent_pages (intent_type, entity_a, COALESCE(entity_b, ''), locale);

-- Index for fast public queries
CREATE INDEX idx_intent_pages_published ON intent_pages (status, locale) WHERE status = 'published';
CREATE INDEX idx_intent_pages_entity ON intent_pages (entity_a, locale);
CREATE INDEX idx_intent_pages_slug ON intent_pages (slug, locale);

-- Auto-update updated_at
CREATE OR REPLACE FUNCTION update_intent_pages_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_intent_pages_updated_at
    BEFORE UPDATE ON intent_pages
    FOR EACH ROW
    EXECUTE FUNCTION update_intent_pages_updated_at();
