-- Gallery schema v2: gallery_categories table, category_id FK, slug, status
-- Date: 2026-03-11

-- ── 1. gallery_categories ────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS gallery_categories (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    slug        TEXT NOT NULL UNIQUE,
    title_en    TEXT NOT NULL DEFAULT '',
    title_pl    TEXT NOT NULL DEFAULT '',
    title_ru    TEXT NOT NULL DEFAULT '',
    title_uk    TEXT NOT NULL DEFAULT '',
    order_index INT NOT NULL DEFAULT 0,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

INSERT INTO gallery_categories (slug, title_en, title_pl, title_ru, title_uk, order_index) VALUES
    ('kitchen', 'Kitchen',    'Kuchnia',       'Кухня',         'Кухня',           1),
    ('market',  'Market',     'Rynek',         'Рынок',         'Ринок',           2),
    ('fish',    'Fish',       'Ryba',          'Рыба',          'Риба',            3),
    ('sushi',   'Sushi',      'Sushi',         'Суши',          'Суші',            4),
    ('events',  'Events',     'Wydarzenie',    'Мероприятия',   'Заходи',          5),
    ('other',   'Other',      'Inne',          'Другое',        'Інше',            6)
ON CONFLICT (slug) DO NOTHING;

-- ── 2. Add new columns to gallery ────────────────────────────────────────────
ALTER TABLE gallery
    ADD COLUMN IF NOT EXISTS category_id UUID REFERENCES gallery_categories(id) ON DELETE SET NULL,
    ADD COLUMN IF NOT EXISTS slug        TEXT UNIQUE,
    ADD COLUMN IF NOT EXISTS status      TEXT NOT NULL DEFAULT 'published';

-- ── 3. Migrate old category text → category_id FK ────────────────────────────
UPDATE gallery g
SET category_id = gc.id
FROM gallery_categories gc
WHERE g.category = gc.slug
  AND g.category IS NOT NULL
  AND g.category <> '';

-- ── 4. Auto-generate slugs for existing rows that don't have one ──────────────
UPDATE gallery
SET slug = 'gallery-' || SUBSTRING(id::TEXT, 1, 8)
WHERE slug IS NULL;

-- ── 5. Drop old category text column ─────────────────────────────────────────
ALTER TABLE gallery DROP COLUMN IF EXISTS category;

-- ── 6. Indexes ────────────────────────────────────────────────────────────────
CREATE INDEX IF NOT EXISTS idx_gallery_category_id ON gallery (category_id);
CREATE INDEX IF NOT EXISTS idx_gallery_slug        ON gallery (slug);
CREATE INDEX IF NOT EXISTS idx_gallery_status      ON gallery (status);

-- ── 7. updated_at trigger for gallery_categories ─────────────────────────────
DROP TRIGGER IF EXISTS trg_gallery_categories_updated_at ON gallery_categories;
CREATE TRIGGER trg_gallery_categories_updated_at
    BEFORE UPDATE ON gallery_categories
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();
