-- Migration: Add catalog_ingredient_translations table (i18n best practice)
-- This replaces the old name_pl/name_en/name_uk/name_ru columns approach

-- Step 1: Create translations table
CREATE TABLE catalog_ingredient_translations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    ingredient_id UUID NOT NULL REFERENCES catalog_ingredients(id) ON DELETE CASCADE,
    language TEXT NOT NULL CHECK (language IN ('en', 'pl', 'uk', 'ru')),
    name TEXT NOT NULL,
    
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    
    -- Ensure one translation per language per ingredient
    UNIQUE (ingredient_id, language)
);

-- Step 2: Create indexes for fast lookups
CREATE INDEX idx_ingredient_translations_ingredient ON catalog_ingredient_translations(ingredient_id);
CREATE INDEX idx_ingredient_translations_language ON catalog_ingredient_translations(language);
CREATE INDEX idx_ingredient_translations_name_search ON catalog_ingredient_translations USING gin (name gin_trgm_ops);

-- Step 3: Migrate existing data from old columns to translations table
INSERT INTO catalog_ingredient_translations (ingredient_id, language, name)
SELECT id, 'en', name_en FROM catalog_ingredients WHERE name_en IS NOT NULL
UNION ALL
SELECT id, 'pl', name_pl FROM catalog_ingredients WHERE name_pl IS NOT NULL
UNION ALL
SELECT id, 'uk', name_uk FROM catalog_ingredients WHERE name_uk IS NOT NULL
UNION ALL
SELECT id, 'ru', name_ru FROM catalog_ingredients WHERE name_ru IS NOT NULL;

-- Step 4: Similarly, create category translations table
CREATE TABLE catalog_category_translations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    category_id UUID NOT NULL REFERENCES catalog_categories(id) ON DELETE CASCADE,
    language TEXT NOT NULL CHECK (language IN ('en', 'pl', 'uk', 'ru')),
    name TEXT NOT NULL,
    
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    
    UNIQUE (category_id, language)
);

CREATE INDEX idx_category_translations_category ON catalog_category_translations(category_id);
CREATE INDEX idx_category_translations_language ON catalog_category_translations(language);

-- Step 5: Migrate category data
INSERT INTO catalog_category_translations (category_id, language, name)
SELECT id, 'en', name_en FROM catalog_categories WHERE name_en IS NOT NULL
UNION ALL
SELECT id, 'pl', name_pl FROM catalog_categories WHERE name_pl IS NOT NULL
UNION ALL
SELECT id, 'uk', name_uk FROM catalog_categories WHERE name_uk IS NOT NULL
UNION ALL
SELECT id, 'ru', name_ru FROM catalog_categories WHERE name_ru IS NOT NULL;

-- Step 6: Drop old columns (CAREFUL: this is destructive!)
-- Uncomment after verifying translations work:
-- ALTER TABLE catalog_ingredients DROP COLUMN name_pl;
-- ALTER TABLE catalog_ingredients DROP COLUMN name_en;
-- ALTER TABLE catalog_ingredients DROP COLUMN name_uk;
-- ALTER TABLE catalog_ingredients DROP COLUMN name_ru;
-- 
-- ALTER TABLE catalog_categories DROP COLUMN name_pl;
-- ALTER TABLE catalog_categories DROP COLUMN name_en;
-- ALTER TABLE catalog_categories DROP COLUMN name_uk;
-- ALTER TABLE catalog_categories DROP COLUMN name_ru;

-- Step 7: Create trigger for auto-updating updated_at
CREATE TRIGGER trg_ingredient_translations_updated
BEFORE UPDATE ON catalog_ingredient_translations
FOR EACH ROW
EXECUTE FUNCTION set_updated_at();

-- Verification queries (run these to check migration success):
-- SELECT language, COUNT(*) FROM catalog_ingredient_translations GROUP BY language;
-- SELECT cit.name FROM catalog_ingredient_translations cit WHERE cit.language = 'ru' AND cit.name ILIKE '%мол%';
