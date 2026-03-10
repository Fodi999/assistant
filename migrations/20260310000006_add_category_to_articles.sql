-- Add category field to knowledge_articles
ALTER TABLE knowledge_articles
    ADD COLUMN IF NOT EXISTS category TEXT NOT NULL DEFAULT '';

-- Update existing seed articles with categories
UPDATE knowledge_articles SET category = 'techniques'  WHERE slug = 'knife-techniques';
UPDATE knowledge_articles SET category = 'ingredients' WHERE slug = 'choose-fresh-fish';
UPDATE knowledge_articles SET category = 'techniques'  WHERE slug = 'sushi-rice-technique';

CREATE INDEX IF NOT EXISTS idx_knowledge_articles_category ON knowledge_articles (category);
