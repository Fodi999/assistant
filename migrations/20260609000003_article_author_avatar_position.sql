ALTER TABLE knowledge_articles
    ADD COLUMN IF NOT EXISTS author_avatar_position TEXT NOT NULL DEFAULT 'center';
