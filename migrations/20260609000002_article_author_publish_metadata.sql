ALTER TABLE knowledge_articles
    ADD COLUMN IF NOT EXISTS author_name TEXT NOT NULL DEFAULT 'Szef Kuchni',
    ADD COLUMN IF NOT EXISTS author_avatar_url TEXT,
    ADD COLUMN IF NOT EXISTS published_at TIMESTAMPTZ;

UPDATE knowledge_articles
SET published_at = COALESCE(published_at, created_at)
WHERE published = true;

CREATE INDEX IF NOT EXISTS idx_knowledge_articles_published_at
    ON knowledge_articles (published_at DESC)
    WHERE published = true;
