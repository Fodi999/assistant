ALTER TABLE knowledge_articles
    ADD COLUMN IF NOT EXISTS seo_title_en TEXT NOT NULL DEFAULT '',
    ADD COLUMN IF NOT EXISTS seo_title_ru TEXT NOT NULL DEFAULT '',
    ADD COLUMN IF NOT EXISTS seo_title_pl TEXT NOT NULL DEFAULT '',
    ADD COLUMN IF NOT EXISTS seo_title_uk TEXT NOT NULL DEFAULT '',
    ADD COLUMN IF NOT EXISTS seo_description_en TEXT NOT NULL DEFAULT '',
    ADD COLUMN IF NOT EXISTS seo_description_ru TEXT NOT NULL DEFAULT '',
    ADD COLUMN IF NOT EXISTS seo_description_pl TEXT NOT NULL DEFAULT '',
    ADD COLUMN IF NOT EXISTS seo_description_uk TEXT NOT NULL DEFAULT '';

UPDATE knowledge_articles
SET seo_title_en = COALESCE(NULLIF(seo_title_en, ''), NULLIF(seo_title, ''), title_en),
    seo_title_ru = COALESCE(NULLIF(seo_title_ru, ''), NULLIF(seo_title, ''), title_ru, title_en),
    seo_title_pl = COALESCE(NULLIF(seo_title_pl, ''), NULLIF(seo_title, ''), title_pl, title_en),
    seo_title_uk = COALESCE(NULLIF(seo_title_uk, ''), NULLIF(seo_title, ''), title_uk, title_en),
    seo_description_en = COALESCE(NULLIF(seo_description_en, ''), seo_description, ''),
    seo_description_ru = COALESCE(NULLIF(seo_description_ru, ''), seo_description, ''),
    seo_description_pl = COALESCE(NULLIF(seo_description_pl, ''), seo_description, ''),
    seo_description_uk = COALESCE(NULLIF(seo_description_uk, ''), seo_description, '');
