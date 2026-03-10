-- CMS improvements: timestamps everywhere, alt text for gallery,
-- article_categories table, updated_at trigger for experience/expertise/gallery
-- Date: 2026-03-10

-- ── 1. Add missing timestamps to experience & expertise ───────────────────────
ALTER TABLE experience
    ADD COLUMN IF NOT EXISTS created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    ADD COLUMN IF NOT EXISTS updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW();

ALTER TABLE expertise
    ADD COLUMN IF NOT EXISTS created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    ADD COLUMN IF NOT EXISTS updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW();

-- ── 2. Add alt text fields to gallery (SEO) ───────────────────────────────────
ALTER TABLE gallery
    ADD COLUMN IF NOT EXISTS alt_en TEXT NOT NULL DEFAULT '',
    ADD COLUMN IF NOT EXISTS alt_pl TEXT NOT NULL DEFAULT '',
    ADD COLUMN IF NOT EXISTS alt_ru TEXT NOT NULL DEFAULT '',
    ADD COLUMN IF NOT EXISTS alt_uk TEXT NOT NULL DEFAULT '';

-- ── 3. Add updated_at to gallery (was missing) ────────────────────────────────
ALTER TABLE gallery
    ADD COLUMN IF NOT EXISTS updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW();

-- ── 4. article_categories table ───────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS article_categories (
    id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    slug       TEXT NOT NULL UNIQUE,
    title_en   TEXT NOT NULL,
    title_pl   TEXT NOT NULL DEFAULT '',
    title_ru   TEXT NOT NULL DEFAULT '',
    title_uk   TEXT NOT NULL DEFAULT '',
    order_index INT NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Seed default categories
INSERT INTO article_categories (slug, title_en, title_pl, title_ru, title_uk, order_index)
VALUES
    ('sushi',             'Sushi',                       'Sushi',                          'Суши',                    'Суші',                    1),
    ('fish',              'Fish & Seafood',               'Ryby i owoce morza',             'Рыба и морепродукты',     'Риба та морепродукти',    2),
    ('kitchen-tech',      'Kitchen Technology',           'Technologia kuchenna',           'Кухонные технологии',     'Кухонні технології',      3),
    ('knife-skills',      'Knife Skills',                 'Techniki noży',                  'Работа с ножом',          'Робота з ножем',          4),
    ('food-safety',       'Food Safety & HACCP',          'Bezpieczeństwo żywności HACCP',  'Безопасность питания',    'Безпека харчування',      5),
    ('product-knowledge', 'Product Knowledge',            'Wiedza o produktach',            'Знание продукта',         'Знання продукту',         6)
ON CONFLICT (slug) DO NOTHING;

-- ── 5. Link articles ↔ categories (many-to-many) ─────────────────────────────
CREATE TABLE IF NOT EXISTS article_category_links (
    article_id  UUID NOT NULL REFERENCES knowledge_articles(id) ON DELETE CASCADE,
    category_id UUID NOT NULL REFERENCES article_categories(id) ON DELETE CASCADE,
    PRIMARY KEY (article_id, category_id)
);

-- ── 6. auto-update updated_at triggers ───────────────────────────────────────
CREATE OR REPLACE FUNCTION set_updated_at()
RETURNS TRIGGER LANGUAGE plpgsql AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$;

-- about_page
DROP TRIGGER IF EXISTS trg_about_page_updated_at ON about_page;
CREATE TRIGGER trg_about_page_updated_at
    BEFORE UPDATE ON about_page
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();

-- expertise
DROP TRIGGER IF EXISTS trg_expertise_updated_at ON expertise;
CREATE TRIGGER trg_expertise_updated_at
    BEFORE UPDATE ON expertise
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();

-- experience
DROP TRIGGER IF EXISTS trg_experience_updated_at ON experience;
CREATE TRIGGER trg_experience_updated_at
    BEFORE UPDATE ON experience
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();

-- gallery
DROP TRIGGER IF EXISTS trg_gallery_updated_at ON gallery;
CREATE TRIGGER trg_gallery_updated_at
    BEFORE UPDATE ON gallery
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();

-- knowledge_articles
DROP TRIGGER IF EXISTS trg_knowledge_articles_updated_at ON knowledge_articles;
CREATE TRIGGER trg_knowledge_articles_updated_at
    BEFORE UPDATE ON knowledge_articles
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();

-- ── 7. Indexes for performance ────────────────────────────────────────────────
CREATE INDEX IF NOT EXISTS idx_experience_order    ON experience (order_index ASC);
CREATE INDEX IF NOT EXISTS idx_expertise_order     ON expertise  (order_index ASC);
CREATE INDEX IF NOT EXISTS idx_gallery_order       ON gallery    (order_index ASC);
CREATE INDEX IF NOT EXISTS idx_articles_updated_at ON knowledge_articles (updated_at DESC);
CREATE INDEX IF NOT EXISTS idx_articles_search     ON knowledge_articles
    USING gin(to_tsvector('english', coalesce(title_en,'') || ' ' || coalesce(content_en,'')));
