-- Chef CMS: about_page, expertise, experience, gallery, knowledge_articles
-- Date: 2026-03-10

-- ── 1. about_page ─────────────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS about_page (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    title_en    TEXT NOT NULL DEFAULT '',
    title_pl    TEXT NOT NULL DEFAULT '',
    title_ru    TEXT NOT NULL DEFAULT '',
    title_uk    TEXT NOT NULL DEFAULT '',
    content_en  TEXT NOT NULL DEFAULT '',
    content_pl  TEXT NOT NULL DEFAULT '',
    content_ru  TEXT NOT NULL DEFAULT '',
    content_uk  TEXT NOT NULL DEFAULT '',
    image_url   TEXT,
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Seed default row (only one row ever exists)
INSERT INTO about_page (id, title_en, title_pl, title_ru, title_uk,
                        content_en, content_pl, content_ru, content_uk)
VALUES (
    '00000000-0000-0000-0000-000000000001',
    'About Me',
    'O mnie',
    'Обо мне',
    'Про мене',
    'Professional chef with over 20 years of experience in European and Japanese cuisine.',
    'Profesjonalny kucharz z ponad 20-letnim doświadczeniem w kuchni europejskiej i japońskiej.',
    'Профессиональный шеф-повар с более чем 20-летним опытом работы в европейской и японской кухне.',
    'Професійний шеф-кухар з понад 20-річним досвідом роботи в європейській та японській кухні.'
)
ON CONFLICT (id) DO NOTHING;

-- ── 2. expertise ──────────────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS expertise (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    icon        TEXT NOT NULL DEFAULT '🍴',
    title_en    TEXT NOT NULL,
    title_pl    TEXT NOT NULL DEFAULT '',
    title_ru    TEXT NOT NULL DEFAULT '',
    title_uk    TEXT NOT NULL DEFAULT '',
    order_index INT NOT NULL DEFAULT 0
);

INSERT INTO expertise (icon, title_en, title_pl, title_ru, title_uk, order_index) VALUES
    ('🍣', 'Sushi & Japanese Cuisine', 'Sushi i kuchnia japońska', 'Суши и японская кухня', 'Суші та японська кухня', 1),
    ('🔪', 'Japanese Knives & Techniques', 'Japońskie noże i techniki', 'Японские ножи и техники', 'Японські ножі та техніки', 2),
    ('🧬', 'Food Technology', 'Technologia żywności', 'Технология питания', 'Технологія харчування', 3),
    ('📋', 'HACCP & Food Safety', 'HACCP i bezpieczeństwo żywności', 'HACCP и безопасность питания', 'HACCP та безпека харчування', 4)
ON CONFLICT DO NOTHING;

-- ── 3. experience ─────────────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS experience (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    restaurant      TEXT NOT NULL,
    country         TEXT NOT NULL DEFAULT '',
    position        TEXT NOT NULL DEFAULT '',
    start_year      INT,
    end_year        INT,  -- NULL = current
    description_en  TEXT NOT NULL DEFAULT '',
    description_pl  TEXT NOT NULL DEFAULT '',
    description_ru  TEXT NOT NULL DEFAULT '',
    description_uk  TEXT NOT NULL DEFAULT '',
    order_index     INT NOT NULL DEFAULT 0
);

INSERT INTO experience (restaurant, country, position, start_year, end_year,
                        description_en, description_pl, description_ru, description_uk, order_index)
VALUES
    ('Fish in House', 'Ukraine, Dnipro', 'Head Chef', 2010, 2018,
     'Japanese cuisine restaurant. Specializing in fresh fish and sushi.',
     'Restauracja kuchni japońskiej. Specjalizacja w świeżych rybach i sushi.',
     'Ресторан японской кухни. Специализация на свежей рыбе и суши.',
     'Ресторан японської кухні. Спеціалізація на свіжій рибі та суші.', 1),
    ('Restaurant Charlemagne', 'France', 'Sous Chef', 2005, 2010,
     'Fine dining French restaurant.',
     'Restauracja fine dining w stylu francuskim.',
     'Французский ресторан высокой кухни.',
     'Французький ресторан вишуканої кухні.', 2),
    ('Boulangerie Wawel', 'Canada', 'Culinary Consultant', 2018, NULL,
     'Polish bakery and culinary consulting.',
     'Polska piekarnia i doradztwo kulinarne.',
     'Польская пекарня и кулинарный консалтинг.',
     'Польська пекарня та кулінарний консалтинг.', 3)
ON CONFLICT DO NOTHING;

-- ── 4. gallery ────────────────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS gallery (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    image_url       TEXT NOT NULL,
    title_en        TEXT NOT NULL DEFAULT '',
    title_pl        TEXT NOT NULL DEFAULT '',
    title_ru        TEXT NOT NULL DEFAULT '',
    title_uk        TEXT NOT NULL DEFAULT '',
    description_en  TEXT NOT NULL DEFAULT '',
    description_pl  TEXT NOT NULL DEFAULT '',
    description_ru  TEXT NOT NULL DEFAULT '',
    description_uk  TEXT NOT NULL DEFAULT '',
    order_index     INT NOT NULL DEFAULT 0,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- ── 5. knowledge_articles ────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS knowledge_articles (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    slug            TEXT NOT NULL UNIQUE,
    title_en        TEXT NOT NULL,
    title_pl        TEXT NOT NULL DEFAULT '',
    title_ru        TEXT NOT NULL DEFAULT '',
    title_uk        TEXT NOT NULL DEFAULT '',
    content_en      TEXT NOT NULL DEFAULT '',
    content_pl      TEXT NOT NULL DEFAULT '',
    content_ru      TEXT NOT NULL DEFAULT '',
    content_uk      TEXT NOT NULL DEFAULT '',
    image_url       TEXT,
    seo_title       TEXT NOT NULL DEFAULT '',
    seo_description TEXT NOT NULL DEFAULT '',
    published       BOOLEAN NOT NULL DEFAULT FALSE,
    order_index     INT NOT NULL DEFAULT 0,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_knowledge_articles_slug      ON knowledge_articles (slug);
CREATE INDEX IF NOT EXISTS idx_knowledge_articles_published ON knowledge_articles (published) WHERE published = true;

-- Seed 3 articles
INSERT INTO knowledge_articles (slug, title_en, title_pl, title_ru, title_uk,
                                 content_en, seo_title, seo_description, published, order_index)
VALUES
    ('knife-techniques',
     'Japanese Knife Techniques',
     'Techniki japońskich noży',
     'Техники японских ножей',
     'Техніки японських ножів',
     'Learn the fundamental cutting techniques used in Japanese cuisine: katsuramuki, sengiri, and brunoise.',
     'Japanese Knife Techniques | Chef Guide',
     'Master Japanese knife techniques: katsuramuki, sengiri, brunoise. Professional chef guide.',
     TRUE, 1),
    ('choose-fresh-fish',
     'How to Choose Fresh Fish',
     'Jak wybrać świeżą rybę',
     'Как выбрать свежую рыбу',
     'Як вибрати свіжу рибу',
     'A professional guide to selecting the freshest fish at the market: eyes, gills, smell, and texture.',
     'How to Choose Fresh Fish | Chef Tips',
     'Professional guide to choosing fresh fish. Learn what to look for: eyes, gills, smell.',
     TRUE, 2),
    ('sushi-rice-technique',
     'Sushi Rice Technique',
     'Technika ryżu do sushi',
     'Техника приготовления риса для суши',
     'Техніка приготування рису для суші',
     'Perfect sushi rice requires the right ratio of rice to water, vinegar seasoning, and proper cooling technique.',
     'Sushi Rice Technique | Perfect Every Time',
     'Master the art of sushi rice. Ratios, vinegar seasoning, and cooling technique explained.',
     TRUE, 3)
ON CONFLICT (slug) DO NOTHING;

COMMENT ON TABLE about_page IS 'Single-row CMS: chef about page content';
COMMENT ON TABLE expertise IS 'Chef expertise cards (icon + title)';
COMMENT ON TABLE experience IS 'Chef work experience timeline';
COMMENT ON TABLE gallery IS 'Kitchen photo gallery';
COMMENT ON TABLE knowledge_articles IS 'SEO knowledge base articles';
